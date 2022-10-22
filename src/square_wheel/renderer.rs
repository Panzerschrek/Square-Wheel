use super::{
	abstract_color::*, config, debug_stats_printer::*, depth_renderer::*, draw_ordering, dynamic_objects_index::*,
	equations::*, fast_math::*, frame_info::*, frame_number::*, inline_models_index::*, light::*,
	map_materials_processor::*, map_visibility_calculator::*, performance_counter::*, rasterizer::*, rect_splitting,
	renderer_config::*, resources_manager::*, surfaces::*, textures::*, triangle_model::*,
	triangle_models_rendering::*,
};
use crate::common::{
	bbox::*, bsp_map_compact, clipping::*, clipping_polygon::*, fixed_math::*, light_cube::*, lightmap, material,
	math_types::*, matrix::*, plane::*, shared_mut_slice::*, system_window,
};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub struct Renderer
{
	app_config: config::ConfigSharedPtr,
	config: RendererConfig,

	current_frame: FrameNumber,
	map: Arc<bsp_map_compact::BSPMap>,
	visibility_calculator: MapVisibilityCalculator,
	shadows_maps_renderer: DepthRenderer,
	polygons_data: Vec<DrawPolygonData>,
	vertices_transformed: Vec<Vec3f>,
	// Store surfaces pixels as raw array.
	// Use specific color while preparing surfaces or performing rasterization.
	// TODO - make sure alignment is correct.
	surfaces_pixels: Vec<u8>,
	num_visible_surfaces_pixels: usize,
	current_frame_visible_polygons: Vec<u32>,
	mip_bias: f32,
	inline_models_index: InlineModelsIndex,
	submodels_info: Vec<VisibleSubmodelInfo>,
	// Material index and clipping polygon of current frame sky.
	current_sky: Option<(u32, ClippingPolygon)>,
	materials_processor: MapMaterialsProcessor,
	performance_counters: Arc<Mutex<RendererPerformanceCounters>>,
	dynamic_lights_info: Vec<DynamicLightInfo>,
	shadow_maps_data: Vec<ShadowMapElement>,
	dynamic_models_index: DynamicObjectsIndex,
	decals_index: DynamicObjectsIndex,
	decals_info: Vec<DecalInfo>,
	sprites_index: DynamicObjectsIndex,
	sprites_info: Vec<SpriteInfo>,
	dynamic_lights_index: DynamicObjectsIndex,
	// TODO - maybe extract dynamic models-related stuff into separate class?
	// Store transformed models vertices and triangles in separate buffer.
	// This is needed to avoid transforming/sorting model's vertices/triangles in each BSP leaf where this model is located.
	visible_dynamic_meshes_list: Vec<VisibleDynamicMeshInfo>,
	dynamic_model_to_dynamic_meshes_index: Vec<DynamicModelInfo>,
	dynamic_meshes_vertices: Vec<ModelVertex3d>,
	dynamic_meshes_triangles: Vec<Triangle>,
}

struct RendererPerformanceCounters
{
	materials_update: PerformanceCounter,
	visible_leafs_search: PerformanceCounter,
	triangle_models_preparation: PerformanceCounter,
	surfaces_preparation: PerformanceCounter,
	shadow_maps_building: PerformanceCounter,
	background_fill: PerformanceCounter,
	rasterization: PerformanceCounter,
}

impl RendererPerformanceCounters
{
	fn new() -> Self
	{
		let window_size = 100;
		Self {
			materials_update: PerformanceCounter::new(window_size),
			visible_leafs_search: PerformanceCounter::new(window_size),
			triangle_models_preparation: PerformanceCounter::new(window_size),
			surfaces_preparation: PerformanceCounter::new(window_size),
			shadow_maps_building: PerformanceCounter::new(window_size),
			background_fill: PerformanceCounter::new(window_size),
			rasterization: PerformanceCounter::new(window_size),
		}
	}
}

fn run_with_measure<F: FnOnce()>(f: F, performanc_counter: &mut PerformanceCounter)
{
	type Clock = std::time::Instant;
	let start_time = Clock::now();

	f();

	let end_time = Clock::now();
	performanc_counter.add_value((end_time - start_time).as_secs_f32());
}

// Mutable data associated with map polygon.
#[derive(Copy, Clone)]
struct DrawPolygonData
{
	// Leaf index where this polygon is located or submodel index.
	parent: DrawPolygonParent,
	// Precalculaed basis vecs for mip 0
	basis_vecs: PolygonBasisVecs,
	// Frame last time this polygon was visible.
	visible_frame: FrameNumber,
	// Projected equations for current frame.
	depth_equation: DepthEquation,
	tex_coord_equation: TexCoordEquation,
	surface_pixels_offset: usize,
	surface_size: [u32; 2],
	mip: u32,
	surface_tc_min: [i32; 2],
}

#[derive(Copy, Clone)]
enum DrawPolygonParent
{
	Leaf(u32),
	Submodel(u32),
}

// Calculate matrices once for frame and use them during polygons preparation, sorting and polygons ordering.
#[derive(Copy, Clone)]
struct VisibleSubmodelMatrices
{
	// Planes matrix for transformation of submodel planes into current position of submodel within the world.
	world_planes_matrix: Mat4f,
	camera_matrices: CameraMatrices,
}

#[derive(Default, Clone)]
struct VisibleSubmodelInfo
{
	matrices: Option<VisibleSubmodelMatrices>,
	// Dynamic lights that affects this submodel.
	dynamic_lights: Vec<DynamicObjectId>,
}

struct VisibleDynamicMeshInfo
{
	entity_index: u32,
	mesh_index: u32,
	vertices_offset: usize,
	triangles_offset: usize,
	num_visible_triangles: usize,
	bbox_vertices_transformed: [Vec3f; 8],
	clipping_polygon: ClippingPolygon,
	model_matrix: Mat4f,
	camera_matrices: CameraMatrices,
	mip: u32,
}

#[derive(Default, Copy, Clone)]
struct DynamicModelInfo
{
	first_visible_mesh: u32,
	num_visible_meshes: u32,
}

#[derive(Default, Copy, Clone)]
struct DynamicLightInfo
{
	// Light is not visible if all leafs where it is located are not visible.
	visible: bool,
	shadow_map_data_offset: usize,
	// Cubemap side size or projector shadowmap size.
	shadow_map_size: u32,
}

#[derive(Copy, Clone)]
struct DecalInfo
{
	camera_planes_matrix: Mat4f,
	dynamic_light: bsp_map_compact::LightGridElement,
}

#[derive(Copy, Clone)]
struct SpriteInfo
{
	vertices_projected: [Vec3f; 4],
	light: [f32; 3],
}

impl Renderer
{
	pub fn new(
		resources_manager: ResourcesManagerSharedPtr,
		app_config: config::ConfigSharedPtr,
		map: Arc<bsp_map_compact::BSPMap>,
	) -> Self
	{
		let config_parsed = RendererConfig::from_app_config(&app_config);
		config_parsed.update_app_config(&app_config); // Update JSON with struct fields.

		let materials_processor = MapMaterialsProcessor::new(resources_manager.clone(), &*map);

		let mut polygons_data: Vec<DrawPolygonData> = map
			.polygons
			.iter()
			.map(|p| DrawPolygonData {
				parent: DrawPolygonParent::Leaf(0), // Set later
				// Pre-calculate basis vecs and use them each frame.
				basis_vecs: PolygonBasisVecs::form_plane_and_tex_coord_equation(&p.plane, &p.tex_coord_equation),
				visible_frame: Default::default(),
				depth_equation: Default::default(),
				tex_coord_equation: Default::default(),
				surface_pixels_offset: 0,
				surface_size: [0, 0],
				mip: 0,
				surface_tc_min: [0, 0],
			})
			.collect();

		// Setup relations between leafs and polygons.
		for (leaf_index, leaf) in map.leafs.iter().enumerate()
		{
			for polygon_index in leaf.first_polygon .. leaf.first_polygon + leaf.num_polygons
			{
				polygons_data[polygon_index as usize].parent = DrawPolygonParent::Leaf(leaf_index as u32);
			}
		}
		// Setup relations between submodels and polygons.
		for (submodel_index, submodel) in map.submodels.iter().enumerate()
		{
			for polygon_index in submodel.first_polygon .. submodel.first_polygon + submodel.num_polygons
			{
				polygons_data[polygon_index as usize].parent = DrawPolygonParent::Submodel(submodel_index as u32);
			}
		}

		Renderer {
			app_config,
			config: config_parsed,
			current_frame: FrameNumber::default(),
			polygons_data,
			vertices_transformed: vec![Vec3f::new(0.0, 0.0, 0.0); map.vertices.len()],
			surfaces_pixels: Vec::new(),
			num_visible_surfaces_pixels: 0,
			current_frame_visible_polygons: Vec::with_capacity(map.polygons.len()),
			mip_bias: 0.0,
			inline_models_index: InlineModelsIndex::new(map.clone()),
			submodels_info: vec![VisibleSubmodelInfo::default(); map.submodels.len()],
			current_sky: None,
			visibility_calculator: MapVisibilityCalculator::new(map.clone()),
			shadows_maps_renderer: DepthRenderer::new(resources_manager, map.clone()),
			map: map.clone(),
			materials_processor,
			performance_counters: Arc::new(Mutex::new(RendererPerformanceCounters::new())),
			dynamic_lights_info: Vec::new(),
			shadow_maps_data: Vec::new(),
			dynamic_models_index: DynamicObjectsIndex::new(map.clone()),
			decals_index: DynamicObjectsIndex::new(map.clone()),
			decals_info: Vec::new(),
			sprites_index: DynamicObjectsIndex::new(map.clone()),
			sprites_info: Vec::new(),
			dynamic_lights_index: DynamicObjectsIndex::new(map),
			visible_dynamic_meshes_list: Vec::new(),
			dynamic_model_to_dynamic_meshes_index: Vec::new(),
			dynamic_meshes_vertices: Vec::new(),
			dynamic_meshes_triangles: Vec::new(),
		}
	}

	pub fn prepare_frame<ColorT: AbstractColor>(
		&mut self,
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
	)
	{
		let performance_counters_ptr = self.performance_counters.clone();
		let mut performance_counters = performance_counters_ptr.lock().unwrap();

		self.synchronize_config();
		self.update_mip_bias();

		self.current_frame.next();

		run_with_measure(
			|| self.materials_processor.update(frame_info.game_time_s),
			&mut performance_counters.materials_update,
		);

		run_with_measure(
			|| {
				// TODO - before preparing frame try to shift camera a little bit away from all planes of BSP nodes before current leaf.
				// This is needed to fix possible z_near clipping of current leaf portals.

				let frame_bounds =
					ClippingPolygon::from_box(0.0, 0.0, surface_info.width as f32, surface_info.height as f32);
				self.visibility_calculator
					.update_visibility(&frame_info.camera_matrices, &frame_bounds);
			},
			&mut performance_counters.visible_leafs_search,
		);

		self.prepare_dynamic_lights(frame_info);
		run_with_measure(
			|| {
				self.build_shadow_maps(&frame_info.lights);
			},
			&mut performance_counters.shadow_maps_building,
		);

		self.prepare_submodels(frame_info);

		run_with_measure(
			|| {
				self.prepare_dynamic_models(&frame_info.camera_matrices, &frame_info.model_entities);
				self.build_dynamic_models_buffers(&frame_info.lights, &frame_info.model_entities);
			},
			&mut performance_counters.triangle_models_preparation,
		);

		self.prepare_decals(frame_info);

		self.prepare_sprites(frame_info);

		run_with_measure(
			|| {
				self.prepare_polygons_surfaces(&frame_info.camera_matrices);
				self.allocate_surfaces_pixels::<ColorT>();

				self.build_polygons_surfaces::<ColorT>(&frame_info.camera_matrices, &frame_info.lights);
			},
			&mut performance_counters.surfaces_preparation,
		);
	}

	pub fn draw_frame<ColorT: AbstractColor>(
		&mut self,
		pixels: &mut [ColorT],
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
		debug_stats_printer: &mut DebugStatsPrinter,
	)
	{
		let performance_counters_ptr = self.performance_counters.clone();
		let mut performance_counters = performance_counters_ptr.lock().unwrap();

		run_with_measure(
			|| {
				// Clear background (if needed) only before performing rasterization.
				// Clear bacgrkound only if camera is located outside of volume of current leaf, defined as space at front of all leaf polygons.
				// If camera is inside volume space, we do not need to fill background because (normally) no gaps between map geometry should be visible.
				if self.config.clear_background && !self.visibility_calculator.is_current_camera_inside_leaf_volume()
				{
					draw_background(pixels, ColorVec::from_color_f32x3(&[32.0, 16.0, 8.0]).into());
				}
			},
			&mut performance_counters.background_fill,
		);

		run_with_measure(
			|| self.perform_rasterization(pixels, surface_info, frame_info),
			&mut performance_counters.rasterization,
		);

		if debug_stats_printer.show_debug_stats()
		{
			self.print_debug_stats(frame_info, debug_stats_printer, &performance_counters);
		}

		if self.config.debug_draw_depth
		{
			for light_info in &self.dynamic_lights_info
			{
				let data_size = (light_info.shadow_map_size * light_info.shadow_map_size) as usize;
				let shadow_map_data = &self.shadow_maps_data
					[light_info.shadow_map_data_offset .. light_info.shadow_map_data_offset + data_size];

				for y in 0 .. light_info.shadow_map_size
				{
					for x in 0 .. light_info.shadow_map_size
					{
						let depth = shadow_map_data[(x + y * light_info.shadow_map_size) as usize];
						let z = (0.5 / depth).max(0.0).min(255.0);
						pixels[(x as usize) + (y as usize) * surface_info.pitch] =
							ColorVec::from_color_f32x3(&[z, z, z]).into();
					}
				}
			}
		}
	}

	fn print_debug_stats(
		&mut self,
		frame_info: &FrameInfo,
		debug_stats_printer: &mut DebugStatsPrinter,
		performance_counters: &RendererPerformanceCounters,
	)
	{
		let mut num_visible_leafs = 0;
		let mut num_visible_submodels_parts = 0;
		let mut num_visible_meshes_parts = 0;
		for leaf_index in 0 .. self.map.leafs.len() as u32
		{
			if self
				.visibility_calculator
				.get_current_frame_leaf_bounds(leaf_index)
				.is_some()
			{
				num_visible_leafs += 1;
				num_visible_submodels_parts += self.inline_models_index.get_leaf_models(leaf_index as u32).len();
				num_visible_meshes_parts += self.dynamic_models_index.get_leaf_objects(leaf_index as u32).len();
			}
		}

		let mut triangles = 0;
		let mut triangle_vertices = 0;
		for visible_dynamic_mesh in &self.visible_dynamic_meshes_list
		{
			triangles += visible_dynamic_mesh.num_visible_triangles;
			triangle_vertices += match &frame_info.model_entities[visible_dynamic_mesh.entity_index as usize]
				.model
				.meshes[visible_dynamic_mesh.mesh_index as usize]
				.vertex_data
			{
				VertexData::NonAnimated(v) => v.len(),
				VertexData::VertexAnimated { constant, .. } => constant.len(),
				VertexData::SkeletonAnimated(v) => v.len(),
			};
		}

		let mut decals = 0;
		let mut decals_leafs_parts = 0;
		for i in 0 .. frame_info.decals.len()
		{
			let mut visible = false;
			for leaf_index in self.decals_index.get_object_leafs(i)
			{
				if self
					.visibility_calculator
					.get_current_frame_leaf_bounds(*leaf_index)
					.is_some()
				{
					decals_leafs_parts += 1;
					visible = true;
				}
			}
			if visible
			{
				decals += 1;
			}
		}

		let mut visible_lights = 0;
		let mut visible_lights_with_shadow = 0;
		for (light, light_info) in frame_info.lights.iter().zip(self.dynamic_lights_info.iter_mut())
		{
			if light_info.visible
			{
				visible_lights += 1;
				if let DynamicLightShadowType::None = light.shadow_type
				{
				}
				else
				{
					visible_lights_with_shadow += 1;
				}
			}
		}

		debug_stats_printer.add_line(format!(
			"materials update: {:04.2}ms",
			performance_counters.materials_update.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"visible leafs search: {:04.2}ms",
			performance_counters.visible_leafs_search.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"triangle models preparation: {:04.2}ms",
			performance_counters.triangle_models_preparation.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"surfaces preparation: {:04.2}ms",
			performance_counters.surfaces_preparation.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"shadow maps building: {:04.2}ms",
			performance_counters.shadow_maps_building.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"background fill: {:04.2}ms",
			performance_counters.background_fill.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"rasterization: {:04.2}ms",
			performance_counters.rasterization.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!("leafs: {}/{}", num_visible_leafs, self.map.leafs.len()));
		debug_stats_printer.add_line(format!("submodels parts: {}", num_visible_submodels_parts));
		debug_stats_printer.add_line(format!("polygons: {}", self.current_frame_visible_polygons.len()));
		debug_stats_printer.add_line(format!(
			"dynamic meshes : {}, parts: {}, triangles: {}, vertices: {}",
			self.visible_dynamic_meshes_list.len(),
			num_visible_meshes_parts,
			triangles,
			triangle_vertices
		));
		debug_stats_printer.add_line(format!("decals: {}, (parsts in leafs: {})", decals, decals_leafs_parts));
		debug_stats_printer.add_line(format!(
			"dynamic lights: {}, (with shadow: {})",
			visible_lights, visible_lights_with_shadow
		));
		debug_stats_printer.add_line(format!(
			"surfaces pixels: {}k",
			(self.num_visible_surfaces_pixels + 1023) / 1024
		));
		debug_stats_printer.add_line(format!("mip bias: {}", self.mip_bias));
	}

	fn prepare_dynamic_lights(&mut self, frame_info: &FrameInfo)
	{
		let lights = &frame_info.lights;

		self.dynamic_lights_index.position_dynamic_lights(lights);

		self.dynamic_lights_info
			.resize(lights.len(), DynamicLightInfo::default());

		// Allocate storage for shadowmaps.
		let mut shadow_map_data_offset = 0;
		for (index, (light, light_info)) in lights.iter().zip(self.dynamic_lights_info.iter_mut()).enumerate()
		{
			light_info.visible = false;
			for leaf_index in self.dynamic_lights_index.get_object_leafs(index)
			{
				if self
					.visibility_calculator
					.get_current_frame_leaf_bounds(*leaf_index)
					.is_some()
				{
					light_info.visible = true;
					break;
				}
			}

			if !light_info.visible
			{
				continue;
			}

			let mut shadow_map_data_size = 0;
			match light.shadow_type
			{
				DynamicLightShadowType::None =>
				{
					light_info.shadow_map_size = 0;
				},
				DynamicLightShadowType::Cubemap =>
				{
					let dist_from_camera = (frame_info.camera_matrices.position - light.position).magnitude();
					let dist_to_closest_point = dist_from_camera - light.radius;
					let min_shadow_map_size = 64;
					let max_shadow_map_size = 256;
					if dist_to_closest_point <= 0.0
					{
						light_info.shadow_map_size = max_shadow_map_size;
					}
					else
					{
						// TODO - tune this formula.
						// TODO - maybe make it dependent on screen resolution and FOV?
						let target_size = 128.0 * light.radius / dist_to_closest_point;
						light_info.shadow_map_size = (1 <<
							(target_size.log2() + self.config.shadows_quality).max(0.0) as u32)
							.min(max_shadow_map_size)
							.max(min_shadow_map_size);
					}

					shadow_map_data_size = light_info.shadow_map_size * light_info.shadow_map_size * 6;
				},
				DynamicLightShadowType::Projector { rotation, fov } =>
				{
					let min_shadow_map_size = 32;
					let max_shadow_map_size = 1024;

					let half_fov_tan_scaled_by_radius = light.radius * (fov * 0.5).tan();

					let light_matrix = get_object_matrix(light.position, rotation);

					let mut closest_square_dist = 1.0e24 as f32;
					for v in [
						Vec3f::new(light.radius, 0.0, 0.0),
						Vec3f::new(
							light.radius,
							half_fov_tan_scaled_by_radius,
							half_fov_tan_scaled_by_radius,
						),
						Vec3f::new(
							light.radius,
							half_fov_tan_scaled_by_radius,
							-half_fov_tan_scaled_by_radius,
						),
						Vec3f::new(
							light.radius,
							-half_fov_tan_scaled_by_radius,
							half_fov_tan_scaled_by_radius,
						),
						Vec3f::new(
							light.radius,
							-half_fov_tan_scaled_by_radius,
							-half_fov_tan_scaled_by_radius,
						),
					]
					{
						let v_transformed = (light_matrix * v.extend(1.0)).truncate();
						closest_square_dist =
							closest_square_dist.min((frame_info.camera_matrices.position - v_transformed).magnitude2());
					}

					let closest_dist = closest_square_dist.sqrt();

					let target_size = 1024.0 * half_fov_tan_scaled_by_radius / closest_dist;
					light_info.shadow_map_size = (1 <<
						((target_size.log2() + self.config.shadows_quality).max(0.0) as u32))
						.min(max_shadow_map_size)
						.max(min_shadow_map_size);

					shadow_map_data_size = light_info.shadow_map_size * light_info.shadow_map_size;
				},
			}

			light_info.shadow_map_data_offset = shadow_map_data_offset;
			shadow_map_data_offset += shadow_map_data_size as usize;
		}

		// Avoid resizing down to avoid refill.
		if self.shadow_maps_data.len() < shadow_map_data_offset
		{
			self.shadow_maps_data.resize(shadow_map_data_offset, 0.0);
		}
	}

	// Call this only after dynamic lights preparation.
	fn build_shadow_maps(&mut self, lights: &[DynamicLight])
	{
		// TODO - use multithreading.
		for (light, light_info) in lights.iter().zip(self.dynamic_lights_info.iter_mut())
		{
			if !light_info.visible
			{
				continue;
			}

			match &light.shadow_type
			{
				DynamicLightShadowType::None =>
				{},
				DynamicLightShadowType::Cubemap =>
				{
					let side_data_size = light_info.shadow_map_size * light_info.shadow_map_size;
					let depth_data = &mut self.shadow_maps_data[light_info.shadow_map_data_offset ..
						light_info.shadow_map_data_offset + (side_data_size * 6) as usize];

					for side in 0 .. 6
					{
						let depth_matrices = calculate_cube_shadow_map_side_matrices(
							light.position,
							light_info.shadow_map_size as f32,
							int_to_cubemap_side(side).unwrap(),
						);

						let side_depth_data =
							&mut depth_data[(side * side_data_size) as usize .. ((side + 1) * side_data_size) as usize];

						self.shadows_maps_renderer.draw_map(
							side_depth_data,
							light_info.shadow_map_size,
							light_info.shadow_map_size,
							&depth_matrices,
							&self.inline_models_index,
						);
					}
				},
				DynamicLightShadowType::Projector { rotation, fov } =>
				{
					let depth_matrices = calculate_projector_shadow_map_matrices(
						light.position,
						*rotation,
						*fov,
						light_info.shadow_map_size as f32,
					);

					let data_size = light_info.shadow_map_size * light_info.shadow_map_size;
					let depth_data = &mut self.shadow_maps_data
						[light_info.shadow_map_data_offset .. light_info.shadow_map_data_offset + data_size as usize];

					self.shadows_maps_renderer.draw_map(
						depth_data,
						light_info.shadow_map_size,
						light_info.shadow_map_size,
						&depth_matrices,
						&self.inline_models_index,
					);

					make_shadow_map_circle(depth_data, light_info.shadow_map_size);
				},
			}
		}
	}

	// Call this only after dynamic lights preparation.
	fn prepare_submodels(&mut self, frame_info: &FrameInfo)
	{
		self.inline_models_index.position_models(&frame_info.submodel_entities);

		for (index, submodel_info) in self.submodels_info.iter_mut().enumerate()
		{
			let model_matrix_opt = self.inline_models_index.get_model_matrix(index as u32);

			submodel_info.matrices = if let Some(model_matrix) = model_matrix_opt
			{
				let model_matrix_inverse = model_matrix.transpose().invert().unwrap();
				Some(VisibleSubmodelMatrices {
					world_planes_matrix: model_matrix_inverse,
					camera_matrices: CameraMatrices {
						view_matrix: frame_info.camera_matrices.view_matrix * model_matrix,
						planes_matrix: frame_info.camera_matrices.planes_matrix * model_matrix_inverse,
						position: frame_info.camera_matrices.position,
					},
				})
			}
			else
			{
				None
			};

			// Search for dynamic lights, affecting this submodel.
			submodel_info.dynamic_lights.clear();
			if let Some(model_matrix) = model_matrix_opt
			{
				// Get initial bounding box, transformm its vertices and obtain new bounding box (around transformed vertices).
				let bbox_vertices_transformed = self
					.inline_models_index
					.get_model_bbox_initial(index as u32)
					.get_corners_vertices()
					.map(|v| (model_matrix * v.extend(1.0)).truncate());

				let mut bbox_world_space = BBox::from_point(&bbox_vertices_transformed[0]);
				for v in &bbox_vertices_transformed[1 ..]
				{
					bbox_world_space.extend_with_point(v);
				}

				// For each light check intersection against bbox of transformed model.
				for (index, (light, light_info)) in frame_info
					.lights
					.iter()
					.zip(self.dynamic_lights_info.iter())
					.enumerate()
				{
					if !light_info.visible
					{
						continue;
					}

					if light.position.x - light.radius > bbox_world_space.max.x ||
						light.position.x + light.radius < bbox_world_space.min.x ||
						light.position.y - light.radius > bbox_world_space.max.y ||
						light.position.y + light.radius < bbox_world_space.min.y ||
						light.position.z - light.radius > bbox_world_space.max.z ||
						light.position.z + light.radius < bbox_world_space.min.z
					{
						continue;
					}
					submodel_info.dynamic_lights.push(index as DynamicObjectId);
				}
			}
		}
	}

	// Call this after lights preparation.
	fn prepare_decals(&mut self, frame_info: &FrameInfo)
	{
		self.decals_index.position_decals(&frame_info.decals);

		self.decals_info.clear();
		for decal in &frame_info.decals
		{
			// TODO - maybe check visibility of decals and skip processing invisible decals?

			let decal_matrix = get_object_matrix_with_scale(decal.position, decal.rotation, decal.scale);

			let camera_planes_matrix =
				frame_info.camera_matrices.planes_matrix * decal_matrix.transpose().invert().unwrap();

			// Calculate light cube for this decal.
			// Do it only for one position within decal.
			// Such approach gives good results for small decals.
			let mut light_cube = LightCube::new();
			if decal.lightmap_light_scale > 0.0
			{
				let min_square_distance = decal.scale.magnitude2() * 0.25;
				for (light, light_info) in frame_info.lights.iter().zip(self.dynamic_lights_info.iter())
				{
					if !light_info.visible
					{
						continue;
					}

					let vec_to_light = light.position - decal.position;
					let square_dist = vec_to_light.magnitude2().max(min_square_distance);
					let inv_square_dist = 1.0 / square_dist;
					let light_inv_square_radius = 1.0 / (light.radius * light.radius);
					if inv_square_dist < light_inv_square_radius
					{
						continue;
					}

					let shadow_factor = match &light.shadow_type
					{
						DynamicLightShadowType::None => 1.0,
						DynamicLightShadowType::Cubemap => cube_shadow_map_fetch(
							&create_dynamic_light_cube_shadow_map(light_info, &self.shadow_maps_data),
							&vec_to_light,
						),
						DynamicLightShadowType::Projector { rotation, fov } => projector_shadow_map_fetch(
							&create_dynamic_light_projector_shadow_map(
								rotation,
								*fov,
								light_info,
								&self.shadow_maps_data,
							),
							&vec_to_light,
						),
					};
					if shadow_factor <= 0.0
					{
						continue;
					}

					let scale = shadow_factor * (inv_square_dist - light_inv_square_radius);
					light_cube.add_light_sample(
						&vec_to_light,
						&[light.color[0] * scale, light.color[1] * scale, light.color[2] * scale],
					);
				}
			}

			let decal_info = DecalInfo {
				camera_planes_matrix,
				dynamic_light: light_cube.convert_into_light_grid_sample(),
			};
			self.decals_info.push(decal_info);
		}

		debug_assert!(self.decals_info.len() == frame_info.decals.len());
	}

	// Call this after visible leafs search.
	fn prepare_sprites(&mut self, frame_info: &FrameInfo)
	{
		self.sprites_index.position_sprites(&frame_info.sprites);

		let view_matrix_inverse = frame_info.camera_matrices.view_matrix.invert().unwrap();

		// let v_vec_base = -Vec3f::unit_z();
		let v_vec_base_initial = (view_matrix_inverse * Vec4f::unit_y()).truncate();
		let v_vec_base = v_vec_base_initial / v_vec_base_initial.magnitude();

		let u_vec_base_initial = (view_matrix_inverse * Vec4f::unit_x()).truncate();
		let u_vec_base = u_vec_base_initial / u_vec_base_initial.magnitude();

		self.sprites_info.clear();
		for sprite in &frame_info.sprites
		{
			let (u_vec_normalized, v_vec_normalized) = match sprite.orientation
			{
				SpriteOrientation::FacingTowardsCamera =>
				{
					let vec_to_camera = frame_info.camera_matrices.position - sprite.position;
					let plane_normal = vec_to_camera / vec_to_camera.magnitude().max(0.001);

					let v_vec_projected_to_plane = v_vec_base - plane_normal * v_vec_base.dot(plane_normal);

					let v_vec_projected_len = v_vec_projected_to_plane.magnitude();
					let v_vec_normalized = if v_vec_projected_len < 0.0001
					{
						Vec3f::unit_y()
					}
					else
					{
						v_vec_projected_to_plane / v_vec_projected_len
					};

					// Should be normalized, since both vectors are normalied and perpendicular.
					let u_vec_normalized = plane_normal.cross(v_vec_normalized);
					(u_vec_normalized, v_vec_normalized)
				},
				SpriteOrientation::ParallelToCameraPlane => (u_vec_base, v_vec_base),
			};

			let texture_mip0 = &sprite.texture[0];
			let ratio_h_w = (texture_mip0.size[1] as f32) / (texture_mip0.size[0] as f32);
			let step_u = sprite.radius * inv_sqrt_fast(1.0 + ratio_h_w * ratio_h_w);
			let ratio_w_h = (texture_mip0.size[0] as f32) / (texture_mip0.size[1] as f32);
			let step_v = sprite.radius * inv_sqrt_fast(1.0 + ratio_w_h * ratio_w_h);

			let u_vec = u_vec_normalized * step_u;
			let v_vec = v_vec_normalized * step_v;
			let vertices = [
				sprite.position + u_vec + v_vec,
				sprite.position + u_vec - v_vec,
				sprite.position - u_vec - v_vec,
				sprite.position - u_vec + v_vec,
			];

			let vertices_projected = vertices.map(|v| {
				let v_projected = frame_info.camera_matrices.view_matrix * v.extend(1.0);
				Vec3f::new(v_projected.x, v_projected.y, v_projected.w)
			});

			let sprite_info = SpriteInfo {
				vertices_projected,
				light: [1.0, 1.0, 1.0], // TODO - use proper light
			};
			self.sprites_info.push(sprite_info);
		}

		debug_assert!(self.sprites_info.len() == frame_info.sprites.len());
	}

	// Call this after visible leafs search.
	fn prepare_dynamic_models(&mut self, camera_matrices: &CameraMatrices, models: &[ModelEntity])
	{
		self.dynamic_models_index.position_models(models);

		self.visible_dynamic_meshes_list.clear();

		self.dynamic_model_to_dynamic_meshes_index
			.resize(models.len(), DynamicModelInfo::default());

		// Reserve place in vertex/triangle buffers for each visible mesh.
		let mut vertices_offset = 0;
		let mut triangles_offset = 0;
		for (entity_index, (model, dynamic_model_info)) in models
			.iter()
			.zip(self.dynamic_model_to_dynamic_meshes_index.iter_mut())
			.enumerate()
		{
			dynamic_model_info.first_visible_mesh = 0;
			dynamic_model_info.num_visible_meshes = 0;

			// Calculate matrices.
			let model_matrix = get_object_matrix(model.position, model.rotation);
			let model_view_matrix = camera_matrices.view_matrix * model_matrix;

			// Transform bbox.
			let bbox = get_current_triangle_model_bbox(&model.model, &model.animation);
			let bbox_vertices_transformed = bbox.get_corners_vertices().map(|pos| {
				let pos_transformed = model_view_matrix * pos.extend(1.0);
				Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w)
			});

			let clipping_polygon = if let Some(c) = calculate_triangle_model_screen_polygon(&bbox_vertices_transformed)
			{
				c
			}
			else
			{
				// Model is behind camera plane.
				continue;
			};

			let mut visible = model.is_view_model;
			for leaf_index in self.dynamic_models_index.get_object_leafs(entity_index)
			{
				if let Some(mut leaf_clipping_polygon) =
					self.visibility_calculator.get_current_frame_leaf_bounds(*leaf_index)
				{
					leaf_clipping_polygon.intersect(&clipping_polygon);
					if leaf_clipping_polygon.is_valid_and_non_empty()
					{
						visible = true;
						break;
					}
				}
			}

			if !visible
			{
				continue;
			}

			dynamic_model_info.first_visible_mesh = self.visible_dynamic_meshes_list.len() as u32;

			let model_camera_matrices = CameraMatrices {
				view_matrix: model_view_matrix,
				planes_matrix: camera_matrices.planes_matrix * model_matrix.transpose().invert().unwrap(),
				position: Vec3f::zero(),
			};

			for (mesh_index, mesh) in model.model.meshes.iter().enumerate()
			{
				self.visible_dynamic_meshes_list.push(VisibleDynamicMeshInfo {
					entity_index: entity_index as u32,
					mesh_index: mesh_index as u32,
					vertices_offset,
					triangles_offset,
					num_visible_triangles: 0, // set later
					bbox_vertices_transformed,
					clipping_polygon,
					model_matrix,
					camera_matrices: model_camera_matrices,
					mip: 0, // Set later.
				});

				let num_vertices = match &mesh.vertex_data
				{
					VertexData::NonAnimated(v) => v.len(),
					VertexData::VertexAnimated { constant, .. } => constant.len(),
					VertexData::SkeletonAnimated(v) => v.len(),
				};

				vertices_offset += num_vertices;
				triangles_offset += mesh.triangles.len();
			}

			dynamic_model_info.num_visible_meshes = model.model.meshes.len() as u32;
		}

		if vertices_offset > self.dynamic_meshes_vertices.len()
		{
			self.dynamic_meshes_vertices.resize(
				vertices_offset,
				ModelVertex3d {
					pos: Vec3f::zero(),
					tc: Vec2f::zero(),
					light: [0.0; 3],
				},
			);
		}
		if triangles_offset > self.dynamic_meshes_triangles.len()
		{
			self.dynamic_meshes_triangles.resize(triangles_offset, [0, 0, 0]);
		}
	}

	fn build_dynamic_models_buffers(&mut self, dynamic_lights: &[DynamicLight], models: &[ModelEntity])
	{
		// Prepare array of dynamic lights with shadowmaps.
		// TODO - avoid allocation.
		let dynamic_lights_info = &self.dynamic_lights_info;
		let shadow_maps_data = &self.shadow_maps_data;
		let lights: Vec<DynamicLightWithShadow> = dynamic_lights
			.iter()
			.zip(dynamic_lights_info.iter())
			.map(|(light, light_info)| create_dynamic_light_with_shadow(light, light_info, shadow_maps_data))
			.collect();

		// It is safe to share vertices and triangle buffers since each mesh uses its own region.
		let dst_vertices_shared = SharedMutSlice::new(&mut self.dynamic_meshes_vertices);
		let dst_triangles_shared = SharedMutSlice::new(&mut self.dynamic_meshes_triangles);

		let map = &self.map;
		let mip_bias = self.mip_bias;

		let func = |visible_dynamic_mesh: &mut VisibleDynamicMeshInfo| {
			let model = &models[visible_dynamic_mesh.entity_index as usize];
			let animation = &model.animation;

			visible_dynamic_mesh.mip = calculate_triangle_model_texture_mip(
				&visible_dynamic_mesh.camera_matrices.view_matrix,
				&get_current_triangle_model_bbox(&model.model, animation),
				model.texture[0].size,
				mip_bias,
			);

			let texture = &model.texture[visible_dynamic_mesh.mip as usize];
			let mesh = &model.model.meshes[visible_dynamic_mesh.mesh_index as usize];

			// Perform vertices transformation.
			let dst_mesh_vertices = unsafe { &mut dst_vertices_shared.get()[visible_dynamic_mesh.vertices_offset ..] };

			animate_and_transform_triangle_mesh_vertices(
				&model.model,
				mesh,
				animation,
				&get_model_light(map, &lights, model, &visible_dynamic_mesh.model_matrix),
				&visible_dynamic_mesh.model_matrix,
				&visible_dynamic_mesh.camera_matrices.view_matrix,
				&Vec2f::new(texture.size[0] as f32, texture.size[1] as f32),
				&model.model.tc_shift,
				dst_mesh_vertices,
			);

			// Copy, filter and sort triangles.
			let dst_triangles = unsafe { &mut dst_triangles_shared.get()[visible_dynamic_mesh.triangles_offset ..] };
			visible_dynamic_mesh.num_visible_triangles =
				reject_triangle_model_back_faces(&dst_mesh_vertices, &mesh.triangles, dst_triangles);

			sort_model_triangles(
				&dst_mesh_vertices,
				&mut dst_triangles[.. visible_dynamic_mesh.num_visible_triangles],
			);
		};

		let num_threads = rayon::current_num_threads();
		if num_threads == 1
		{
			self.visible_dynamic_meshes_list.iter_mut().for_each(func);
		}
		else
		{
			self.visible_dynamic_meshes_list.par_iter_mut().for_each(func);
		}
	}

	fn perform_rasterization<ColorT: AbstractColor>(
		&self,
		pixels: &mut [ColorT],
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
	)
	{
		let screen_rect = rect_splitting::Rect {
			min: Vec2f::new(0.0, 0.0),
			max: Vec2f::new(surface_info.width as f32, surface_info.height as f32),
		};

		let num_threads = rayon::current_num_threads();
		if num_threads == 1
		{
			let mut rasterizer = Rasterizer::new(
				pixels,
				&surface_info,
				ClipRect {
					min_x: 0,
					min_y: 0,
					max_x: surface_info.width as i32,
					max_y: surface_info.height as i32,
				},
			);

			let viewport_clipping_polygon = ClippingPolygon::from_box(
				screen_rect.min.x,
				screen_rect.min.y,
				screen_rect.max.x,
				screen_rect.max.y,
			);

			self.perform_rasterization_for_viewport_part(&mut rasterizer, frame_info, &viewport_clipping_polygon);
		}
		else
		{
			let pixels_shared = SharedMutSlice::new(pixels);

			// Split viewport rect into several rects for each thread.
			// Use tricky splitting method that avoid creation of thin rects.
			// This is needed to speed-up rasterization - reject as much polygons outside given rect, as possible.
			let mut rects = [rect_splitting::Rect::default(); 64];
			rect_splitting::split_rect(&screen_rect, num_threads as u32, &mut rects);

			rects[.. num_threads].par_iter().for_each(|rect| {
				let pixels_cur = unsafe { pixels_shared.get() };

				// Create rasterizer with custom clip rect in order to perform pixel-perfect clipping.
				// TODO - change this. Just create rasterizer with shifted raster and shift vertex coordinates instead.
				let mut rasterizer = Rasterizer::new(
					pixels_cur,
					&surface_info,
					ClipRect {
						min_x: rect.min.x as i32,
						min_y: rect.min.y as i32,
						max_x: rect.max.x as i32,
						max_y: rect.max.y as i32,
					},
				);

				// Extend it just a bit to fix possible gaps.
				let mut rect_corrected = *rect;
				rect_corrected.min -= Vec2f::new(0.5, 0.5);
				rect_corrected.max += Vec2f::new(0.5, 0.5);

				// Use clipping polygon to totally reject whole leafs and polygons.
				let viewport_clipping_polygon = ClippingPolygon::from_box(
					rect_corrected.min.x,
					rect_corrected.min.y,
					rect_corrected.max.x,
					rect_corrected.max.y,
				);

				self.perform_rasterization_for_viewport_part(&mut rasterizer, frame_info, &viewport_clipping_polygon);
			});
		}
	}

	fn perform_rasterization_for_viewport_part<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		frame_info: &FrameInfo,
		viewport_clipping_polygon: &ClippingPolygon,
	)
	{
		if !self.config.invert_polygons_order
		{
			self.draw_skybox(
				rasterizer,
				&frame_info.camera_matrices,
				&frame_info.skybox_rotation,
				&viewport_clipping_polygon,
			);
		}

		let root_node = bsp_map_compact::get_root_node_index(&self.map);
		self.draw_tree_r(rasterizer, frame_info, &viewport_clipping_polygon, root_node);

		if self.config.invert_polygons_order
		{
			self.draw_skybox(
				rasterizer,
				&frame_info.camera_matrices,
				&frame_info.skybox_rotation,
				&viewport_clipping_polygon,
			);
		}

		self.draw_view_models(rasterizer, &viewport_clipping_polygon, &frame_info.model_entities);
	}

	fn prepare_polygons_surfaces(&mut self, camera_matrices: &CameraMatrices)
	{
		self.current_frame_visible_polygons.clear();

		self.current_sky = None;

		let mut surfaces_pixels_accumulated_offset = 0;

		// TODO - try to speed-up iteration, do not scan all leafs.
		for i in 0 .. self.map.leafs.len()
		{
			if let Some(leaf_bounds) = self.visibility_calculator.get_current_frame_leaf_bounds(i as u32)
			{
				let leaf = &self.map.leafs[i];
				// TODO - maybe just a little bit extend clipping polygon?
				let clip_planes = leaf_bounds.get_clip_planes();
				for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
				{
					self.prepare_polygon_surface(
						camera_matrices,
						&clip_planes,
						&mut surfaces_pixels_accumulated_offset,
						polygon_index as usize,
					);
				}
			}
		}

		// Prepare surfaces for submodels.
		// Do this only for sumbodels located in visible leafs.
		for index in 0 .. self.map.submodels.len()
		{
			let submodel_matrices = if let Some(m) = self.submodels_info[index].matrices
			{
				m
			}
			else
			{
				continue;
			};

			let mut bounds: Option<ClippingPolygon> = None;
			for &leaf_index in self.inline_models_index.get_model_leafs(index as u32)
			{
				if let Some(leaf_bounds) = self.visibility_calculator.get_current_frame_leaf_bounds(leaf_index)
				{
					if let Some(bounds) = &mut bounds
					{
						bounds.extend(&leaf_bounds);
					}
					else
					{
						bounds = Some(leaf_bounds);
					}
				}
			}

			let clip_planes = if let Some(b) = bounds
			{
				b.get_clip_planes()
			}
			else
			{
				continue;
			};

			let submodel = &self.map.submodels[index];
			for polygon_index in
				submodel.first_polygon as usize .. (submodel.first_polygon + submodel.num_polygons) as usize
			{
				self.prepare_polygon_surface(
					&submodel_matrices.camera_matrices,
					&clip_planes,
					&mut surfaces_pixels_accumulated_offset,
					polygon_index,
				);

				// If this submodel polygon is visible in current frame - recalculate its basis vecs, using transformations of submodel.
				// This is needed for specular and/or dynamic ligting.
				let polygon_data = &mut self.polygons_data[polygon_index];
				if polygon_data.visible_frame == self.current_frame
				{
					let polygon = &self.map.polygons[polygon_index];

					let plane_transformed_vec =
						submodel_matrices.world_planes_matrix * polygon.plane.vec.extend(-polygon.plane.dist);
					let tc_equation_transformed_vecs = [
						submodel_matrices.world_planes_matrix *
							polygon.tex_coord_equation[0]
								.vec
								.extend(polygon.tex_coord_equation[0].dist),
						submodel_matrices.world_planes_matrix *
							polygon.tex_coord_equation[1]
								.vec
								.extend(polygon.tex_coord_equation[1].dist),
					];

					polygon_data.basis_vecs = PolygonBasisVecs::form_plane_and_tex_coord_equation(
						&Plane {
							vec: plane_transformed_vec.truncate(),
							dist: -plane_transformed_vec.w,
						},
						&[
							Plane {
								vec: tc_equation_transformed_vecs[0].truncate(),
								dist: tc_equation_transformed_vecs[0].w,
							},
							Plane {
								vec: tc_equation_transformed_vecs[1].truncate(),
								dist: tc_equation_transformed_vecs[1].w,
							},
						],
					);
				}
			}
		}

		self.num_visible_surfaces_pixels = surfaces_pixels_accumulated_offset;
	}

	fn allocate_surfaces_pixels<ColorT>(&mut self)
	{
		// Resize surfaces pixels vector only up to avoid filling it with zeros each frame.
		let target_size = self.num_visible_surfaces_pixels * std::mem::size_of::<ColorT>();
		if self.surfaces_pixels.len() < target_size
		{
			self.surfaces_pixels.resize(target_size, 0);
		}
	}

	fn prepare_polygon_surface(
		&mut self,
		camera_matrices: &CameraMatrices,
		clip_planes: &ClippingPolygonPlanes,
		surfaces_pixels_accumulated_offset: &mut usize,
		polygon_index: usize,
	)
	{
		let polygon_data = &mut self.polygons_data[polygon_index];

		let polygon = &self.map.polygons[polygon_index];

		let plane_transformed = camera_matrices.planes_matrix * polygon.plane.vec.extend(-polygon.plane.dist);
		// Cull back faces.
		if plane_transformed.w <= 0.0
		{
			return;
		}

		let material = self.materials_processor.get_material(polygon.texture);

		if !material.draw
		{
			// Do not prepare surfaces for invisible polygons.
			return;
		}

		// Transform polygon vertices once and put transformation result into transformed vertices container.
		// Use these vertices later also for rasterization.
		let polygon_vertices_range =
			(polygon.first_vertex as usize) .. ((polygon.first_vertex + polygon.num_vertices) as usize);
		let polygon_vertices = &self.map.vertices[polygon_vertices_range.clone()];
		let polygon_vertices_transformed = &mut self.vertices_transformed[polygon_vertices_range];

		for (in_vertex, out_vertex) in polygon_vertices.iter().zip(polygon_vertices_transformed.iter_mut())
		{
			let vertex_transformed = camera_matrices.view_matrix * in_vertex.extend(1.0);
			*out_vertex = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
		}

		let mut vertices_2d = [Vec2f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
		let vertex_count = project_and_clip_polygon(clip_planes, polygon_vertices_transformed, &mut vertices_2d[..]);
		if vertex_count < 3
		{
			return;
		}

		if material.skybox.is_some()
		{
			// Do not draw sky polygons but update bounds.
			let mut polygon_bounds = ClippingPolygon::from_point(&vertices_2d[0]);
			for p in &vertices_2d[1 .. vertex_count]
			{
				polygon_bounds.extend_with_point(p);
			}

			if let Some(current_sky) = &mut self.current_sky
			{
				current_sky.1.extend(&polygon_bounds);
			}
			else
			{
				self.current_sky = Some((polygon.texture, polygon_bounds));
			}
			return;
		}

		let depth_equation = DepthEquation::from_transformed_plane_equation(&plane_transformed);

		let tex_coord_equation = &polygon.tex_coord_equation;

		// Calculate texture coordinates equations.
		let tc_basis_transformed = [
			camera_matrices.planes_matrix * tex_coord_equation[0].vec.extend(tex_coord_equation[0].dist),
			camera_matrices.planes_matrix * tex_coord_equation[1].vec.extend(tex_coord_equation[1].dist),
		];
		// Equation projeted to polygon plane.
		let tc_equation = TexCoordEquation::from_depth_equation_and_transformed_tex_coord_equations(
			&depth_equation,
			&tc_basis_transformed,
		);

		let mip = calculate_mip(
			&vertices_2d[.. vertex_count],
			&depth_equation,
			&tc_equation,
			self.mip_bias,
		);

		let tc_equation_scaled = tc_equation * (1.0 / ((1 << mip) as f32));

		// Calculate minimum/maximum texture coordinates.
		// Use clipped vertices for this.
		// With such approach we can allocate data only for visible part of surface, not whole polygon.
		let inf = (1 << 29) as f32; // Maximum value without integer overflow in subtraction.
		let max_z = (1 << 16) as f32;
		let mut tc_min = [inf, inf];
		let mut tc_max = [-inf, -inf];
		for p in &vertices_2d[.. vertex_count]
		{
			// Limit inv_z in case of computational errors (if it is 0 or negative).
			let inv_z = depth_equation.sample_point(p).max(1.0 / max_z);
			let z = 1.0 / inv_z;
			for i in 0 .. 2
			{
				let tc = z *
					(tc_equation_scaled.d_tc_dx[i] * p.x +
						tc_equation_scaled.d_tc_dy[i] * p.y +
						tc_equation_scaled.k[i]);
				if tc < tc_min[i]
				{
					tc_min[i] = tc;
				}
				if tc > tc_max[i]
				{
					tc_max[i] = tc;
				}
			}
		}

		let mut surface_tc_min = [0, 0];
		let mut surface_size = [0, 0];
		for i in 0 .. 2
		{
			// Reduce min/max texture coordinates slightly to avoid adding extra pixels
			// in case if min/max tex coord is exact integer, but slightly changed due to computational errors.
			let tc_reduce_eps = 1.0 / 32.0;
			tc_min[i] += tc_reduce_eps;
			tc_max[i] -= tc_reduce_eps;

			// Clamp coordinates to min/max polygon coordinates (they may be out of range because of computational errors).
			// It's important to clamp texture coordinates to avoid reading lightmap outside borders.
			let round_mask = !((lightmap::LIGHTMAP_SCALE as i32) - 1);
			let tc_min_round_down = (polygon.tex_coord_min[i] & round_mask) >> mip;
			let tc_max_round_up =
				((polygon.tex_coord_max[i] + (lightmap::LIGHTMAP_SCALE as i32) - 1) & round_mask) >> mip;

			let mut tc_min_int = (tc_min[i].max(-inf).floor() as i32).max(tc_min_round_down);
			let mut tc_max_int = (tc_max[i].min(inf).ceil() as i32).min(tc_max_round_up);

			if tc_min_int >= tc_max_int
			{
				// Degenerte case - correct surface size.
				tc_min_int = tc_min_int.min(tc_max_round_up - 1);
				tc_max_int = tc_min_int + 1;
			}

			surface_tc_min[i] = tc_min_int;
			surface_size[i] = tc_max_int - tc_min_int;
			debug_assert!(tc_min_int >= tc_min_round_down);
			debug_assert!(tc_max_int <= tc_max_round_up);
		}

		let surface_pixels_offset = *surfaces_pixels_accumulated_offset;
		*surfaces_pixels_accumulated_offset += (surface_size[0] * surface_size[1]) as usize;

		polygon_data.visible_frame = self.current_frame;
		polygon_data.depth_equation = depth_equation;
		polygon_data.tex_coord_equation = tc_equation_scaled;
		polygon_data.surface_pixels_offset = surface_pixels_offset;
		polygon_data.surface_size = [surface_size[0] as u32, surface_size[1] as u32];
		polygon_data.mip = mip;
		polygon_data.surface_tc_min = surface_tc_min;

		// Correct texture coordinates equation to compensate shift to surface rect.
		for i in 0 .. 2
		{
			let tc_min = surface_tc_min[i] as f32;
			polygon_data.tex_coord_equation.d_tc_dx[i] -= tc_min * depth_equation.d_inv_z_dx;
			polygon_data.tex_coord_equation.d_tc_dy[i] -= tc_min * depth_equation.d_inv_z_dy;
			polygon_data.tex_coord_equation.k[i] -= tc_min * depth_equation.k;
		}

		self.current_frame_visible_polygons.push(polygon_index as u32);
	}

	fn build_polygons_surfaces<ColorT: AbstractColor>(
		&mut self,
		camera_matrices: &CameraMatrices,
		dynamic_lights: &[DynamicLight],
	)
	{
		// Prepare array of dynamic lights with shadowmaps.
		// TODO - avoid allocation.
		let dynamic_lights_info = &self.dynamic_lights_info;
		let shadow_maps_data = &self.shadow_maps_data;
		let lights: Vec<DynamicLightWithShadow> = dynamic_lights
			.iter()
			.zip(dynamic_lights_info.iter())
			.map(|(light, light_info)| create_dynamic_light_with_shadow(light, light_info, shadow_maps_data))
			.collect();

		// Used only to initialize references.
		let dummy_light = DynamicLightWithShadow {
			position: Vec3f::zero(),
			radius: 1.0,
			inv_square_radius: 1.0,
			color: [0.0; 3],
			shadow_map: ShadowMap::None,
		};
		const MAX_POLYGON_LIGHTS: usize = 6;

		// Perform parallel surfaces building.
		// Use "unsafe" to write into surfaces data concurrently.
		// It is fine since each surface uses its own region.

		let lightmaps_data = &self.map.lightmaps_data;
		let directional_lightmaps_data = &self.map.directional_lightmaps_data;
		let polygons = &self.map.polygons;
		let polygons_data = &self.polygons_data;
		let materials_processor = &self.materials_processor;

		let use_directional_lightmap = self.config.use_directional_lightmaps && !directional_lightmaps_data.is_empty();

		let surfaces_pixels_casted = unsafe { self.surfaces_pixels.align_to_mut::<ColorT>().1 };
		let surfaces_pixels_shared = SharedMutSlice::new(surfaces_pixels_casted);

		let func = |&polygon_index| {
			let polygon = &polygons[polygon_index as usize];
			let polygon_data = &polygons_data[polygon_index as usize];
			let surface_size = polygon_data.surface_size;

			// Collect lights, affecting this polygon.
			let mut polygon_lights = [&dummy_light; MAX_POLYGON_LIGHTS];
			let mut num_polygon_lights = 0;

			let lights_list = match polygon_data.parent
			{
				DrawPolygonParent::Leaf(leaf_index) =>
				{
					// Check only lights, located inside leaf of this polygon.
					self.dynamic_lights_index.get_leaf_objects(leaf_index)
				},
				DrawPolygonParent::Submodel(submodel_index) =>
				{
					// Check only lights, affecting this submodel.
					&self.submodels_info[submodel_index as usize].dynamic_lights
				},
			};
			for light_index in lights_list
			{
				let light = &lights[*light_index as usize];
				if polygon_is_affected_by_light(polygon, &polygon_data.basis_vecs, light)
				{
					polygon_lights[num_polygon_lights] = light;
					num_polygon_lights += 1;
					if num_polygon_lights == MAX_POLYGON_LIGHTS
					{
						break;
					}
				}
			}

			let basis_vecs_scaled = polygon_data.basis_vecs.get_basis_vecs_for_mip(polygon_data.mip);

			let texture = &materials_processor.get_texture(polygon.texture)[polygon_data.mip as usize];
			let surface_data = unsafe {
				&mut surfaces_pixels_shared.get()[polygon_data.surface_pixels_offset ..
					polygon_data.surface_pixels_offset + (surface_size[0] * surface_size[1]) as usize]
			};

			let mut lightmap_tc_shift: [u32; 2] = [0, 0];
			for i in 0 .. 2
			{
				let round_mask = !((lightmap::LIGHTMAP_SCALE as i32) - 1);
				let shift =
					polygon_data.surface_tc_min[i] - ((polygon.tex_coord_min[i] & round_mask) >> polygon_data.mip);
				debug_assert!(shift >= 0);
				lightmap_tc_shift[i] = shift as u32;
			}

			let lightmap_size = lightmap::get_polygon_lightmap_size(polygon);

			let lightmap_scale_log2 = lightmap::LIGHTMAP_SCALE_LOG2 - polygon_data.mip;
			if use_directional_lightmap
			{
				let polygon_lightmap_data = if polygon.lightmap_data_offset != 0
				{
					&directional_lightmaps_data[polygon.lightmap_data_offset as usize ..
						((polygon.lightmap_data_offset + lightmap_size[0] * lightmap_size[1]) as usize)]
				}
				else
				{
					&[]
				};
				build_surface_directional_lightmap(
					&basis_vecs_scaled,
					surface_size,
					polygon_data.surface_tc_min,
					texture,
					lightmap_size,
					lightmap_scale_log2,
					lightmap_tc_shift,
					polygon_lightmap_data,
					&polygon_lights[.. num_polygon_lights],
					&camera_matrices.position,
					surface_data,
				);
			}
			else
			{
				let polygon_lightmap_data = if polygon.lightmap_data_offset != 0
				{
					&lightmaps_data[polygon.lightmap_data_offset as usize ..
						((polygon.lightmap_data_offset + lightmap_size[0] * lightmap_size[1]) as usize)]
				}
				else
				{
					&[]
				};
				build_surface_simple_lightmap(
					&basis_vecs_scaled,
					surface_size,
					polygon_data.surface_tc_min,
					texture,
					lightmap_size,
					lightmap_scale_log2,
					lightmap_tc_shift,
					polygon_lightmap_data,
					&polygon_lights[.. num_polygon_lights],
					&camera_matrices.position,
					surface_data,
				);
			}
		};

		if rayon::current_num_threads() == 1
		{
			// Perform single-threaded surfaces build using main thread.
			self.current_frame_visible_polygons.iter().for_each(func);
		}
		else
		{
			// Perform parallel surfaces building.
			self.current_frame_visible_polygons.par_iter().for_each(func);
		}
	}

	fn draw_view_models<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		viewport_clipping_polygon: &ClippingPolygon,
		models: &[ModelEntity],
	)
	{
		for (dynamic_model_index, model) in models.iter().enumerate()
		{
			if !model.is_view_model
			{
				continue;
			}
			let entry = self.dynamic_model_to_dynamic_meshes_index[dynamic_model_index];
			for visible_mesh_index in entry.first_visible_mesh .. entry.first_visible_mesh + entry.num_visible_meshes
			{
				self.draw_mesh(
					rasterizer,
					&viewport_clipping_polygon,
					&[], // No 3d clip planes.
					models,
					&self.visible_dynamic_meshes_list[visible_mesh_index as usize],
				);
			}
		}
	}

	fn draw_skybox<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		camera_matrices: &CameraMatrices,
		skybox_rotation: &QuaternionF,
		viewport_clipping_polygon: &ClippingPolygon,
	)
	{
		let (material_index, mut bounds) = if let Some(current_sky) = self.current_sky
		{
			current_sky
		}
		else
		{
			return;
		};

		bounds.intersect(viewport_clipping_polygon);
		if bounds.is_empty_or_invalid()
		{
			return;
		}

		let skybox_textures = if let Some(t) = self.materials_processor.get_skybox_textures(material_index)
		{
			t
		}
		else
		{
			return;
		};

		const BOX_VERTICES: [[f32; 3]; 8] = [
			[-1.0, -1.0, -1.0],
			[-1.0, -1.0, 1.0],
			[-1.0, 1.0, -1.0],
			[-1.0, 1.0, 1.0],
			[1.0, -1.0, -1.0],
			[1.0, -1.0, 1.0],
			[1.0, 1.0, -1.0],
			[1.0, 1.0, 1.0],
		];

		let side_plane_dist = -1.0;
		let side_tc_shift = 1.0;
		let bbox_polygons = [
			// -X
			([0, 1, 3, 2], Vec3f::unit_x(), Vec3f::unit_y(), -Vec3f::unit_z()),
			// +X
			([4, 6, 7, 5], -Vec3f::unit_x(), -Vec3f::unit_y(), -Vec3f::unit_z()),
			// -Y
			([0, 4, 5, 1], Vec3f::unit_y(), -Vec3f::unit_x(), -Vec3f::unit_z()),
			// +Y
			([2, 3, 7, 6], -Vec3f::unit_y(), Vec3f::unit_x(), -Vec3f::unit_z()),
			// -Z
			([0, 2, 6, 4], Vec3f::unit_z(), Vec3f::unit_x(), -Vec3f::unit_y()),
			// +Z
			([1, 5, 7, 3], -Vec3f::unit_z(), Vec3f::unit_x(), Vec3f::unit_y()),
		];

		let skybox_matrix = get_object_matrix(camera_matrices.position, *skybox_rotation);
		let skybox_matrix_inverse = skybox_matrix.transpose().invert().unwrap();
		let skybox_view_matrix = camera_matrices.view_matrix * skybox_matrix;
		let skybox_planes_matrix = camera_matrices.planes_matrix * skybox_matrix_inverse;

		let box_vertices_transformed = BOX_VERTICES.map(|v| {
			let v_transformed = skybox_view_matrix * (Vec3f::from(v) * 4.0).extend(1.0);
			Vec3f::new(v_transformed.x, v_transformed.y, v_transformed.w)
		});

		let clip_planes = bounds.get_clip_planes();

		let mut vertices_2d = [Vec2f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory

		for (side, polygon) in bbox_polygons.iter().enumerate()
		{
			let side_textures = &skybox_textures[side];
			if side_textures[0].size == 0
			{
				// There is no texture for this side.
				continue;
			}

			let vertices_transformed = [
				box_vertices_transformed[polygon.0[0]],
				box_vertices_transformed[polygon.0[1]],
				box_vertices_transformed[polygon.0[2]],
				box_vertices_transformed[polygon.0[3]],
			];

			let depth_equation = DepthEquation::from_transformed_plane_equation(
				&(skybox_planes_matrix * polygon.1.extend(-side_plane_dist)),
			);

			let tc_equation_scale = side_textures[0].size as f32 * 0.5;

			// Calculate texture coordinates equations.
			let tc_basis_transformed = [
				skybox_planes_matrix * (polygon.2.extend(side_tc_shift) * tc_equation_scale),
				skybox_planes_matrix * (polygon.3.extend(side_tc_shift) * tc_equation_scale),
			];
			// Equation projeted to polygon plane.
			let tc_equation = TexCoordEquation::from_depth_equation_and_transformed_tex_coord_equations(
				&depth_equation,
				&tc_basis_transformed,
			);

			let mip = {
				let vertex_count = project_and_clip_polygon(&clip_planes, &vertices_transformed, &mut vertices_2d[..]);
				if vertex_count < 3
				{
					continue;
				}
				calculate_mip(
					&vertices_2d[.. vertex_count],
					&depth_equation,
					&tc_equation,
					self.mip_bias,
				)
			};

			let tc_equation_scaled = tc_equation * (1.0 / ((1 << mip) as f32));

			let side_texture = &side_textures[mip as usize];

			draw_polygon(
				rasterizer,
				&clip_planes,
				&vertices_transformed,
				&depth_equation,
				&tc_equation_scaled,
				&[side_texture.size, side_texture.size],
				&side_texture.pixels,
				material::BlendingMode::None,
			);
		}
	}

	fn draw_tree_r<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		frame_info: &FrameInfo,
		viewport_clipping_polygon: &ClippingPolygon,
		current_index: u32,
	)
	{
		if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
			if let Some(mut leaf_bounds) = self.visibility_calculator.get_current_frame_leaf_bounds(leaf)
			{
				leaf_bounds.intersect(viewport_clipping_polygon);
				if leaf_bounds.is_valid_and_non_empty()
				{
					self.draw_leaf(rasterizer, frame_info, &leaf_bounds, leaf);
				}
			}
		}
		else
		{
			let node = &self.map.nodes[current_index as usize];
			let plane_transformed = frame_info.camera_matrices.planes_matrix * node.plane.vec.extend(-node.plane.dist);
			let mut mask = if plane_transformed.w >= 0.0 { 1 } else { 0 };
			if self.config.invert_polygons_order
			{
				mask ^= 1;
			}
			for i in 0 .. 2
			{
				self.draw_tree_r(
					rasterizer,
					frame_info,
					viewport_clipping_polygon,
					node.children[(i ^ mask) as usize],
				);
			}
		}
	}

	fn draw_leaf<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		frame_info: &FrameInfo,
		bounds: &ClippingPolygon,
		leaf_index: u32,
	)
	{
		let leaf = &self.map.leafs[leaf_index as usize];

		// TODO - maybe just a little bit extend clipping polygon?
		let clip_planes = bounds.get_clip_planes();

		// Draw polygons of leaf itself.
		for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
		{
			let polygon = &self.map.polygons[polygon_index as usize];
			let polygon_data = &self.polygons_data[polygon_index as usize];
			if polygon_data.visible_frame != self.current_frame
			{
				continue;
			}

			draw_polygon(
				rasterizer,
				&clip_planes,
				&self.vertices_transformed
					[(polygon.first_vertex as usize) .. ((polygon.first_vertex + polygon.num_vertices) as usize)],
				&polygon_data.depth_equation,
				&polygon_data.tex_coord_equation,
				&polygon_data.surface_size,
				self.get_polygon_surface_data(polygon_data),
				self.materials_processor.get_material(polygon.texture).blending_mode,
			);
		}

		// Draw decals after all leaf polygons.
		let leaf_decals = self.decals_index.get_leaf_objects(leaf_index);
		if !leaf_decals.is_empty()
		{
			for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
			{
				self.draw_polygon_decals(rasterizer, &clip_planes, polygon_index, &frame_info.decals, leaf_decals);
			}
		}

		// Draw contents of leaf - submodels and triangle models.

		let leaf_submodels = self.inline_models_index.get_leaf_models(leaf_index);
		let leaf_dynamic_models = self.dynamic_models_index.get_leaf_objects(leaf_index);
		let leaf_sprites = self.sprites_index.get_leaf_objects(leaf_index);
		if leaf_submodels.is_empty() && leaf_dynamic_models.is_empty() && leaf_sprites.is_empty()
		{
			return;
		}

		// Collect clip planes, that will be used for models clipping.
		// TODO - use uninitialized memory.
		let mut leaf_clip_planes = [Plane {
			vec: Vec3f::zero(),
			dist: 0.0,
		}; MAX_LEAF_CLIP_PLANES];
		let mut num_clip_planes = 0;

		let mut add_clip_plane = |plane: Plane| {
			// We need to use planes with normalized vector in order to compare distances properly.
			let normal_length = plane.vec.magnitude();
			if normal_length < 0.00000000001
			{
				return;
			}
			let plane_normalized = Plane {
				vec: plane.vec / normal_length,
				dist: plane.dist / normal_length,
			};

			// Perform dedupliction - iterate over previous planes.
			// We have quadratic complexity here, but it is not a problem since number of planes are usually small (6 for cube-shaped leaf).
			for prev_plane in &mut leaf_clip_planes[.. num_clip_planes]
			{
				// Dot product is angle cos since vectors are normalized.
				let dot = plane_normalized.vec.dot(prev_plane.vec);
				if dot >= 1.0 - 1.0 / 256.0
				{
					// Planes are (almost) parallel.
					// Select plane with greater distance to clip more.
					prev_plane.dist = prev_plane.dist.max(plane_normalized.dist);
					return;
				}
			}

			if num_clip_planes == MAX_LEAF_CLIP_PLANES
			{
				return;
			}

			leaf_clip_planes[num_clip_planes] = plane_normalized;
			num_clip_planes += 1;
		};

		// Clip models polygons by portal planes of current leaf.
		for &portal_index in &self.map.leafs_portals
			[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
		{
			let portal = &self.map.portals[portal_index as usize];
			let clip_plane = if portal.leafs[0] == leaf_index
			{
				portal.plane
			}
			else
			{
				portal.plane.get_inverted()
			};
			add_clip_plane(clip_plane);
		}

		// Clip models also by polygons of current leaf.
		for polygon in
			&self.map.polygons[leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize]
		{
			add_clip_plane(polygon.plane);
		}

		let used_leaf_clip_planes = &mut leaf_clip_planes[.. num_clip_planes];

		// Perform planes transformation after deduplication.
		// This is needed because deduplication works badly in stretched camera space.
		// Also it's faster to transform only unique planes.
		for plane in used_leaf_clip_planes.iter_mut()
		{
			let plane_transformed_vec4 = frame_info.camera_matrices.planes_matrix * plane.vec.extend(-plane.dist);
			*plane = Plane {
				vec: plane_transformed_vec4.truncate(),
				dist: -plane_transformed_vec4.w,
			};
		}

		// Fast path for cases with single model, to avoid expensive sorting structures preparations.
		if leaf_submodels.len() == 1 && leaf_dynamic_models.len() == 0 && leaf_sprites.len() == 0
		{
			self.draw_submodel_in_leaf(
				rasterizer,
				frame_info,
				&clip_planes,
				used_leaf_clip_planes,
				leaf_decals,
				leaf_submodels[0],
			);
			return;
		}
		if leaf_submodels.len() == 0 && leaf_dynamic_models.len() == 1 && leaf_sprites.len() == 0
		{
			let entry = self.dynamic_model_to_dynamic_meshes_index[leaf_dynamic_models[0] as usize];
			for visible_mesh_index in entry.first_visible_mesh .. entry.first_visible_mesh + entry.num_visible_meshes
			{
				self.draw_mesh(
					rasterizer,
					&bounds,
					used_leaf_clip_planes,
					&frame_info.model_entities,
					&self.visible_dynamic_meshes_list[visible_mesh_index as usize],
				);
			}
			return;
		}
		if leaf_submodels.len() == 0 && leaf_dynamic_models.len() == 0 && leaf_sprites.len() == 1
		{
			self.draw_sprite(
				rasterizer,
				&clip_planes,
				used_leaf_clip_planes,
				&frame_info.sprites,
				leaf_sprites[0],
			);
			return;
		}

		// Multiple models. Sort them.

		// TODO - use uninitialized memory and increase this value.
		const MAX_SUBMODELS_IN_LEAF: usize = 12;
		let mut models_for_sorting = [draw_ordering::BBoxForDrawOrdering::default(); MAX_SUBMODELS_IN_LEAF];

		const DYNAMIC_MESH_INDEX_ADD: u32 = 65536;
		const SPRITE_INDEX_ADD: u32 = DYNAMIC_MESH_INDEX_ADD + 65536;

		for (&model_index, model_for_sorting) in leaf_submodels.iter().zip(models_for_sorting.iter_mut())
		{
			if let Some(submodel_matrices) = &self.submodels_info[model_index as usize].matrices
			{
				*model_for_sorting = (
					model_index,
					draw_ordering::project_bbox(
						&self.inline_models_index.get_model_bbox_for_ordering(model_index),
						&submodel_matrices.camera_matrices,
					),
				);
			}
		}
		let mut num_models = std::cmp::min(leaf_submodels.len(), MAX_SUBMODELS_IN_LEAF);

		for dynamic_model_index in leaf_dynamic_models
		{
			if num_models == MAX_SUBMODELS_IN_LEAF
			{
				break;
			}

			let model = &frame_info.model_entities[*dynamic_model_index as usize];
			let bbox = if let Some(bb) = model.ordering_custom_bbox
			{
				bb
			}
			else
			{
				get_current_triangle_model_bbox(&model.model, &model.animation)
			};

			let entry = self.dynamic_model_to_dynamic_meshes_index[*dynamic_model_index as usize];
			for visible_mesh_index in entry.first_visible_mesh .. entry.first_visible_mesh + entry.num_visible_meshes
			{
				if num_models == MAX_SUBMODELS_IN_LEAF
				{
					break;
				}

				let mesh = &self.visible_dynamic_meshes_list[visible_mesh_index as usize];
				models_for_sorting[num_models] = (
					visible_mesh_index + DYNAMIC_MESH_INDEX_ADD,
					draw_ordering::project_bbox(&bbox, &mesh.camera_matrices),
				);
				num_models += 1;
			}
		}

		for sprite_index in leaf_sprites
		{
			if num_models == MAX_SUBMODELS_IN_LEAF
			{
				break;
			}
			let sprite = &frame_info.sprites[*sprite_index as usize];
			let extend_vec = Vec3f::new(sprite.radius, sprite.radius, sprite.radius);
			let bbox = BBox::from_min_max(sprite.position - extend_vec, sprite.position + extend_vec);

			models_for_sorting[num_models] = (
				sprite_index + SPRITE_INDEX_ADD,
				draw_ordering::project_bbox(&bbox, &frame_info.camera_matrices),
			);
			num_models += 1;
		}

		draw_ordering::order_bboxes(&mut models_for_sorting[.. num_models]);

		// Draw dynamic models and submodels, located in this leaf, after leaf polygons.
		for (submodel_index, _bbox) in &models_for_sorting[.. num_models]
		{
			if *submodel_index >= SPRITE_INDEX_ADD
			{
				self.draw_sprite(
					rasterizer,
					&clip_planes,
					used_leaf_clip_planes,
					&frame_info.sprites,
					*submodel_index - SPRITE_INDEX_ADD,
				);
			}
			else if *submodel_index >= DYNAMIC_MESH_INDEX_ADD
			{
				let visible_mesh_index = *submodel_index - DYNAMIC_MESH_INDEX_ADD;
				self.draw_mesh(
					rasterizer,
					bounds,
					used_leaf_clip_planes,
					&frame_info.model_entities,
					&self.visible_dynamic_meshes_list[visible_mesh_index as usize],
				);
			}
			else
			{
				self.draw_submodel_in_leaf(
					rasterizer,
					frame_info,
					&clip_planes,
					used_leaf_clip_planes,
					leaf_decals,
					*submodel_index,
				);
			}
		}
	}

	fn draw_polygon_decals<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		clip_planes: &ClippingPolygonPlanes,
		polygon_index: u32,
		decals: &[Decal],
		current_decals: &[DynamicObjectId],
	)
	{
		let polygon_data = &self.polygons_data[polygon_index as usize];
		if polygon_data.visible_frame != self.current_frame
		{
			return;
		}

		let polygon = &self.map.polygons[polygon_index as usize];

		if !self.materials_processor.get_material(polygon.texture).decals
		{
			return;
		}

		const CUBE_SIDES: [[f32; 3]; 6] = [
			[-1.0, 0.0, 0.0],
			[1.0, 0.0, 0.0],
			[0.0, -1.0, 0.0],
			[0.0, 1.0, 0.0],
			[0.0, 0.0, -1.0],
			[0.0, 0.0, 1.0],
		];

		// TODO - maybe use different basis?
		const DECAL_TEXTURE_BASIS: [[f32; 4]; 2] = [[0.0, -0.5, 0.0, 0.5], [0.0, 0.0, -0.5, 0.5]];
		const MAX_ANGLE_COS: f32 = 0.1;

		// TODO - use uninitialized memory.
		let mut vertices_clipped0 = unsafe { std::mem::zeroed::<[Vec3f; MAX_VERTICES]>() };
		let mut vertices_clipped1 = unsafe { std::mem::zeroed::<[Vec3f; MAX_VERTICES]>() };
		let mut vertices_projected = unsafe { std::mem::zeroed::<[Vec2f; MAX_VERTICES]>() };

		'decals_loop: for &decal_index in current_decals
		{
			let decal = &decals[decal_index as usize];
			let decal_info = &self.decals_info[decal_index as usize];

			// Both vectors are normalized, so, dot product is just cosine of angle.
			let decal_angle_cos = polygon_data
				.basis_vecs
				.normal
				.dot(decal.rotation.rotate_vector(-Vec3f::unit_x()));
			// Do not apply decals to back faces and faces with extreme slope.
			if decal_angle_cos < MAX_ANGLE_COS
			{
				continue;
			}

			let decal_planes_matrix = decal_info.camera_planes_matrix;

			// Use polygon itself for further clipping.
			let src_vertices = &self.vertices_transformed
				[polygon.first_vertex as usize .. ((polygon.first_vertex + polygon.num_vertices) as usize)];

			let mut vc_src = &mut vertices_clipped0;
			let mut vc_dst = &mut vertices_clipped1;

			for (dst, src) in vc_src.iter_mut().zip(src_vertices.iter())
			{
				*dst = *src;
			}
			let mut num_vertices = vc_src.len().min(src_vertices.len());

			// Clip polygon by all planes of transformed decal cube.
			for cube_side_normal in CUBE_SIDES
			{
				let plane_vec = decal_planes_matrix * Vec3f::from(cube_side_normal).extend(1.0);
				num_vertices = clip_3d_polygon_by_plane(
					&vc_src[.. num_vertices],
					&Plane {
						vec: plane_vec.truncate(),
						dist: -plane_vec.w,
					},
					vc_dst,
				);
				if num_vertices < 3
				{
					continue 'decals_loop;
				}
				std::mem::swap(&mut vc_src, &mut vc_dst);
			}

			num_vertices = project_and_clip_polygon(clip_planes, &vc_src[.. num_vertices], &mut vertices_projected);
			if num_vertices < 3
			{
				continue;
			}

			// Calculate texture coordinates equation.
			let texture = &decal.texture[0];

			let tc_basis_transformed = [
				decal_planes_matrix * (Vec4f::from(DECAL_TEXTURE_BASIS[0]) * (texture.size[0] as f32)),
				decal_planes_matrix * (Vec4f::from(DECAL_TEXTURE_BASIS[1]) * (texture.size[1] as f32)),
			];
			let depth_equation = &polygon_data.depth_equation;
			let tc_equation = TexCoordEquation::from_depth_equation_and_transformed_tex_coord_equations(
				depth_equation,
				&tc_basis_transformed,
			);

			let mip = calculate_mip(
				&vertices_projected[.. num_vertices],
				&depth_equation,
				&tc_equation,
				self.mip_bias,
			);
			let mip_texture = &decal.texture[mip as usize];
			let tc_equation_scaled = tc_equation * (1.0 / ((1 << mip) as f32));

			// Use projected polygon texture coordinates equation in order to get lightmap coordinates for decal points.
			let polygon_lightmap_coord_scale = ((1 << (polygon_data.mip)) as f32) / (lightmap::LIGHTMAP_SCALE as f32);
			let polygon_lightmap_coord_shift = [
				(polygon_data.surface_tc_min[0] as f32) * polygon_lightmap_coord_scale -
					((polygon.tex_coord_min[0] >> lightmap::LIGHTMAP_SCALE_LOG2) as f32),
				(polygon_data.surface_tc_min[1] as f32) * polygon_lightmap_coord_scale -
					((polygon.tex_coord_min[1] >> lightmap::LIGHTMAP_SCALE_LOG2) as f32),
			];
			let polygon_lightmap_eqution = polygon_data.tex_coord_equation * polygon_lightmap_coord_scale;

			// Calculate dynamic light based on normal of current polygon.
			let mut dynamic_light =
				get_light_cube_light(&decal_info.dynamic_light.light_cube, &polygon_data.basis_vecs.normal);
			let dynamic_light_dir_normal_dot = polygon_data
				.basis_vecs
				.normal
				.dot(decal_info.dynamic_light.light_direction_vector_scaled)
				.max(0.0);
			for i in 0 .. 3
			{
				dynamic_light[i] += dynamic_light_dir_normal_dot * decal_info.dynamic_light.directional_light_color[i];
			}

			for t in 0 .. num_vertices - 2
			{
				self.subdivide_and_draw_decal_triangle(
					rasterizer,
					decal,
					polygon,
					depth_equation,
					&tc_equation_scaled,
					&polygon_lightmap_eqution,
					&polygon_lightmap_coord_shift,
					&[
						vertices_projected[0],
						vertices_projected[t + 1],
						vertices_projected[t + 2],
					],
					mip_texture,
					&dynamic_light,
					0,
				);
			}
		} // for decals.
	}

	fn subdivide_and_draw_decal_triangle<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		decal: &Decal,
		polygon: &bsp_map_compact::Polygon,
		depth_equation: &DepthEquation,
		tc_equation: &TexCoordEquation,
		polygon_lightmap_eqution: &TexCoordEquation,
		polygon_lightmap_coord_shift: &[f32; 2],
		points: &[Vec2f; 3],
		texture: &TextureLite,
		dynamic_light: &[f32; 3],
		recursion_depth: usize,
	)
	{
		let point0 = &points[0];
		let mut min_inv_z_point = point0;
		let mut max_inv_z_point = point0;
		let mut min_inv_z = depth_equation.sample_point(point0);
		let mut max_inv_z = min_inv_z;
		for point in &points[1 ..]
		{
			let inv_z = depth_equation.sample_point(point);
			if inv_z < min_inv_z
			{
				min_inv_z = inv_z;
				min_inv_z_point = point;
			}
			if inv_z > max_inv_z
			{
				max_inv_z = inv_z;
				max_inv_z_point = point;
			}
		}

		let max_recursion_depth = 2;

		if recursion_depth >= max_recursion_depth ||
			affine_texture_coordinates_interpolation_may_be_used(
				depth_equation,
				tc_equation,
				min_inv_z_point,
				max_inv_z_point,
			)
		{
			// Clamp texture coordinates to proper range and add extra epsilon to avoid ugly texture coordinates clamping in rasterizer.
			let tc_eps = 1.0 / 32.0;
			let min_tc = [tc_eps, tc_eps];
			let max_tc = [texture.size[0] as f32 - tc_eps, texture.size[1] as f32 - tc_eps];

			// TODO - use uninitialized memory.
			let mut vertices_fixed = unsafe { std::mem::zeroed::<[TrianglePointProjected; 3]>() };
			for (src, dst) in points.iter().zip(vertices_fixed.iter_mut())
			{
				let z = 1.0 / depth_equation.sample_point(src);

				let mut light = decal.light_add;
				if decal.lightmap_light_scale > 0.0
				{
					let lightmap_coord =
						Vec2f::new(
							z * (polygon_lightmap_eqution.d_tc_dx[0] * src.x +
								polygon_lightmap_eqution.d_tc_dy[0] * src.y +
								polygon_lightmap_eqution.k[0]) + polygon_lightmap_coord_shift[0],
							z * (polygon_lightmap_eqution.d_tc_dx[1] * src.x +
								polygon_lightmap_eqution.d_tc_dy[1] * src.y +
								polygon_lightmap_eqution.k[1]) + polygon_lightmap_coord_shift[1],
						);

					let lightmap_light = get_polygon_lightmap_light(&self.map, polygon, &lightmap_coord);
					for i in 0 .. 3
					{
						light[i] += lightmap_light[i] * decal.lightmap_light_scale;
						light[i] += dynamic_light[i];
					}
				}

				*dst = TrianglePointProjected {
					x: f32_to_fixed16(src.x),
					y: f32_to_fixed16(src.y),
					tc: [
						f32_to_fixed16(
							(z * (tc_equation.d_tc_dx[0] * src.x + tc_equation.d_tc_dy[0] * src.y + tc_equation.k[0]))
								.max(min_tc[0])
								.min(max_tc[0]),
						),
						f32_to_fixed16(
							(z * (tc_equation.d_tc_dx[1] * src.x + tc_equation.d_tc_dy[1] * src.y + tc_equation.k[1]))
								.max(min_tc[1])
								.min(max_tc[1]),
						),
					],
					light: [
						f32_to_fixed16(light[0]),
						f32_to_fixed16(light[1]),
						f32_to_fixed16(light[2]),
					],
				};
			}

			// Perform rasteriation of result triangles.
			let texture_info = TextureInfo {
				size: [texture.size[0] as i32, texture.size[1] as i32],
			};

			let texture_data = &texture.pixels;
			let blending_mode = decal.blending_mode;

			rasterizer.fill_triangle(&vertices_fixed, &texture_info, texture_data, blending_mode);
		}
		else
		{
			let center0 = (points[0] + points[1]) / 2.0;
			let center1 = (points[1] + points[2]) / 2.0;
			let center2 = (points[2] + points[0]) / 2.0;
			for triangle in &[
				[points[0], center0, center2],
				[points[1], center1, center0],
				[points[2], center2, center1],
				[center0, center1, center2],
			]
			{
				self.subdivide_and_draw_decal_triangle(
					rasterizer,
					decal,
					polygon,
					depth_equation,
					&tc_equation,
					&polygon_lightmap_eqution,
					&polygon_lightmap_coord_shift,
					triangle,
					texture,
					dynamic_light,
					recursion_depth + 1,
				);
			}
		}
	}

	fn draw_submodel_in_leaf<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		frame_info: &FrameInfo,
		clip_planes: &ClippingPolygonPlanes,
		leaf_clip_planes: &[Plane],
		leaf_decals: &[DynamicObjectId],
		submodel_index: u32,
	)
	{
		if let Some(submodel_matrices) = &self.submodels_info[submodel_index as usize].matrices
		{
			let submodel = &self.map.submodels[submodel_index as usize];
			self.draw_submodel_bsp_node_r(
				rasterizer,
				frame_info,
				&submodel_matrices.camera_matrices.planes_matrix,
				clip_planes,
				leaf_clip_planes,
				leaf_decals,
				submodel.root_node,
			);
		}
	}

	fn draw_submodel_bsp_node_r<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		frame_info: &FrameInfo,
		submodel_planes_matrix: &Mat4f,
		clip_planes: &ClippingPolygonPlanes,
		leaf_clip_planes: &[Plane],
		leaf_decals: &[DynamicObjectId],
		node_index: u32,
	)
	{
		let &node = &self.map.submodels_bsp_nodes[node_index as usize];

		let plane_transformed = submodel_planes_matrix * node.plane.vec.extend(-node.plane.dist);
		let mut mask = if plane_transformed.w >= 0.0 { 1 } else { 0 };
		if self.config.invert_polygons_order
		{
			mask ^= 1;
		}

		let c_b = node.children[mask as usize];
		let c_f = node.children[(mask ^ 1) as usize];

		let num_nodes = self.map.submodels_bsp_nodes.len() as u32;
		if c_b < num_nodes
		{
			self.draw_submodel_bsp_node_r(
				rasterizer,
				frame_info,
				submodel_planes_matrix,
				clip_planes,
				leaf_clip_planes,
				leaf_decals,
				c_b,
			);
		}

		for polygon_index in node.first_polygon .. (node.first_polygon + node.num_polygons)
		{
			self.draw_submodel_polygon(rasterizer, &clip_planes, leaf_clip_planes, polygon_index);

			if !leaf_decals.is_empty()
			{
				self.draw_polygon_decals(rasterizer, clip_planes, polygon_index, &frame_info.decals, leaf_decals);
			}
		}

		if c_f < num_nodes
		{
			self.draw_submodel_bsp_node_r(
				rasterizer,
				frame_info,
				submodel_planes_matrix,
				clip_planes,
				leaf_clip_planes,
				leaf_decals,
				c_f,
			);
		}
	}

	fn draw_submodel_polygon<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		clip_planes: &ClippingPolygonPlanes,
		leaf_clip_planes: &[Plane],
		polygon_index: u32,
	)
	{
		let polygon = &self.map.polygons[polygon_index as usize];
		let polygon_data = &self.polygons_data[polygon_index as usize];
		if polygon_data.visible_frame != self.current_frame
		{
			return;
		}

		// HACK! Shift polygon vertices a bit away from camera to fix buggy polygon clipping,
		// when polygon lies exactly on clip plane.
		// Such hack doesn't solve problems completely, but it resolves most actual cases.
		let vertex_pos_shift_eps = 1.0 / 4.0;

		let mut vertices_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory.
		let mut vertex_count = std::cmp::min(polygon.num_vertices as usize, MAX_VERTICES);

		for (in_vertex, out_vertex) in self.vertices_transformed
			[(polygon.first_vertex as usize) .. (polygon.first_vertex as usize) + vertex_count]
			.iter()
			.zip(vertices_clipped[.. vertex_count].iter_mut())
		{
			*out_vertex = Vec3f::new(in_vertex.x, in_vertex.y, in_vertex.z + vertex_pos_shift_eps);
		}

		let mut vertices_temp = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory.

		for clip_plane in leaf_clip_planes
		{
			vertex_count =
				clip_3d_polygon_by_plane(&vertices_clipped[.. vertex_count], clip_plane, &mut vertices_temp[..]);
			if vertex_count < 3
			{
				return;
			}
			vertices_clipped[.. vertex_count].copy_from_slice(&vertices_temp[.. vertex_count]);
		}

		// Shift clipped vertices back.
		for v in &mut vertices_clipped[.. vertex_count]
		{
			v.z -= vertex_pos_shift_eps;
		}

		draw_polygon(
			rasterizer,
			&clip_planes,
			&vertices_clipped[.. vertex_count],
			&polygon_data.depth_equation,
			&polygon_data.tex_coord_equation,
			&polygon_data.surface_size,
			self.get_polygon_surface_data(polygon_data),
			self.materials_processor.get_material(polygon.texture).blending_mode,
		);
	}

	fn draw_sprite<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		clip_planes: &ClippingPolygonPlanes,
		leaf_clip_planes: &[Plane],
		sprites: &[Sprite],
		sprite_index: u32,
	)
	{
		let sprite = &sprites[sprite_index as usize];
		let sprite_info = &self.sprites_info[sprite_index as usize];

		// TODO - select proper mip level.
		let texture_mip = &sprite.texture[0];
		let texture_size = [texture_mip.size[0] as f32, texture_mip.size[1] as f32];

		let mut vertices_clipped0 = unsafe { std::mem::zeroed::<[ModelVertex3d; MAX_VERTICES]>() };
		let mut vertices_clipped1 = unsafe { std::mem::zeroed::<[ModelVertex3d; MAX_VERTICES]>() };
		let mut vc_src = &mut vertices_clipped0;
		let mut vc_dst = &mut vertices_clipped1;

		let tc_reduce = 1.0 / 16.0;
		vc_src[0] = ModelVertex3d {
			light: sprite_info.light,
			pos: sprite_info.vertices_projected[0],
			tc: Vec2f::new(texture_size[0] - tc_reduce, texture_size[1] - tc_reduce),
		};
		vc_src[1] = ModelVertex3d {
			light: sprite_info.light,
			pos: sprite_info.vertices_projected[1],
			tc: Vec2f::new(texture_size[0] - tc_reduce, tc_reduce),
		};
		vc_src[2] = ModelVertex3d {
			light: sprite_info.light,
			pos: sprite_info.vertices_projected[2],
			tc: Vec2f::new(tc_reduce, tc_reduce),
		};
		vc_src[3] = ModelVertex3d {
			light: sprite_info.light,
			pos: sprite_info.vertices_projected[3],
			tc: Vec2f::new(tc_reduce, texture_size[1] - tc_reduce),
		};

		let z_near_plane = Plane {
			vec: Vec3f::unit_z(),
			dist: Z_NEAR,
		};

		let mut num_vertices = 4;
		for clip_plane in [z_near_plane].iter().chain(leaf_clip_planes.iter())
		{
			num_vertices = clip_3d_model_polygon_by_plane(&vc_src[.. num_vertices], clip_plane, vc_dst);
			if num_vertices < 3
			{
				return;
			}
			std::mem::swap(&mut vc_src, &mut vc_dst);
		}

		let mut vertices_projected0 = unsafe { std::mem::zeroed::<[ModelVertex2d; MAX_VERTICES]>() };
		let mut vertices_projected1 = unsafe { std::mem::zeroed::<[ModelVertex2d; MAX_VERTICES]>() };
		let mut vp_src = &mut vertices_projected0;
		let mut vp_dst = &mut vertices_projected1;

		for (src, dst) in vc_src[.. num_vertices].iter().zip(vp_src.iter_mut())
		{
			*dst = ModelVertex2d {
				pos: src.pos.truncate() / src.pos.z,
				tc: src.tc,
				light: src.light,
			};
		}
		for clip_plane in clip_planes
		{
			num_vertices = clip_2d_model_polygon(&vp_src[.. num_vertices], clip_plane, vp_dst);
			if num_vertices < 3
			{
				return;
			}
			std::mem::swap(&mut vp_src, &mut vp_dst);
		}

		let mut vertices_fixed = unsafe { std::mem::zeroed::<[TrianglePointProjected; MAX_VERTICES]>() };
		for (src, dst) in vp_src[.. num_vertices].iter().zip(vertices_fixed.iter_mut())
		{
			*dst = TrianglePointProjected {
				x: f32_to_fixed16(src.pos.x),
				y: f32_to_fixed16(src.pos.y),
				tc: [f32_to_fixed16(src.tc.x), f32_to_fixed16(src.tc.y)],
				light: [
					f32_to_fixed16(src.light[0]),
					f32_to_fixed16(src.light[1]),
					f32_to_fixed16(src.light[2]),
				],
			};
		}

		let texture_info = TextureInfo {
			size: [texture_mip.size[0] as i32, texture_mip.size[1] as i32],
		};
		let texture_data = &texture_mip.pixels;
		let blending_mode = sprite.blending_mode;

		for t in 0 .. num_vertices - 2
		{
			rasterizer.fill_triangle(
				&[vertices_fixed[0], vertices_fixed[t + 1], vertices_fixed[t + 2]],
				&texture_info,
				texture_data,
				blending_mode,
			);
		} // for subtriangles
	}

	fn draw_mesh<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		clipping_polygon: &ClippingPolygon,
		leaf_clip_planes: &[Plane],
		models: &[ModelEntity],
		visible_dynamic_mesh: &VisibleDynamicMeshInfo,
	)
	{
		{
			let mut mesh_clipping_polygon = visible_dynamic_mesh.clipping_polygon;
			mesh_clipping_polygon.intersect(clipping_polygon);
			if mesh_clipping_polygon.is_empty_or_invalid()
			{
				// This mesh is not visble in this leaf or for this screeen rect.
				return;
			}
		}

		let model = &models[visible_dynamic_mesh.entity_index as usize];

		// TODO - maybe specialize inner loop for each blending mode?
		let blending_mode = model.blending_mode;

		// Find clip planes that affect this model.
		// TODO - use uninitialized memory.
		let mut clip_planes_3d = [Plane {
			vec: Vec3f::zero(),
			dist: 0.0,
		}; MAX_LEAF_CLIP_PLANES];
		let mut num_clip_planes_3d = 0;

		let near_z_plane = Plane {
			vec: Vec3f::unit_z(),
			dist: Z_NEAR,
		};
		for clip_plane in [near_z_plane].iter().chain(leaf_clip_planes.iter())
		{
			let mut vertices_front = 0;
			for v in visible_dynamic_mesh.bbox_vertices_transformed
			{
				if clip_plane.vec.dot(v) >= clip_plane.dist
				{
					vertices_front += 1;
				}
			}

			if vertices_front == visible_dynamic_mesh.bbox_vertices_transformed.len()
			{
				// This clip plane is useless.
			}
			else if vertices_front == 0
			{
				// Model is fully clipped.
				return;
			}
			else
			{
				clip_planes_3d[num_clip_planes_3d] = *clip_plane;
				num_clip_planes_3d += 1;
			}
		}

		// Find 2d clip planes that affect this model.
		// Use only box clip planes to reduce number of checks.
		// TODO - use uninitialized memory.
		let mut clip_planes_2d = [Vec3f::zero(); 4];
		let mut num_clip_planes_2d = 0;
		for (mesh_plane, cur_plane) in visible_dynamic_mesh
			.clipping_polygon
			.get_box_clip_planes()
			.iter()
			.zip(clipping_polygon.get_box_clip_planes().iter())
		{
			if cur_plane.z > mesh_plane.z
			{
				clip_planes_2d[num_clip_planes_2d] = *cur_plane;
				num_clip_planes_2d += 1;
			}
		}

		let texture = &model.texture[visible_dynamic_mesh.mip as usize];

		// TODO - use individual texture for each mesh.
		let texture_info = TextureInfo {
			size: [texture.size[0] as i32, texture.size[1] as i32],
		};

		let texture_data = &texture.pixels;

		let vertices_combined = &self.dynamic_meshes_vertices[visible_dynamic_mesh.vertices_offset ..];
		let triangles = &self.dynamic_meshes_triangles[visible_dynamic_mesh.triangles_offset ..
			visible_dynamic_mesh.triangles_offset + visible_dynamic_mesh.num_visible_triangles];

		if num_clip_planes_3d == 0 && num_clip_planes_2d == 0
		{
			// Special case - perform no clipping at all, just draw source triangles.
			for triangle in triangles
			{
				let vertices_projected = triangle.map(|index| {
					let v = triangle_vertex_debug_checked_fetch(vertices_combined, index);
					let point = v.pos.truncate() / v.pos.z;
					TrianglePointProjected {
						x: f32_to_fixed16(point.x),
						y: f32_to_fixed16(point.y),
						tc: [f32_to_fixed16(v.tc.x), f32_to_fixed16(v.tc.y)],
						light: [
							f32_to_fixed16(v.light[0]),
							f32_to_fixed16(v.light[1]),
							f32_to_fixed16(v.light[2]),
						],
					}
				});

				rasterizer.fill_triangle(&vertices_projected, &texture_info, texture_data, blending_mode);
			}
		}
		else
		{
			let mut vertices_clipped0 = unsafe { std::mem::zeroed::<[ModelVertex3d; MAX_VERTICES]>() };
			let mut vertices_clipped1 = unsafe { std::mem::zeroed::<[ModelVertex3d; MAX_VERTICES]>() };
			let mut vertices_projected0 = unsafe { std::mem::zeroed::<[ModelVertex2d; MAX_VERTICES]>() };
			let mut vertices_projected1 = unsafe { std::mem::zeroed::<[ModelVertex2d; MAX_VERTICES]>() };
			let mut vertices_fixed = unsafe { std::mem::zeroed::<[TrianglePointProjected; MAX_VERTICES]>() };

			'triangles_loop: for triangle in triangles
			{
				let mut vc_src = &mut vertices_clipped0;
				let mut vc_dst = &mut vertices_clipped1;

				for (&index, dst_vertex) in triangle.iter().zip(vc_src.iter_mut())
				{
					*dst_vertex = triangle_vertex_debug_checked_fetch(vertices_combined, index);
				}

				let mut num_vertices = 3;
				for clip_plane in &clip_planes_3d[.. num_clip_planes_3d]
				{
					num_vertices = clip_3d_model_polygon_by_plane(&vc_src[.. num_vertices], clip_plane, vc_dst);
					if num_vertices < 3
					{
						continue 'triangles_loop;
					}
					std::mem::swap(&mut vc_src, &mut vc_dst);
				}

				let mut vp_src = &mut vertices_projected0;
				let mut vp_dst = &mut vertices_projected1;

				for (src, dst) in vc_src[.. num_vertices].iter().zip(vp_src.iter_mut())
				{
					*dst = ModelVertex2d {
						pos: src.pos.truncate() / src.pos.z,
						tc: src.tc,
						light: src.light,
					};
				}

				for clip_plane in &clip_planes_2d[.. num_clip_planes_2d]
				{
					num_vertices = clip_2d_model_polygon(&vp_src[.. num_vertices], clip_plane, vp_dst);
					if num_vertices < 3
					{
						continue 'triangles_loop;
					}
					std::mem::swap(&mut vp_src, &mut vp_dst);
				}

				for (src, dst) in vp_src[.. num_vertices].iter().zip(vertices_fixed.iter_mut())
				{
					*dst = TrianglePointProjected {
						x: f32_to_fixed16(src.pos.x),
						y: f32_to_fixed16(src.pos.y),
						tc: [f32_to_fixed16(src.tc.x), f32_to_fixed16(src.tc.y)],
						light: [
							f32_to_fixed16(src.light[0]),
							f32_to_fixed16(src.light[1]),
							f32_to_fixed16(src.light[2]),
						],
					};
				}

				for t in 0 .. num_vertices - 2
				{
					rasterizer.fill_triangle(
						&[vertices_fixed[0], vertices_fixed[t + 1], vertices_fixed[t + 2]],
						&texture_info,
						texture_data,
						blending_mode,
					);
				} // for subtriangles
			} // For triangles
		}
	}

	fn get_polygon_surface_data<ColorT>(&self, polygon_data: &DrawPolygonData) -> &[ColorT]
	{
		let pixels_casted = unsafe { self.surfaces_pixels.align_to::<ColorT>().1 };
		&pixels_casted[polygon_data.surface_pixels_offset ..
			polygon_data.surface_pixels_offset +
				((polygon_data.surface_size[0] * polygon_data.surface_size[1]) as usize)]
	}

	fn update_mip_bias(&mut self)
	{
		if self.config.dynamic_mip_bias
		{
			let target_num_pixels = 1024 * 256;
			let target_mip_bias = ((self.num_visible_surfaces_pixels as f32) / (target_num_pixels as f32))
				.log2()
				.max(0.0)
				.min(3.0);
			if (self.mip_bias - target_mip_bias).abs() >= 1.0 / 16.0
			{
				self.mip_bias = (target_mip_bias + self.mip_bias * 15.0) / 16.0;
			}
		}
		else
		{
			self.mip_bias = self.config.textures_mip_bias;
		}
	}

	fn synchronize_config(&mut self)
	{
		self.config = RendererConfig::from_app_config(&self.app_config);

		// Make sure that config values are reasonable.
		let mut config_is_dirty = false;
		if self.config.textures_mip_bias < -1.0
		{
			self.config.textures_mip_bias = -1.0;
			config_is_dirty = true;
		}
		if self.config.textures_mip_bias > 2.0
		{
			self.config.textures_mip_bias = 2.0;
			config_is_dirty = true;
		}

		if self.config.shadows_quality < -1.0
		{
			self.config.shadows_quality = -1.0;
			config_is_dirty = true;
		}
		if self.config.shadows_quality > 1.0
		{
			self.config.shadows_quality = 1.0;
			config_is_dirty = true;
		}

		if config_is_dirty
		{
			self.config.update_app_config(&self.app_config);
		}
	}
}

fn get_polygon_lightmap_light(
	map: &bsp_map_compact::BSPMap,
	polygon: &bsp_map_compact::Polygon,
	lightmap_coord: &Vec2f,
) -> [f32; 3]
{
	if polygon.lightmap_data_offset == 0
	{
		// No lightmap.
		return [0.0; 3];
	}

	// Use simple (non-diretional) lightmap.
	let lightmaps_data = &map.lightmaps_data;
	if lightmaps_data.is_empty()
	{
		return [0.0; 3];
	}

	let polygon_lightmap_data = &lightmaps_data[polygon.lightmap_data_offset as usize ..];

	let lightmap_coord_int = [lightmap_coord.x.floor() as i32, lightmap_coord.y.floor() as i32];

	let lightmap_size = lightmap::get_polygon_lightmap_size(polygon);

	// Perform fetch with linear interpolation.
	let mut total_light = [0.0; 3];
	let mut total_factor = 0.0;
	for dy in 0 ..= 1
	{
		let y = lightmap_coord_int[1] + dy;
		if y < 0 || y >= lightmap_size[1] as i32
		{
			continue;
		}

		let factor_y = 1.0 - (lightmap_coord.y - (y as f32)).abs();
		for dx in 0 ..= 1
		{
			let x = lightmap_coord_int[0] + dx;
			if x < 0 || x >= lightmap_size[0] as i32
			{
				continue;
			}

			let factor_x = 1.0 - (lightmap_coord.x - (x as f32)).abs();

			let lightmap_light = polygon_lightmap_data[(x + y * (lightmap_size[0] as i32)) as usize];
			let cur_sample_factor = factor_x * factor_y;
			for i in 0 .. 3
			{
				total_light[i] += cur_sample_factor * lightmap_light[i];
			}
			total_factor += cur_sample_factor;
		}
	}

	if total_factor < 1.0
	{
		// Perform normalization in case if same sample points were rejected.
		let inv_total_factor = 1.0 / total_factor;
		for i in 0 .. 3
		{
			total_light[i] *= inv_total_factor;
		}
	}

	total_light
}

fn polygon_is_affected_by_light(
	polygon: &bsp_map_compact::Polygon,
	basis_vecs: &PolygonBasisVecs,
	light: &DynamicLightWithShadow,
) -> bool
{
	// TODO - check mathemathics here.

	let vec_from_light_position_to_tc_start_point = light.position - basis_vecs.start;
	let signed_dinstance_to_polygon_plane = vec_from_light_position_to_tc_start_point.dot(basis_vecs.normal);
	if signed_dinstance_to_polygon_plane <= 0.0
	{
		// This light is behind polygon plane.
		return false;
	}

	let square_radius_at_polygon_plane =
		light.radius * light.radius - signed_dinstance_to_polygon_plane * signed_dinstance_to_polygon_plane;
	if square_radius_at_polygon_plane <= 0.0
	{
		// This light is too far from polygon plane.
		return false;
	}

	if let ShadowMap::Projector(projector_shadow_map) = &light.shadow_map
	{
		// Check intersection of light pyramid planes with (approximate) vertices of polygon.

		let u0 = basis_vecs.u * (polygon.tex_coord_min[0] as f32);
		let u1 = basis_vecs.u * (polygon.tex_coord_max[0] as f32);
		let v0 = basis_vecs.v * (polygon.tex_coord_min[1] as f32);
		let v1 = basis_vecs.v * (polygon.tex_coord_max[1] as f32);
		let vertices = [
			basis_vecs.start + u0 + v0,
			basis_vecs.start + u0 + v1,
			basis_vecs.start + u1 + v0,
			basis_vecs.start + u1 + v1,
		];

		let plane_vecs = [
			projector_shadow_map.basis_z,
			projector_shadow_map.basis_z - projector_shadow_map.basis_x,
			projector_shadow_map.basis_z + projector_shadow_map.basis_x,
			projector_shadow_map.basis_z - projector_shadow_map.basis_y,
			projector_shadow_map.basis_z + projector_shadow_map.basis_y,
		];

		for plane_vec in &plane_vecs
		{
			let plane = Plane {
				vec: *plane_vec,
				dist: plane_vec.dot(light.position),
			};
			let mut vertices_front = 0;
			for vertex in vertices
			{
				if plane.vec.dot(vertex) >= plane.dist
				{
					vertices_front += 1;
				}
			}
			if vertices_front >= vertices.len()
			{
				// Clipped by one of planes.
				return false;
			}
		}

		true
	}
	else
	{
		// Calculate texture coordinates at point of projection of light position to polygon plane.
		let mat = if let Some(m) = Mat3f::from_cols(basis_vecs.u, basis_vecs.v, basis_vecs.normal).invert()
		{
			m.transpose()
		}
		else
		{
			return false;
		};
		let tc_at_projected_light_position = [
			vec_from_light_position_to_tc_start_point.dot(mat.x),
			vec_from_light_position_to_tc_start_point.dot(mat.y),
		];

		// Check min/max texture coordinates of projected circle agains polygon min/max texture coordinates.
		// This is inexact check (not proper polygon check) but it gives good enough result.
		let radius_at_polygon_plane = square_radius_at_polygon_plane.sqrt();
		let u_radius = radius_at_polygon_plane * inv_sqrt_fast(basis_vecs.u.magnitude2());
		let v_radius = radius_at_polygon_plane * inv_sqrt_fast(basis_vecs.v.magnitude2());

		if tc_at_projected_light_position[0] + u_radius < (polygon.tex_coord_min[0] as f32) ||
			tc_at_projected_light_position[1] + v_radius < (polygon.tex_coord_min[1] as f32) ||
			tc_at_projected_light_position[0] - u_radius > (polygon.tex_coord_max[0] as f32) ||
			tc_at_projected_light_position[1] - v_radius > (polygon.tex_coord_max[1] as f32)
		{
			// Light porjection circle is outside polygon borders.
			false
		}
		else
		{
			// Light affects this polygon.
			// TODO - maybe process corner cases here (literally, check intersection with corners)?
			true
		}
	}
}

fn create_dynamic_light_with_shadow<'a>(
	light: &DynamicLight,
	light_info: &DynamicLightInfo,
	shadow_maps_data: &'a [ShadowMapElement],
) -> DynamicLightWithShadow<'a>
{
	DynamicLightWithShadow {
		position: light.position,
		radius: light.radius,
		inv_square_radius: 1.0 / (light.radius * light.radius),
		color: light.color,
		shadow_map: match &light.shadow_type
		{
			DynamicLightShadowType::None => ShadowMap::None,
			DynamicLightShadowType::Cubemap => ShadowMap::Cube(
				if light_info.visible
				{
					create_dynamic_light_cube_shadow_map(light_info, shadow_maps_data)
				}
				else
				{
					create_dynamic_light_cube_shadow_map_dummy()
				},
			),
			DynamicLightShadowType::Projector { rotation, fov } => ShadowMap::Projector(
				if light_info.visible
				{
					create_dynamic_light_projector_shadow_map(rotation, *fov, light_info, shadow_maps_data)
				}
				else
				{
					create_dynamic_light_projector_shadow_map_dummy()
				},
			),
		},
	}
}

fn create_dynamic_light_cube_shadow_map<'a>(
	light_info: &DynamicLightInfo,
	shadow_maps_data: &'a [ShadowMapElement],
) -> CubeShadowMap<'a>
{
	let side_data_size = (light_info.shadow_map_size * light_info.shadow_map_size) as usize;
	let shadow_map_data =
		&shadow_maps_data[light_info.shadow_map_data_offset .. light_info.shadow_map_data_offset + side_data_size * 6];

	CubeShadowMap {
		size: light_info.shadow_map_size,
		sides: [
			&shadow_map_data[0 * side_data_size .. 1 * side_data_size],
			&shadow_map_data[1 * side_data_size .. 2 * side_data_size],
			&shadow_map_data[2 * side_data_size .. 3 * side_data_size],
			&shadow_map_data[3 * side_data_size .. 4 * side_data_size],
			&shadow_map_data[4 * side_data_size .. 5 * side_data_size],
			&shadow_map_data[5 * side_data_size .. 6 * side_data_size],
		],
	}
}

fn create_dynamic_light_projector_shadow_map<'a>(
	rotation: &QuaternionF,
	fov: RadiansF,
	light_info: &DynamicLightInfo,
	shadow_maps_data: &'a [ShadowMapElement],
) -> ProjectorShadowMap<'a>
{
	let data_size = (light_info.shadow_map_size * light_info.shadow_map_size) as usize;
	let shadow_map_data =
		&shadow_maps_data[light_info.shadow_map_data_offset .. light_info.shadow_map_data_offset + data_size];

	let inv_half_fov_tan = 1.0 / (fov * 0.5).tan();

	ProjectorShadowMap {
		size: light_info.shadow_map_size,
		data: shadow_map_data,
		basis_x: rotation.rotate_vector(Vec3f::unit_y()) * inv_half_fov_tan,
		basis_y: rotation.rotate_vector(Vec3f::unit_z()) * inv_half_fov_tan,
		basis_z: rotation.rotate_vector(-Vec3f::unit_x()),
	}
}

fn draw_background<ColorT: Copy + Send + Sync>(pixels: &mut [ColorT], color: ColorT)
{
	let num_threads = rayon::current_num_threads();
	if num_threads == 1
	{
		draw_background_impl(pixels, color);
	}
	else
	{
		let num_pixels = pixels.len();
		pixels.par_chunks_mut(num_pixels / num_threads).for_each(|pixels_part| {
			draw_background_impl(pixels_part, color);
		});
	}
}

fn draw_background_impl<ColorT: Copy>(pixels: &mut [ColorT], color: ColorT)
{
	for pixel in pixels
	{
		*pixel = color;
	}
}

fn draw_polygon<'a, ColorT: AbstractColor>(
	rasterizer: &mut Rasterizer<'a, ColorT>,
	clip_planes: &ClippingPolygonPlanes,
	vertices_transformed: &[Vec3f],
	depth_equation: &DepthEquation,
	tex_coord_equation: &TexCoordEquation,
	texture_size: &[u32; 2],
	texture_data: &[ColorT],
	blending_mode: material::BlendingMode,
)
{
	if vertices_transformed.len() < 3
	{
		return;
	}

	let mut vertices_2d = [Vec2f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	let vertex_count = project_and_clip_polygon(clip_planes, vertices_transformed, &mut vertices_2d[..]);
	if vertex_count < 3
	{
		return;
	}

	// Find min/max inv_z to check if we can use affine texture coordinates interpolation.
	// TODO - calculate this during surface preparation?
	let mut min_inv_z = 1e24;
	let mut max_inv_z = -1e24;
	let mut min_x = 1e24;
	let mut max_x = -1e24;
	let mut min_inv_z_point = &vertices_2d[0];
	let mut max_inv_z_point = &vertices_2d[0];
	for point in &vertices_2d[.. vertex_count]
	{
		let inv_z = depth_equation.sample_point(point);
		if inv_z < min_inv_z
		{
			min_inv_z = inv_z;
			min_inv_z_point = point;
		}
		if inv_z > max_inv_z
		{
			max_inv_z = inv_z;
			max_inv_z_point = point;
		}
		if point.x < min_x
		{
			min_x = point.x;
		}
		if point.x > max_x
		{
			max_x = point.x;
		}
	}

	// Perform f32 to Fixed16 conversion.
	let mut vertices_for_rasterizer = [PolygonPointProjected { x: 0, y: 0 }; MAX_VERTICES]; // TODO - use uninitialized memory
	for (vertex_2d, vertex_for_rasterizer) in vertices_2d
		.iter()
		.take(vertex_count)
		.zip(vertices_for_rasterizer.iter_mut())
	{
		// Use unchecked conversion since we know that coords are in fixed16 range.
		*vertex_for_rasterizer = PolygonPointProjected {
			x: unsafe { f32_to_fixed16_unchecked(vertex_2d.x) },
			y: unsafe { f32_to_fixed16_unchecked(vertex_2d.y) },
		};
	}

	// Perform rasterization of fully clipped polygon.
	let texture_info = TextureInfo {
		size: [texture_size[0] as i32, texture_size[1] as i32],
	};

	if affine_texture_coordinates_interpolation_may_be_used(
		depth_equation,
		tex_coord_equation,
		min_inv_z_point,
		max_inv_z_point,
	)
	{
		rasterizer.fill_polygon(
			&vertices_for_rasterizer[0 .. vertex_count],
			&depth_equation,
			&tex_coord_equation,
			&texture_info,
			texture_data,
			TetureCoordinatesInterpolationMode::Affine,
			blending_mode,
		);
	}
	else
	{
		// Scale depth and texture coordinates equation in order to increase precision inside rasterizer.
		// Use only power of 2 scale for this.
		// This is equivalent to moving far polygons closer to camera.
		let z_scale = (-5.0 - max_inv_z.max(1.0 / ((1 << 20) as f32)).log2().ceil()).exp2();

		let depth_equation_scaled = *depth_equation * z_scale;
		let tex_coord_equation_scaled = *tex_coord_equation * z_scale;

		if line_z_corrected_texture_coordinates_interpolation_may_be_used(
			depth_equation,
			tex_coord_equation,
			max_inv_z_point,
			min_x,
			max_x,
		)
		{
			rasterizer.fill_polygon(
				&vertices_for_rasterizer[0 .. vertex_count],
				&depth_equation_scaled,
				&tex_coord_equation_scaled,
				&texture_info,
				texture_data,
				TetureCoordinatesInterpolationMode::LineZCorrection,
				blending_mode,
			);
		}
		else
		{
			rasterizer.fill_polygon(
				&vertices_for_rasterizer[0 .. vertex_count],
				&depth_equation_scaled,
				&tex_coord_equation_scaled,
				&texture_info,
				texture_data,
				TetureCoordinatesInterpolationMode::FullPerspective,
				blending_mode,
			);
		}
	}
}

fn affine_texture_coordinates_interpolation_may_be_used(
	depth_equation: &DepthEquation,
	tex_coord_equation: &TexCoordEquation,
	min_inv_z_point: &Vec2f,
	max_inv_z_point: &Vec2f,
) -> bool
{
	// Projects depth and texture coordinates eqution to edge between min and max z vertices of the polygon.
	// Than calculate maximum texture coordinates deviation along the edge.
	// If this value is less than specific threshold - enable affine texturing.

	// TODO - maybe use inverse function - enable texel shift no more than this threshold?

	let edge = max_inv_z_point - min_inv_z_point;
	let edge_square_len = edge.magnitude2();
	if edge_square_len == 0.0
	{
		return true;
	}

	let edge_len = edge_square_len.sqrt();
	let edge_vec_normalized = edge / edge_len;

	let inv_z_clamp = 1.0 / ((1 << 20) as f32);
	let min_point_inv_z = depth_equation.sample_point(min_inv_z_point).max(inv_z_clamp);
	let max_point_inv_z = depth_equation.sample_point(max_inv_z_point).max(inv_z_clamp);

	let depth_equation_projected_a =
		Vec2f::new(depth_equation.d_inv_z_dx, depth_equation.d_inv_z_dy).dot(edge_vec_normalized);
	let depth_equation_projected_b = min_point_inv_z;

	if depth_equation_projected_a.abs() < 1.0e-10
	{
		// Z is almost constant along this edge.
		return true;
	}

	let depth_b_div_a = depth_equation_projected_b / depth_equation_projected_a;
	let max_diff_point = ((0.0 + depth_b_div_a) * (edge_len + depth_b_div_a)).sqrt() - depth_b_div_a;

	let max_diff_point_inv_z = depth_equation_projected_a * max_diff_point + depth_equation_projected_b;

	for i in 0 .. 2
	{
		let min_point_tc = tex_coord_equation.d_tc_dx[i] * min_inv_z_point.x +
			tex_coord_equation.d_tc_dy[i] * min_inv_z_point.y +
			tex_coord_equation.k[i];
		let max_point_tc = tex_coord_equation.d_tc_dx[i] * max_inv_z_point.x +
			tex_coord_equation.d_tc_dy[i] * max_inv_z_point.y +
			tex_coord_equation.k[i];

		let tc_projected_a =
			Vec2f::new(tex_coord_equation.d_tc_dx[i], tex_coord_equation.d_tc_dy[i]).dot(edge_vec_normalized);
		let tc_projected_b = min_point_tc;

		let min_point_tc_z_mul = min_point_tc / min_point_inv_z;
		let max_point_tc_z_mul = max_point_tc / max_point_inv_z;

		// Calculate difference of true texture coordinates and linear approximation (based on edge points).

		let max_diff_point_tc_real = (tc_projected_a * max_diff_point + tc_projected_b) / max_diff_point_inv_z;
		let max_diff_point_tc_approximate =
			min_point_tc_z_mul + (max_point_tc_z_mul - min_point_tc_z_mul) * (max_diff_point - 0.0) / (edge_len - 0.0);
		let tc_abs_diff = (max_diff_point_tc_real - max_diff_point_tc_approximate).abs();
		if tc_abs_diff > TC_ERROR_THRESHOLD
		{
			// Difference is too large - can't use affine texturing.
			return false;
		}
	}

	true
}

fn line_z_corrected_texture_coordinates_interpolation_may_be_used(
	depth_equation: &DepthEquation,
	tex_coord_equation: &TexCoordEquation,
	max_inv_z_point: &Vec2f,
	min_polygon_x: f32,
	max_polygon_x: f32,
) -> bool
{
	// Build linear approximation of texture coordinates function based on two points with y = max_inv_z_point.y and x = min/max polygon point x.
	// If linear approximation error is smaller than threshold - use line z corrected texture coordinates interpolation.

	if max_polygon_x - min_polygon_x < 1.0
	{
		// Thin polygon - can use line z corrected texture coordinates interpolation.
		return true;
	}

	let test_line_depth_equation_a = depth_equation.d_inv_z_dx;
	let test_line_depth_equation_b = depth_equation.d_inv_z_dy * max_inv_z_point.y + depth_equation.k;

	if test_line_depth_equation_a.abs() < 1.0e-10
	{
		// Z is almost constant along line.
		return true;
	}

	let depth_b_div_a = test_line_depth_equation_b / test_line_depth_equation_a;
	let max_diff_x = ((min_polygon_x + depth_b_div_a) * (max_polygon_x + depth_b_div_a)).sqrt() - depth_b_div_a;

	let max_diff_point_inv_z = test_line_depth_equation_a * max_diff_x + test_line_depth_equation_b;
	let inv_z_at_min_x = test_line_depth_equation_a * min_polygon_x + test_line_depth_equation_b;
	let inv_z_at_max_x = test_line_depth_equation_a * max_polygon_x + test_line_depth_equation_b;

	let almost_zero = 1e-20;
	if inv_z_at_min_x <= almost_zero || inv_z_at_max_x <= almost_zero || max_diff_point_inv_z <= almost_zero
	{
		// Overflow of inv_z - possible for inclined polygons.
		return false;
	}

	for i in 0 .. 2
	{
		let test_line_tex_coord_equation_a = tex_coord_equation.d_tc_dx[i];
		let test_line_tex_coord_equation_b =
			tex_coord_equation.d_tc_dy[i] * max_inv_z_point.y + tex_coord_equation.k[i];

		let tc_at_min_x =
			(test_line_tex_coord_equation_a * min_polygon_x + test_line_tex_coord_equation_b) / inv_z_at_min_x;
		let tc_at_max_x =
			(test_line_tex_coord_equation_a * max_polygon_x + test_line_tex_coord_equation_b) / inv_z_at_max_x;

		let max_diff_point_tc_real =
			(test_line_tex_coord_equation_a * max_diff_x + test_line_tex_coord_equation_b) / max_diff_point_inv_z;
		let max_diff_point_tc_approximate =
			tc_at_min_x + (tc_at_max_x - tc_at_min_x) * (max_diff_x - min_polygon_x) / (max_polygon_x - min_polygon_x);
		let tc_abs_diff = (max_diff_point_tc_real - max_diff_point_tc_approximate).abs();
		if tc_abs_diff > TC_ERROR_THRESHOLD
		{
			// Difference is too large - can't use line z corrected texture coordinates interpolation.
			return false;
		}
	}
	true
}

const TC_ERROR_THRESHOLD: f32 = 0.75;

pub const MAX_VERTICES: usize = 24;
const MAX_LEAF_CLIP_PLANES: usize = 20;

const Z_NEAR: f32 = 1.0;

// Returns number of result vertices. < 3 if polygon is clipped.
pub fn project_and_clip_polygon(
	clip_planes: &ClippingPolygonPlanes,
	vertices_transformed: &[Vec3f],
	out_vertices: &mut [Vec2f],
) -> usize
{
	let mut vertex_count = std::cmp::min(vertices_transformed.len(), MAX_VERTICES);

	// Perform z_near clipping.
	let mut vertices_transformed_z_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	vertex_count = clip_3d_polygon_by_z_plane(
		&vertices_transformed[.. vertex_count],
		Z_NEAR,
		&mut vertices_transformed_z_clipped,
	);
	if vertex_count < 3
	{
		return vertex_count;
	}

	// Make 2d vertices, then perform clipping in 2d space.
	// This is needed to avoid later overflows for Fixed16 vertex coords in rasterizer.
	for (vertex_transformed, out_vertex) in vertices_transformed_z_clipped
		.iter()
		.take(vertex_count)
		.zip(out_vertices.iter_mut())
	{
		*out_vertex = vertex_transformed.truncate() / vertex_transformed.z;
	}

	let mut vertices_temp = [Vec2f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory

	// Perform clipping in pairs - use pair of buffers.
	for i in 0 .. clip_planes.len() / 2
	{
		vertex_count = clip_2d_polygon(
			&out_vertices[.. vertex_count],
			&clip_planes[i * 2],
			&mut vertices_temp[..],
		);
		if vertex_count < 3
		{
			return vertex_count;
		}
		vertex_count = clip_2d_polygon(
			&vertices_temp[.. vertex_count],
			&clip_planes[i * 2 + 1],
			&mut out_vertices[..],
		);
		if vertex_count < 3
		{
			return vertex_count;
		}
	}

	vertex_count
}

fn calculate_mip(points: &[Vec2f], depth_equation: &DepthEquation, tc_equation: &TexCoordEquation, mip_bias: f32)
	-> u32
{
	// Calculate screen-space derivatives of texture coordinates for closest polygon point.
	// Calculate mip-level as logarithm of maximim texture coordinate component derivative.

	let mut mip_point = points[0];
	let mut mip_point_inv_z = 0.0;
	for p in points
	{
		let inv_z = depth_equation.sample_point(p);
		if inv_z > mip_point_inv_z
		{
			mip_point_inv_z = inv_z;
			mip_point = *p;
		}
	}

	let z_2 = 1.0 / (mip_point_inv_z * mip_point_inv_z);
	let z_4 = z_2 * z_2;

	let mut d_tc_2: [f32; 2] = [0.0, 0.0];
	for i in 0 .. 2
	{
		let d_tc_dx = tc_equation.d_tc_dx[i] * (depth_equation.k + depth_equation.d_inv_z_dy * mip_point.y) -
			(tc_equation.k[i] + tc_equation.d_tc_dy[i] * mip_point.y) * depth_equation.d_inv_z_dx;
		let d_tc_dy = tc_equation.d_tc_dy[i] * (depth_equation.k + depth_equation.d_inv_z_dx * mip_point.x) -
			(tc_equation.k[i] + tc_equation.d_tc_dx[i] * mip_point.x) * depth_equation.d_inv_z_dy;

		d_tc_2[i] = (d_tc_dx * d_tc_dx + d_tc_dy * d_tc_dy) * z_4;
	}

	let max_d_tc_2 = d_tc_2[0].max(d_tc_2[1]);
	let mip_f = max_d_tc_2.log2() * 0.5 + mip_bias; // log(sqrt(x)) = log(x) * 0.5
	let mip = std::cmp::max(0, std::cmp::min(mip_f.ceil() as i32, MAX_MIP as i32));

	mip as u32
}
