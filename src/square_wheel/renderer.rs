use super::{
	abstract_color::*, config, debug_stats_printer::*, depth_renderer::*, draw_ordering, dynamic_models_index::*,
	fast_math::*, frame_info::*, frame_number::*, inline_models_index::*, map_materials_processor::*,
	map_visibility_calculator::*, performance_counter::*, rasterizer::*, rect_splitting, renderer_config::*,
	resources_manager::*, shadow_map::*, surfaces::*, textures::*, triangle_model::*, triangle_models_rendering::*,
};
use common::{
	bsp_map_compact, clipping::*, clipping_polygon::*, fixed_math::*, lightmap, material, math_types::*, matrix::*,
	plane::*, shared_mut_slice::*, system_window,
};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub struct Renderer
{
	app_config: config::ConfigSharedPtr,
	config: RendererConfig,
	config_is_durty: bool,

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
	materials_processor: MapMaterialsProcessor,
	performance_counters: Arc<Mutex<RendererPerformanceCounters>>,
	// TODO - maybe extract dynamic models-related stuff into separate class?
	dynamic_models_index: DynamicModelsIndex,
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
#[derive(Default, Copy, Clone)]
struct DrawPolygonData
{
	// Frame last time this polygon was visible.
	visible_frame: FrameNumber,
	depth_equation: DepthEquation,
	tex_coord_equation: TexCoordEquation,
	surface_pixels_offset: usize,
	surface_size: [u32; 2],
	mip: u32,
	surface_tc_min: [i32; 2],
}

struct VisibleDynamicMeshInfo
{
	entity_index: u32,
	mesh_index: u32,
	vertices_offset: usize,
	triangles_offset: usize,
	num_visible_triangles: usize,
	bbox_vertices_transformed: [Vec3f; 8],
	camera_matrices: CameraMatrices,
	light: bsp_map_compact::LightGridElement,
}

#[derive(Default, Copy, Clone)]
struct DynamicModelInfo
{
	first_visible_mesh: u32,
	num_visible_meshes: u32,
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

		let materials_processor = MapMaterialsProcessor::new(resources_manager, &*map);

		Renderer {
			app_config,
			config: config_parsed,
			config_is_durty: false,
			current_frame: FrameNumber::default(),
			polygons_data: vec![DrawPolygonData::default(); map.polygons.len()],
			vertices_transformed: vec![Vec3f::new(0.0, 0.0, 0.0); map.vertices.len()],
			surfaces_pixels: Vec::new(),
			num_visible_surfaces_pixels: 0,
			current_frame_visible_polygons: Vec::with_capacity(map.polygons.len()),
			mip_bias: 0.0,
			visibility_calculator: MapVisibilityCalculator::new(map.clone()),
			shadows_maps_renderer: DepthRenderer::new(map.clone()),
			map: map.clone(),
			materials_processor,
			performance_counters: Arc::new(Mutex::new(RendererPerformanceCounters::new())),
			dynamic_models_index: DynamicModelsIndex::new(map),
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
		inline_models_index: &InlineModelsIndex,
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

		let depth_map_size = 256;
		let mut test_lights_shadow_maps = Vec::with_capacity(frame_info.lights.len());
		// TODO - perform parallel shadowmaps build.
		for light in &frame_info.lights
		{
			let mut cube_shadow_map = CubeShadowMap {
				size: depth_map_size,
				sides: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()],
			};
			for side in 0 .. 6
			{
				let depth_matrices = calculate_cube_shadow_map_side_matrices(
					light.pos,
					depth_map_size as f32,
					int_to_cubemap_side(side).unwrap(),
				);

				let mut depth_data = vec![0.0; (depth_map_size * depth_map_size) as usize];
				self.shadows_maps_renderer
					.draw_map(&mut depth_data, depth_map_size, depth_map_size, &depth_matrices);

				cube_shadow_map.sides[side as usize] = depth_data;
			}
			test_lights_shadow_maps.push(cube_shadow_map);
		}

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

		run_with_measure(
			|| {
				self.dynamic_models_index.position_models(&frame_info.model_entities);
				self.prepare_dynamic_models(&frame_info.model_entities);
				self.build_dynamic_models_buffers(&frame_info.camera_matrices, &frame_info.model_entities);
			},
			&mut performance_counters.triangle_models_preparation,
		);

		run_with_measure(
			|| {
				self.prepare_polygons_surfaces(&frame_info.camera_matrices, inline_models_index);
				self.allocate_surfaces_pixels::<ColorT>();

				// TODO - avoid allocation.
				let mut lights_with_shadow_maps = Vec::new();
				for (light, shadow_map) in frame_info.lights.iter().zip(test_lights_shadow_maps.iter())
				{
					lights_with_shadow_maps.push((light, shadow_map));
				}
				self.build_polygons_surfaces::<ColorT>(&frame_info.camera_matrices, &lights_with_shadow_maps);
			},
			&mut performance_counters.surfaces_preparation,
		);
	}

	pub fn draw_frame<ColorT: AbstractColor>(
		&mut self,
		pixels: &mut [ColorT],
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
		inline_models_index: &InlineModelsIndex,
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
			|| {
				let root_node = (self.map.nodes.len() - 1) as u32;
				self.perform_rasterization(
					pixels,
					surface_info,
					&frame_info.camera_matrices,
					inline_models_index,
					&frame_info.model_entities,
					root_node,
				)
			},
			&mut performance_counters.rasterization,
		);

		if debug_stats_printer.show_debug_stats()
		{
			self.print_debug_stats(
				frame_info,
				inline_models_index,
				debug_stats_printer,
				&performance_counters,
			);
		}
	}

	fn print_debug_stats(
		&mut self,
		frame_info: &FrameInfo,
		inline_models_index: &InlineModelsIndex,
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
				num_visible_submodels_parts += inline_models_index.get_leaf_models(leaf_index as u32).len();
				num_visible_meshes_parts += self.dynamic_models_index.get_leaf_models(leaf_index as u32).len();
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
				VertexData::VertexAnimated(va) => va.constant.len(),
				VertexData::SkeletonAnimated(v) => v.len(),
			};
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
		debug_stats_printer.add_line(format!(
			"surfaces pixels: {}k",
			(self.num_visible_surfaces_pixels + 1023) / 1024
		));
		debug_stats_printer.add_line(format!("mip bias: {}", self.mip_bias));
	}

	// Call this after visible leafs search.
	fn prepare_dynamic_models(&mut self, models: &[ModelEntity])
	{
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
			let mut visible = model.is_view_model;
			for leaf_index in self.dynamic_models_index.get_model_leafs(entity_index)
			{
				if self
					.visibility_calculator
					.get_current_frame_leaf_bounds(*leaf_index)
					.is_some()
				{
					visible = true;
					break;
				}
			}

			if !visible
			{
				dynamic_model_info.first_visible_mesh = 0;
				dynamic_model_info.num_visible_meshes = 0;
				continue;
			}

			dynamic_model_info.first_visible_mesh = self.visible_dynamic_meshes_list.len() as u32;

			for (mesh_index, mesh) in model.model.meshes.iter().enumerate()
			{
				self.visible_dynamic_meshes_list.push(VisibleDynamicMeshInfo {
					entity_index: entity_index as u32,
					mesh_index: mesh_index as u32,
					vertices_offset,
					triangles_offset,
					num_visible_triangles: 0,                      // set later
					bbox_vertices_transformed: [Vec3f::zero(); 8], // set later
					camera_matrices: CameraMatrices {
						view_matrix: Mat4f::zero(),
						planes_matrix: Mat4f::zero(),
						position: Vec3f::zero(),
					}, // Set later
					light: bsp_map_compact::LightGridElement::default(), // Set later
				});

				let num_vertices = match &mesh.vertex_data
				{
					VertexData::VertexAnimated(va) => va.constant.len(),
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
				},
			);
		}
		if triangles_offset > self.dynamic_meshes_triangles.len()
		{
			self.dynamic_meshes_triangles.resize(triangles_offset, [0, 0, 0]);
		}
	}

	fn build_dynamic_models_buffers(&mut self, camera_matrices: &CameraMatrices, models: &[ModelEntity])
	{
		// It is safe to share vertices and triangle buffers since each mesh uses its own region.
		let dst_vertices_shared = SharedMutSlice::new(&mut self.dynamic_meshes_vertices);
		let dst_triangles_shared = SharedMutSlice::new(&mut self.dynamic_meshes_triangles);

		let map = &self.map;

		let func = |visible_dynamic_mesh: &mut VisibleDynamicMeshInfo| {
			let model = &models[visible_dynamic_mesh.entity_index as usize];
			let animation = &model.animation;

			visible_dynamic_mesh.light = fetch_light_from_grid(map, &model.position);

			let model_matrix = get_object_matrix(model.position, model.angles);
			let model_matrix_inverse = model_matrix.transpose().invert().unwrap();

			let final_matrix = camera_matrices.view_matrix * model_matrix;

			visible_dynamic_mesh.camera_matrices.view_matrix = final_matrix;
			visible_dynamic_mesh.camera_matrices.planes_matrix = camera_matrices.planes_matrix * model_matrix_inverse;

			// Transform bbox.
			let bbox = get_current_triangle_model_bbox(&model.model, animation);
			visible_dynamic_mesh.bbox_vertices_transformed = bbox.get_corners_vertices().map(|pos| {
				let pos_transformed = final_matrix * pos.extend(1.0);
				Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w)
			});

			let texture = &model.texture;
			let mesh = &model.model.meshes[visible_dynamic_mesh.mesh_index as usize];

			// Perform vertices transformation.
			let dst_mesh_vertices = unsafe { &mut dst_vertices_shared.get()[visible_dynamic_mesh.vertices_offset ..] };

			animate_and_transform_triangle_mesh_vertices(
				&model.model,
				mesh,
				animation,
				&final_matrix,
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
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
		models: &[ModelEntity],
		root_node: u32,
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

			let viewport_clippung_polygon = ClippingPolygon::from_box(
				screen_rect.min.x,
				screen_rect.min.y,
				screen_rect.max.x,
				screen_rect.max.y,
			);

			self.draw_tree_r(
				&mut rasterizer,
				camera_matrices,
				&viewport_clippung_polygon,
				inline_models_index,
				models,
				root_node,
			);

			self.draw_view_models(&mut rasterizer, &viewport_clippung_polygon, models);
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
				let viewport_clippung_polygon = ClippingPolygon::from_box(
					rect_corrected.min.x,
					rect_corrected.min.y,
					rect_corrected.max.x,
					rect_corrected.max.y,
				);

				self.draw_tree_r(
					&mut rasterizer,
					camera_matrices,
					&viewport_clippung_polygon,
					inline_models_index,
					models,
					root_node,
				);

				self.draw_view_models(&mut rasterizer, &viewport_clippung_polygon, models);
			});
		}
	}

	fn prepare_polygons_surfaces(&mut self, camera_matrices: &CameraMatrices, inline_models_index: &InlineModelsIndex)
	{
		self.current_frame_visible_polygons.clear();

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
		for model_index in 0 .. self.map.submodels.len()
		{
			let mut bounds: Option<ClippingPolygon> = None;
			for &leaf_index in inline_models_index.get_model_leafs(model_index as u32)
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

			let submodel = &self.map.submodels[model_index];

			let model_matrix = inline_models_index.get_model_matrix(model_index as u32);
			let model_matrix_inverse = model_matrix.transpose().invert().unwrap();
			let model_matrices = CameraMatrices {
				view_matrix: camera_matrices.view_matrix * model_matrix,
				planes_matrix: camera_matrices.planes_matrix * model_matrix_inverse,
				position: camera_matrices.position,
			};

			for polygon_index in submodel.first_polygon .. (submodel.first_polygon + submodel.num_polygons)
			{
				self.prepare_polygon_surface(
					&model_matrices,
					&clip_planes,
					&mut surfaces_pixels_accumulated_offset,
					polygon_index as usize,
				);
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

		if !self.materials_processor.get_material(polygon.texture).draw
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

		let plane_transformed_w = -plane_transformed.w;
		let depth_equation = DepthEquation {
			d_inv_z_dx: plane_transformed.x / plane_transformed_w,
			d_inv_z_dy: plane_transformed.y / plane_transformed_w,
			k: plane_transformed.z / plane_transformed_w,
		};

		let tex_coord_equation = &polygon.tex_coord_equation;

		// Calculate texture coordinates equations.
		let tc_basis_transformed = [
			camera_matrices.planes_matrix * tex_coord_equation[0].vec.extend(tex_coord_equation[0].dist),
			camera_matrices.planes_matrix * tex_coord_equation[1].vec.extend(tex_coord_equation[1].dist),
		];
		// Equation projeted to polygon plane.
		let tc_equation = TexCoordEquation {
			d_tc_dx: [
				tc_basis_transformed[0].x + tc_basis_transformed[0].w * depth_equation.d_inv_z_dx,
				tc_basis_transformed[1].x + tc_basis_transformed[1].w * depth_equation.d_inv_z_dx,
			],
			d_tc_dy: [
				tc_basis_transformed[0].y + tc_basis_transformed[0].w * depth_equation.d_inv_z_dy,
				tc_basis_transformed[1].y + tc_basis_transformed[1].w * depth_equation.d_inv_z_dy,
			],
			k: [
				tc_basis_transformed[0].z + tc_basis_transformed[0].w * depth_equation.k,
				tc_basis_transformed[1].z + tc_basis_transformed[1].w * depth_equation.k,
			],
		};

		let mip = calculate_mip(
			&vertices_2d[.. vertex_count],
			&depth_equation,
			&tc_equation,
			self.mip_bias,
		);
		let tc_equation_scale = 1.0 / ((1 << mip) as f32);

		let tc_equation_scaled = TexCoordEquation {
			d_tc_dx: [
				tc_equation.d_tc_dx[0] * tc_equation_scale,
				tc_equation.d_tc_dx[1] * tc_equation_scale,
			],
			d_tc_dy: [
				tc_equation.d_tc_dy[0] * tc_equation_scale,
				tc_equation.d_tc_dy[1] * tc_equation_scale,
			],
			k: [
				tc_equation.k[0] * tc_equation_scale,
				tc_equation.k[1] * tc_equation_scale,
			],
		};

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
			let inv_z =
				(depth_equation.d_inv_z_dx * p.x + depth_equation.d_inv_z_dy * p.y + depth_equation.k).max(1.0 / max_z);
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
		lights: &[LightWithShadowMap],
	)
	{
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

			let texture = &materials_processor.get_texture(polygon.texture)[polygon_data.mip as usize];
			let surface_data = unsafe {
				&mut surfaces_pixels_shared.get()[polygon_data.surface_pixels_offset ..
					polygon_data.surface_pixels_offset + (surface_size[0] * surface_size[1]) as usize]
			};

			let mip_scale = 1.0 / (1 << polygon_data.mip) as f32;
			let tex_coord_equation_scaled = [
				Plane {
					vec: polygon.tex_coord_equation[0].vec * mip_scale,
					dist: polygon.tex_coord_equation[0].dist * mip_scale,
				},
				Plane {
					vec: polygon.tex_coord_equation[1].vec * mip_scale,
					dist: polygon.tex_coord_equation[1].dist * mip_scale,
				},
			];

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
					&polygon.plane,
					&tex_coord_equation_scaled,
					surface_size,
					polygon_data.surface_tc_min,
					texture,
					lightmap_size,
					lightmap_scale_log2,
					lightmap_tc_shift,
					polygon_lightmap_data,
					lights,
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
					&polygon.plane,
					&tex_coord_equation_scaled,
					surface_size,
					polygon_data.surface_tc_min,
					texture,
					lightmap_size,
					lightmap_scale_log2,
					lightmap_tc_shift,
					polygon_lightmap_data,
					lights,
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
		let clip_planes = viewport_clipping_polygon.get_clip_planes();

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
					&clip_planes,
					&[], // No 3d clip planes.
					models,
					&self.visible_dynamic_meshes_list[visible_mesh_index as usize],
				);
			}
		}
	}

	fn draw_tree_r<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		camera_matrices: &CameraMatrices,
		viewport_clippung_polygon: &ClippingPolygon,
		inline_models_index: &InlineModelsIndex,
		models: &[ModelEntity],
		current_index: u32,
	)
	{
		if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
			if let Some(mut leaf_bounds) = self.visibility_calculator.get_current_frame_leaf_bounds(leaf)
			{
				leaf_bounds.intersect(viewport_clippung_polygon);
				if leaf_bounds.is_valid_and_non_empty()
				{
					self.draw_leaf(
						rasterizer,
						camera_matrices,
						&leaf_bounds,
						inline_models_index,
						models,
						leaf,
					);
				}
			}
		}
		else
		{
			let node = &self.map.nodes[current_index as usize];
			let plane_transformed = camera_matrices.planes_matrix * node.plane.vec.extend(-node.plane.dist);
			let mut mask = if plane_transformed.w >= 0.0 { 1 } else { 0 };
			if self.config.invert_polygons_order
			{
				mask ^= 1;
			}
			for i in 0 .. 2
			{
				self.draw_tree_r(
					rasterizer,
					camera_matrices,
					viewport_clippung_polygon,
					inline_models_index,
					models,
					node.children[(i ^ mask) as usize],
				);
			}
		}
	}

	fn draw_leaf<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		camera_matrices: &CameraMatrices,
		bounds: &ClippingPolygon,
		inline_models_index: &InlineModelsIndex,
		models: &[ModelEntity],
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

		// Draw contents of leaf - submodels and triangle models.

		let leaf_submodels = inline_models_index.get_leaf_models(leaf_index);
		let leaf_dynamic_models = self.dynamic_models_index.get_leaf_models(leaf_index);
		if leaf_submodels.is_empty() && leaf_dynamic_models.is_empty()
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
		// Do this in camera space (transform clip planes for this).
		for &portal_index in &self.map.leafs_portals
			[(leaf.first_leaf_portal as usize) .. ((leaf.first_leaf_portal + leaf.num_leaf_portals) as usize)]
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
		for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
		{
			add_clip_plane(self.map.polygons[polygon_index as usize].plane);
		}

		// Perform planes transformation after deduplication.
		// This is needed because deduplication works badly in stretched camera space.
		// Also it's faster to transform only unique planes.
		for plane in &mut leaf_clip_planes[.. num_clip_planes]
		{
			let plane_transformed_vec4 = camera_matrices.planes_matrix * plane.vec.extend(-plane.dist);
			*plane = Plane {
				vec: plane_transformed_vec4.truncate(),
				dist: -plane_transformed_vec4.w,
			};
		}

		// Fast path for cases with single model, to avoid expensive sorting structures preparations.
		if leaf_submodels.len() == 1 && leaf_dynamic_models.len() == 0
		{
			self.draw_submodel_in_leaf(
				rasterizer,
				&clip_planes,
				&leaf_clip_planes[.. num_clip_planes],
				leaf_submodels[0],
			);
			return;
		}
		if leaf_submodels.len() == 0 && leaf_dynamic_models.len() == 1
		{
			let entry = self.dynamic_model_to_dynamic_meshes_index[leaf_dynamic_models[0] as usize];
			for visible_mesh_index in entry.first_visible_mesh .. entry.first_visible_mesh + entry.num_visible_meshes
			{
				self.draw_mesh(
					rasterizer,
					&clip_planes,
					&leaf_clip_planes[.. num_clip_planes],
					models,
					&self.visible_dynamic_meshes_list[visible_mesh_index as usize],
				);
			}
			return;
		}

		// Multiple models. Sort them.

		// TODO - use uninitialized memory and increase this value.
		const MAX_SUBMODELS_IN_LEAF: usize = 12;
		let mut models_for_sorting = [draw_ordering::BBoxForDrawOrdering::default(); MAX_SUBMODELS_IN_LEAF];

		for (&model_index, model_for_sorting) in leaf_submodels.iter().zip(models_for_sorting.iter_mut())
		{
			let bbox = inline_models_index.get_model_bbox_for_ordering(model_index);
			let model_matrix = inline_models_index.get_model_matrix(model_index as u32);
			let model_matrix_inverse = model_matrix.transpose().invert().unwrap();

			*model_for_sorting = (
				model_index,
				draw_ordering::project_bbox(
					&bbox,
					&CameraMatrices {
						view_matrix: camera_matrices.view_matrix * model_matrix,
						planes_matrix: camera_matrices.planes_matrix * model_matrix_inverse,
						position: camera_matrices.position,
					},
				),
			);
		}
		let mut num_models = std::cmp::min(leaf_submodels.len(), MAX_SUBMODELS_IN_LEAF);

		const DYNAMIC_MESH_INDEX_ADD: u32 = 65536;
		for dynamic_model_index in leaf_dynamic_models
		{
			if num_models == MAX_SUBMODELS_IN_LEAF
			{
				break;
			}

			let model = &models[*dynamic_model_index as usize];
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

		draw_ordering::order_bboxes(&mut models_for_sorting[.. num_models]);

		// Draw dynamic models and submodels, located in this leaf, after leaf polygons.
		for (submodel_index, _bbox) in &models_for_sorting[.. num_models]
		{
			if *submodel_index >= DYNAMIC_MESH_INDEX_ADD
			{
				let visible_mesh_index = *submodel_index - DYNAMIC_MESH_INDEX_ADD;
				self.draw_mesh(
					rasterizer,
					&clip_planes,
					&leaf_clip_planes[.. num_clip_planes],
					models,
					&self.visible_dynamic_meshes_list[visible_mesh_index as usize],
				);
			}
			else
			{
				self.draw_submodel_in_leaf(
					rasterizer,
					&clip_planes,
					&leaf_clip_planes[.. num_clip_planes],
					*submodel_index,
				);
			}
		}
	}

	fn draw_submodel_in_leaf<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		clip_planes: &ClippingPolygonPlanes,
		leaf_clip_planes: &[Plane],
		submodel_index: u32,
	)
	{
		let submodel = &self.map.submodels[submodel_index as usize];
		for polygon_index in submodel.first_polygon .. (submodel.first_polygon + submodel.num_polygons)
		{
			self.draw_submodel_polygon(rasterizer, &clip_planes, leaf_clip_planes, polygon_index);
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

		// HACK! Shift polygon vertices a bit away from camera to fix buggy pol.ygon clipping,
		// When polygon lies exactly on clip plane.
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

	fn draw_mesh<'a, ColorT: AbstractColor>(
		&self,
		rasterizer: &mut Rasterizer<'a, ColorT>,
		clip_planes: &ClippingPolygonPlanes,
		leaf_clip_planes: &[Plane],
		models: &[ModelEntity],
		visible_dynamic_mesh: &VisibleDynamicMeshInfo,
	)
	{
		// let bbox_vertices = visible_dynamic_mesh.bbox_vertices_transformed.map(|v| ModelVertex3d{ pos: v, tc: Vec2f::zero()} );
		// let bbox_triangles =
		// [
		// [0, 3, 1 ], [ 0, 1, 2 ],
		// [ 4, 6, 7 ], [ 4, 7, 5 ],
		// [ 0, 1, 5 ], [ 0, 5, 4 ],
		// [ 2, 3, 7 ], [2, 7, 6 ],
		// [ 1, 3, 7 ], [ 1, 7, 5] ,
		// [ 0, 2, 6], [ 0, 6, 4],
		// ];

		let model = &models[visible_dynamic_mesh.entity_index as usize];

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

		let texture = &model.texture;

		// TODO - use individual texture for each mesh.
		let texture_info = TextureInfo {
			size: [texture.size[0] as i32, texture.size[1] as i32],
		};

		let texture_data = &texture.pixels;

		let mut vertices_clipped = unsafe { std::mem::zeroed::<[ModelVertex3d; MAX_VERTICES]>() };
		let mut vertices_clipped_temp = unsafe { std::mem::zeroed::<[ModelVertex3d; MAX_VERTICES]>() };
		let mut vertices_projected = unsafe { std::mem::zeroed::<[ModelVertex2d; MAX_VERTICES]>() };
		let mut vertices_projected_temp = unsafe { std::mem::zeroed::<[ModelVertex2d; MAX_VERTICES]>() };
		let mut vertices_fixed = unsafe { std::mem::zeroed::<[TrianglePointProjected; MAX_VERTICES]>() };

		let vertices_combined = &self.dynamic_meshes_vertices[visible_dynamic_mesh.vertices_offset ..];

		let triangles = &self.dynamic_meshes_triangles[visible_dynamic_mesh.triangles_offset ..
			visible_dynamic_mesh.triangles_offset + visible_dynamic_mesh.num_visible_triangles];
		for triangle in triangles
		{
			for (&index, dst_vertex) in triangle.iter().zip(vertices_clipped.iter_mut())
			{
				*dst_vertex = triangle_vertex_debug_checked_fetch(vertices_combined, index);
			}

			let mut num_vertices = 3;
			for clip_plane in &clip_planes_3d[.. num_clip_planes_3d]
			{
				num_vertices = clip_3d_model_polygon_by_plane(
					&vertices_clipped[0 .. num_vertices],
					clip_plane,
					&mut vertices_clipped_temp,
				);
				if num_vertices < 3
				{
					break;
				}
				vertices_clipped[.. num_vertices].copy_from_slice(&vertices_clipped_temp[.. num_vertices]);
			}
			if num_vertices < 3
			{
				continue;
			}

			for (src, dst) in vertices_clipped[0 .. num_vertices]
				.iter()
				.zip(vertices_projected.iter_mut())
			{
				*dst = ModelVertex2d {
					pos: src.pos.truncate() / src.pos.z,
					tc: src.tc,
				};
			}

			// TODO - use only required clip planes.
			for clip_plane in clip_planes
			{
				num_vertices = clip_2d_model_polygon(
					&vertices_projected[0 .. num_vertices],
					clip_plane,
					&mut vertices_projected_temp,
				);
				if num_vertices < 3
				{
					break;
				}
				vertices_projected[.. num_vertices].copy_from_slice(&vertices_projected_temp[.. num_vertices]);
			}
			if num_vertices < 3
			{
				continue;
			}

			for (src, dst) in vertices_projected[0 .. num_vertices]
				.iter()
				.zip(vertices_fixed.iter_mut())
			{
				*dst = TrianglePointProjected {
					x: f32_to_fixed16(src.pos.x),
					y: f32_to_fixed16(src.pos.y),
					tc: [f32_to_fixed16(src.tc.x), f32_to_fixed16(src.tc.y)],
				};
			}

			for t in 0 .. num_vertices - 2
			{
				// TODO - use unchecked vertex fetch?
				rasterizer.fill_triangle(
					&[vertices_fixed[0], vertices_fixed[t + 1], vertices_fixed[t + 2]],
					// TODO - perform per-vertex lighting
					&visible_dynamic_mesh.light.light_cube[0],
					&texture_info,
					texture_data,
				);
			} // for subtriangles
		} // For triangles
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
		if self.config_is_durty
		{
			self.config_is_durty = false;
			self.config.update_app_config(&self.app_config);
		}
		else
		{
			self.config = RendererConfig::from_app_config(&self.app_config);
		}

		// Make sure that config values are reasonable.
		if self.config.textures_mip_bias < -1.0
		{
			self.config.textures_mip_bias = -1.0;
			self.config_is_durty = true;
		}
		if self.config.textures_mip_bias > 2.0
		{
			self.config.textures_mip_bias = 2.0;
			self.config_is_durty = true;
		}
	}
}

fn fetch_light_from_grid(map: &bsp_map_compact::BSPMap, pos: &Vec3f) -> bsp_map_compact::LightGridElement
{
	let zero_light = bsp_map_compact::LightGridElement::default();

	let light_grid_header = &map.light_grid_header;
	if light_grid_header.grid_size[0] == 0 ||
		light_grid_header.grid_size[1] == 0 ||
		light_grid_header.grid_size[2] == 0 ||
		light_grid_header.grid_cell_size[0] == 0.0 ||
		light_grid_header.grid_cell_size[1] == 0.0 ||
		light_grid_header.grid_cell_size[2] == 0.0 ||
		map.light_grid_samples.is_empty() ||
		map.light_grid_columns.is_empty()
	{
		return zero_light;
	}

	let grid_pos = (pos - Vec3f::from(light_grid_header.grid_start))
		.div_element_wise(Vec3f::from(light_grid_header.grid_cell_size));

	let grid_pos_i = [
		grid_pos.x.floor() as i32,
		grid_pos.y.floor() as i32,
		grid_pos.z.floor() as i32,
	];

	// Perform linear interpolation of light grid values.
	// We need to read 8 values in order to do this.
	// Ignore non-existing values and absolute zero values and perform result renormalization.
	let mut total_light = bsp_map_compact::LightGridElement::default();
	let mut total_factor = 0.0;
	for dx in 0 ..= 1
	{
		let x = grid_pos_i[0] + dx;
		if x < 0 || x >= (light_grid_header.grid_size[0] as i32)
		{
			continue;
		}
		let factor_x = 1.0 - (grid_pos.x - (x as f32)).abs();

		for dy in 0 ..= 1
		{
			let y = grid_pos_i[1] + dy;
			if y < 0 || y >= (light_grid_header.grid_size[1] as i32)
			{
				continue;
			}

			let column = map.light_grid_columns[((x as u32) + (y as u32) * light_grid_header.grid_size[0]) as usize];
			if column.num_samples == 0
			{
				continue;
			}

			let factor_y = 1.0 - (grid_pos.y - (y as f32)).abs();

			for dz in 0 ..= 1
			{
				let z = grid_pos_i[2] + dz;
				if z < 0 || z >= (light_grid_header.grid_size[2] as i32)
				{
					continue;
				}

				if (z as u32) < column.start_z || (z as u32) >= column.start_z + column.num_samples
				{
					continue;
				}

				let sample_address_in_column = (z as u32) - column.start_z;
				let sample_value = map.light_grid_samples[(column.first_sample + sample_address_in_column) as usize];
				if sample_value == zero_light
				{
					continue;
				}

				let factor_z = 1.0 - (grid_pos.z - (z as f32)).abs();

				let cur_sample_factor = factor_x * factor_y * factor_z;
				/*
				total_light[0] += sample_value[0] * cur_sample_factor;
				total_light[1] += sample_value[1] * cur_sample_factor;
				total_light[2] += sample_value[2] * cur_sample_factor;
				total_factor += cur_sample_factor;
				*/
				// TODO - perform proper interpolation.
				total_light = sample_value;
				total_factor = 1.0;
			} // for dz
		} // for dy
	} // for dx

	if total_factor <= 0.0
	{
		return zero_light;
	}

	// Perform normalization in case if same sample points were rejected.
	// total_light[0] /= total_factor;
	// total_light[1] /= total_factor;
	// total_light[2] /= total_factor;

	total_light
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
		let inv_z = point.x * depth_equation.d_inv_z_dx + point.y * depth_equation.d_inv_z_dy + depth_equation.k;
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

		let depth_equation_scaled = DepthEquation {
			d_inv_z_dx: depth_equation.d_inv_z_dx * z_scale,
			d_inv_z_dy: depth_equation.d_inv_z_dy * z_scale,
			k: depth_equation.k * z_scale,
		};
		let tex_coord_equation_scaled = TexCoordEquation {
			d_tc_dx: [
				tex_coord_equation.d_tc_dx[0] * z_scale,
				tex_coord_equation.d_tc_dx[1] * z_scale,
			],
			d_tc_dy: [
				tex_coord_equation.d_tc_dy[0] * z_scale,
				tex_coord_equation.d_tc_dy[1] * z_scale,
			],
			k: [tex_coord_equation.k[0] * z_scale, tex_coord_equation.k[1] * z_scale],
		};

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
	let min_point_inv_z = (depth_equation.d_inv_z_dx * min_inv_z_point.x +
		depth_equation.d_inv_z_dy * min_inv_z_point.y +
		depth_equation.k)
		.max(inv_z_clamp);
	let max_point_inv_z = (depth_equation.d_inv_z_dx * max_inv_z_point.x +
		depth_equation.d_inv_z_dy * max_inv_z_point.y +
		depth_equation.k)
		.max(inv_z_clamp);

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
	for &p in points
	{
		let inv_z = depth_equation.d_inv_z_dx * p.x + depth_equation.d_inv_z_dy * p.y + depth_equation.k;
		if inv_z > mip_point_inv_z
		{
			mip_point_inv_z = inv_z;
			mip_point = p;
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
