use super::{clipping_polygon::*, inline_models_index::*, rasterizer::*, renderer_config::*};
use common::{
	bsp_map_compact, camera_controller::CameraMatrices, clipping::*, color::*, fixed_math::*, image, math_types::*,
	performance_counter::*, plane::*, system_window,
};
use std::rc::Rc;

type Clock = std::time::Instant;

pub struct Renderer
{
	current_frame: FrameNumber,
	config: RendererConfig,
	map: Rc<bsp_map_compact::BSPMap>,
	leafs_data: Vec<DrawLeafData>,
	portals_data: Vec<DrawPortalData>,
	polygons_data: Vec<DrawPolygonData>,
	vertices_transformed: Vec<Vec3f>,
	surfaces_pixels: Vec<Color32>,
	leafs_search_waves: LeafsSearchWavesPair,
	textures: Vec<TextureWithMips>,
	performance_counters: RendererPerformanceCounters,
}

struct RendererPerformanceCounters
{
	frame_duration: PerformanceCounter,
	visible_leafs_search: PerformanceCounter,
	surfaces_preparation: PerformanceCounter,
	rasterization: PerformanceCounter,
}

impl RendererPerformanceCounters
{
	fn new() -> Self
	{
		let window_size = 100;
		Self {
			frame_duration: PerformanceCounter::new(window_size),
			visible_leafs_search: PerformanceCounter::new(window_size),
			surfaces_preparation: PerformanceCounter::new(window_size),
			rasterization: PerformanceCounter::new(window_size),
		}
	}
}

// Mutable data associated with map BSP Leaf.
#[derive(Default, Copy, Clone)]
struct DrawLeafData
{
	// Frame last time this leaf was visible.
	visible_frame: FrameNumber,
	// Bounds, combined from all paths through portals.
	current_frame_bounds: ClippingPolygon,
	num_search_visits: usize,
}

// Mutable data associated with map portal.
#[derive(Default, Copy, Clone)]
struct DrawPortalData
{
	// Frame last time this leaf was visible.
	visible_frame: FrameNumber,
	// None if behind camera.
	current_frame_projection: Option<ClippingPolygon>,
}

// Mutable data associated with map polygon.
#[derive(Default, Copy, Clone)]
struct DrawPolygonData
{
	// Precalculated during map loading min/max texture coordinates
	tc_min: [f32; 2],
	tc_max: [f32; 2],

	// Frame last time this polygon was visible.
	visible_frame: FrameNumber,
	depth_equation: DepthEquation,
	tex_coord_equation: TexCoordEquation,
	surface_pixels_offset: usize,
	surface_size: [u32; 2],
	mip: u32,
	surface_tc_min: [i32; 2],
}

// 32 bits are enough for frames enumeration.
// It is more than year at 60FPS.
#[derive(Default, Copy, Clone, PartialEq, Eq)]
struct FrameNumber(u32);

type LeafsSearchWaveElement = u32; // Leaf index
type LeafsSearchWave = Vec<LeafsSearchWaveElement>;
#[derive(Default)]
struct LeafsSearchWavesPair(LeafsSearchWave, LeafsSearchWave);

const MAX_MIP: usize = 3;
const NUM_MIPS: usize = MAX_MIP + 1;
type TextureWithMips = [image::Image; NUM_MIPS];

impl Renderer
{
	pub fn new(app_config: &serde_json::Value, map: Rc<bsp_map_compact::BSPMap>) -> Self
	{
		let textures = load_textures(&map.textures);

		let mut polygons_data = vec![DrawPolygonData::default(); map.polygons.len()];
		precalculate_polygons_tex_coords_bounds(&map, &mut polygons_data);

		Renderer {
			current_frame: FrameNumber(0),
			config: RendererConfig::from_app_config(app_config),
			leafs_data: vec![DrawLeafData::default(); map.leafs.len()],
			portals_data: vec![DrawPortalData::default(); map.portals.len()],
			polygons_data,
			vertices_transformed: vec![Vec3f::new(0.0, 0.0, 0.0); map.vertices.len()],
			surfaces_pixels: Vec::new(),
			leafs_search_waves: LeafsSearchWavesPair::default(),
			map,
			textures,
			performance_counters: RendererPerformanceCounters::new(),
		}
	}

	pub fn draw_frame(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
	)
	{
		let frame_start_time = Clock::now();
		self.current_frame.0 += 1;

		if self.config.clear_background
		{
			draw_background(pixels);
		}

		let mut debug_stats = DebugStats::default();

		self.draw_map(
			pixels,
			surface_info,
			camera_matrices,
			inline_models_index,
			&mut debug_stats,
		);

		// TODO - remove such temporary fuinction.
		draw_crosshair(pixels, surface_info);

		let frame_end_time = Clock::now();
		let frame_duration_s = (frame_end_time - frame_start_time).as_secs_f32();
		self.performance_counters.frame_duration.add_value(frame_duration_s);

		if self.config.show_stats
		{
			let mut num_visible_leafs = 0;
			let mut max_search_visits = 0;
			let mut num_visible_models_parts = 0;
			for (leaf_index, leaf_data) in self.leafs_data.iter().enumerate()
			{
				if leaf_data.visible_frame == self.current_frame
				{
					num_visible_leafs += 1;
					max_search_visits = std::cmp::max(max_search_visits, leaf_data.num_search_visits);
					num_visible_models_parts += inline_models_index.get_leaf_models(leaf_index as u32).len();
				}
			}

			let mut num_visible_portals = 0;
			for portal_data in &self.portals_data
			{
				if portal_data.visible_frame == self.current_frame
				{
					num_visible_portals += 1;
				}
			}

			let mut num_visible_polygons = 0;
			for polygon_data in &self.polygons_data
			{
				if polygon_data.visible_frame == self.current_frame
				{
					num_visible_polygons += 1;
				}
			}

			common::text_printer::print(
				pixels,
				surface_info,
				&format!(
					"frame time: {:04.2}ms\nvisible leafs search: {:04.2}ms\nsurfaces preparation: \
					 {:04.2}ms\nrasterization: {:04.2}ms\nleafs: {}/{}\nportals: {}/{}\nmodels parts: {}\npolygons: \
					 {}\nsurfaces pixels: {}k\nnum reachable leaf search  calls: {}\nmax visits: {}\nmax reachable \
					 leaf search depth: {}\nmax reqachable leafs search wave size: {}",
					self.performance_counters.frame_duration.get_average_value() * 1000.0,
					self.performance_counters.visible_leafs_search.get_average_value() * 1000.0,
					self.performance_counters.surfaces_preparation.get_average_value() * 1000.0,
					self.performance_counters.rasterization.get_average_value() * 1000.0,
					num_visible_leafs,
					self.leafs_data.len(),
					num_visible_portals,
					self.portals_data.len(),
					num_visible_models_parts,
					num_visible_polygons,
					(self.surfaces_pixels.len() + 1023) / 1024,
					debug_stats.num_reachable_leafs_search_calls,
					max_search_visits,
					debug_stats.reachable_leafs_search_calls_depth,
					debug_stats.reachable_leafs_search_max_wave_size,
				),
				0,
				0,
				Color32::from_rgb(255, 255, 255),
			);
		}
	}

	fn draw_map(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
		debug_stats: &mut DebugStats,
	)
	{
		let mut rasterizer = Rasterizer::new(pixels, surface_info);
		let root_node = (self.map.nodes.len() - 1) as u32;
		let current_leaf = self.find_current_leaf(root_node, &camera_matrices.planes_matrix);

		// TODO - before preparing frame try to shift camera a little bit away from all planes of BSP nodes before current leaf.
		// This is needed to fix possible z_near clipping of current leaf portals.

		let visibile_leafs_search_start_time = Clock::now();

		let frame_bounds = ClippingPolygon::from_box(0.0, 0.0, surface_info.width as f32, surface_info.height as f32);
		if self.config.recursive_visible_leafs_marking
		{
			mark_reachable_leafs_recursive(
				current_leaf,
				&self.map,
				self.current_frame,
				camera_matrices,
				0,
				&frame_bounds,
				&mut self.leafs_data,
				&mut self.portals_data,
				debug_stats,
			);
		}
		else
		{
			self.mark_reachable_leafs_iterative(current_leaf, camera_matrices, &frame_bounds, debug_stats);
		}

		let visibile_leafs_search_end_time = Clock::now();
		let visibile_leafs_search_duration_s =
			(visibile_leafs_search_end_time - visibile_leafs_search_start_time).as_secs_f32();
		self.performance_counters
			.visible_leafs_search
			.add_value(visibile_leafs_search_duration_s);

		let surfaces_preparation_start_time = Clock::now();

		self.prepare_polygons_surfaces(
			camera_matrices,
			&[
				rasterizer.get_width() as f32 * 0.5,
				rasterizer.get_height() as f32 * 0.5,
			],
			inline_models_index,
		);

		self.build_polygons_surfaces();

		let surfaces_preparation_end_time = Clock::now();
		let surfaces_preparation_duration_s =
			(surfaces_preparation_end_time - surfaces_preparation_start_time).as_secs_f32();
		self.performance_counters
			.surfaces_preparation
			.add_value(surfaces_preparation_duration_s);

		let rasterization_start_time = Clock::now();

		// Draw BSP tree in back to front order, skip unreachable leafs.
		self.draw_tree_r(&mut rasterizer, camera_matrices, inline_models_index, root_node);

		let rasterization_end_time = Clock::now();
		let rasterization_duration_s = (rasterization_end_time - rasterization_start_time).as_secs_f32();
		self.performance_counters
			.rasterization
			.add_value(rasterization_duration_s);
	}

	fn find_current_leaf(&self, mut index: u32, planes_matrix: &Mat4f) -> u32
	{
		loop
		{
			if index >= bsp_map_compact::FIRST_LEAF_INDEX
			{
				return index - bsp_map_compact::FIRST_LEAF_INDEX;
			}

			let node = &self.map.nodes[index as usize];
			let plane_transformed = planes_matrix * node.plane.vec.extend(-node.plane.dist);
			index = if plane_transformed.w >= 0.0
			{
				node.children[0]
			}
			else
			{
				node.children[1]
			};
		}
	}

	fn mark_reachable_leafs_iterative(
		&mut self,
		start_leaf: u32,
		camera_matrices: &CameraMatrices,
		start_bounds: &ClippingPolygon,
		debug_stats: &mut DebugStats,
	)
	{
		debug_stats.reachable_leafs_search_max_wave_size = 0;

		let cur_wave = &mut self.leafs_search_waves.0;
		let next_wave = &mut self.leafs_search_waves.1;

		cur_wave.clear();
		next_wave.clear();

		cur_wave.push(start_leaf);
		self.leafs_data[start_leaf as usize].current_frame_bounds = *start_bounds;
		self.leafs_data[start_leaf as usize].visible_frame = self.current_frame;

		let mut depth = 0;
		while !cur_wave.is_empty()
		{
			for &leaf in cur_wave.iter()
			{
				debug_stats.num_reachable_leafs_search_calls += 1;

				let leaf_bounds = self.leafs_data[leaf as usize].current_frame_bounds;

				let leaf_value = self.map.leafs[leaf as usize];
				for &portal in &self.map.leafs_portals[(leaf_value.first_leaf_portal as usize) ..
					((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
				{
					let portal_value = &self.map.portals[portal as usize];

					// Do not look through portals that are facing from camera.
					let portal_plane_pos =
						(camera_matrices.planes_matrix * portal_value.plane.vec.extend(-portal_value.plane.dist)).w;

					let next_leaf;
					if portal_value.leafs[0] == leaf
					{
						if portal_plane_pos <= 0.0
						{
							continue;
						}
						next_leaf = portal_value.leafs[1];
					}
					else
					{
						if portal_plane_pos >= 0.0
						{
							continue;
						}
						next_leaf = portal_value.leafs[0];
					}

					// Same portal may be visited multiple times.
					// So, cache calculation of portal bounds.
					let portal_data = &mut self.portals_data[portal as usize];
					if portal_data.visible_frame != self.current_frame
					{
						portal_data.visible_frame = self.current_frame;
						portal_data.current_frame_projection =
							project_portal(portal_value, &self.map, &camera_matrices.view_matrix);
					}

					let mut bounds_intersection = if let Some(b) = portal_data.current_frame_projection
					{
						b
					}
					else
					{
						continue;
					};
					bounds_intersection.intersect(&leaf_bounds);
					if bounds_intersection.is_empty_or_invalid()
					{
						continue;
					}

					let next_leaf_data = &mut self.leafs_data[next_leaf as usize];
					if next_leaf_data.visible_frame != self.current_frame
					{
						next_leaf_data.visible_frame = self.current_frame;
						next_leaf_data.current_frame_bounds = bounds_intersection;
						next_leaf_data.num_search_visits = 1;
					}
					else
					{
						next_leaf_data.num_search_visits += 1;

						// If we visit this leaf not first time, check if bounds is inside current.
						// If so - we can skip it.
						if next_leaf_data.current_frame_bounds.contains(&bounds_intersection)
						{
							continue;
						}
						// Perform clipping of portals of this leaf using combined bounds to ensure that we visit all possible paths with such bounds.
						next_leaf_data.current_frame_bounds.extend(&bounds_intersection);
					}

					next_wave.push(next_leaf);
				} // For leaf portals.
			} // For wave elements.

			debug_stats.reachable_leafs_search_max_wave_size =
				std::cmp::max(debug_stats.reachable_leafs_search_max_wave_size, next_wave.len());

			cur_wave.clear();
			std::mem::swap(cur_wave, next_wave);

			depth += 1;
			if depth > 1024
			{
				// Prevent infinite loop in case of broken graph.
				break;
			}
		}
		debug_stats.reachable_leafs_search_calls_depth = depth;
	}

	fn prepare_polygons_surfaces(
		&mut self,
		camera_matrices: &CameraMatrices,
		viewport_half_size: &[f32; 2],
		inline_models_index: &InlineModelsIndex,
	)
	{
		let mut surfaces_pixels_accumulated_offset = 0;

		// TODO - try to speed-up iteration, do not scan all leafs.
		for i in 0 .. self.map.leafs.len()
		{
			let leaf_data = &self.leafs_data[i];
			if leaf_data.visible_frame == self.current_frame
			{
				let leaf = &self.map.leafs[i];
				// TODO - maybe just a little bit extend clipping polygon?
				let clip_planes = leaf_data.current_frame_bounds.get_clip_planes();
				for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
				{
					self.prepare_polygon_surface(
						camera_matrices,
						&clip_planes,
						viewport_half_size,
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
				let leaf_data = &self.leafs_data[leaf_index as usize];
				if leaf_data.visible_frame != self.current_frame
				{
					continue;
				}
				if let Some(bounds) = &mut bounds
				{
					bounds.extend(&leaf_data.current_frame_bounds);
				}
				else
				{
					bounds = Some(leaf_data.current_frame_bounds);
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

			for polygon_index in submodel.first_polygon .. (submodel.first_polygon + submodel.num_polygons)
			{
				self.prepare_polygon_surface(
					camera_matrices,
					&clip_planes,
					viewport_half_size,
					&mut surfaces_pixels_accumulated_offset,
					polygon_index as usize,
				);
			}
		}

		// Resize surfaces pixels vector only up to avoid filling it with zeros each frame.
		if self.surfaces_pixels.len() < surfaces_pixels_accumulated_offset
		{
			self.surfaces_pixels
				.resize(surfaces_pixels_accumulated_offset, Color32::from_rgb(0, 0, 0));
		}
	}

	fn prepare_polygon_surface(
		&mut self,
		camera_matrices: &CameraMatrices,
		clip_planes: &ClippingPolygonPlanes,
		viewport_half_size: &[f32; 2],
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
		let d_inv_z_dx = plane_transformed.x / plane_transformed_w;
		let d_inv_z_dy = plane_transformed.y / plane_transformed_w;
		let depth_equation = DepthEquation {
			d_inv_z_dx,
			d_inv_z_dy,
			k: plane_transformed.z / plane_transformed_w -
				d_inv_z_dx * viewport_half_size[0] -
				d_inv_z_dy * viewport_half_size[1],
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
				tc_basis_transformed[0].z + tc_basis_transformed[0].w * depth_equation.k -
					tc_basis_transformed[0].x * viewport_half_size[0] -
					tc_basis_transformed[0].y * viewport_half_size[1],
				tc_basis_transformed[1].z + tc_basis_transformed[1].w * depth_equation.k -
					tc_basis_transformed[1].x * viewport_half_size[0] -
					tc_basis_transformed[1].y * viewport_half_size[1],
			],
		};

		let mip = calculate_mip(
			&vertices_2d[.. vertex_count],
			&depth_equation,
			&tc_equation,
			self.config.textures_mip_bias,
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

		// Reduce min/max texture coordinates slightly to avoid adding extra pixels
		// in case if min/max tex coord is exact integer, but slightly changed due to computational errors.
		// Clamp also coordinates to min/max polygon coordinates (they may be out of range because of computational errors).
		let tc_reduce_eps = 1.0 / 32.0;
		for i in 0 .. 2
		{
			tc_min[i] += tc_reduce_eps;
			tc_max[i] -= tc_reduce_eps;
			let polygon_tc_min = polygon_data.tc_min[i] * tc_equation_scale;
			let polygon_tc_max = polygon_data.tc_max[i] * tc_equation_scale;
			if tc_min[i] < polygon_tc_min
			{
				tc_min[i] = polygon_tc_min;
			}
			if tc_max[i] > polygon_tc_max
			{
				tc_max[i] = polygon_tc_max;
			}
		}

		let max_surface_size = 2048; // Limit max size in case of computational errors.
							 // TODO - split long polygons during export to avoid reducing size for such polygons.
		let tc_min_int = [tc_min[0].floor() as i32, tc_min[1].floor() as i32];
		let tc_max_int = [tc_max[0].ceil() as i32, tc_max[1].ceil() as i32];
		let surface_size = [
			(tc_max_int[0] - tc_min_int[0]).max(1).min(max_surface_size),
			(tc_max_int[1] - tc_min_int[1]).max(1).min(max_surface_size),
		];

		let surface_pixels_offset = *surfaces_pixels_accumulated_offset;
		*surfaces_pixels_accumulated_offset += (surface_size[0] * surface_size[1]) as usize;

		polygon_data.visible_frame = self.current_frame;
		polygon_data.depth_equation = depth_equation;
		polygon_data.tex_coord_equation = tc_equation_scaled;
		polygon_data.surface_pixels_offset = surface_pixels_offset;
		polygon_data.surface_size = [surface_size[0] as u32, surface_size[1] as u32];
		polygon_data.mip = mip;
		polygon_data.surface_tc_min = tc_min_int;

		// Correct texture coordinates equation to compensate shift to surface rect.
		for i in 0 .. 2
		{
			let tc_min = tc_min_int[i] as f32;
			polygon_data.tex_coord_equation.d_tc_dx[i] -= tc_min * depth_equation.d_inv_z_dx;
			polygon_data.tex_coord_equation.d_tc_dy[i] -= tc_min * depth_equation.d_inv_z_dy;
			polygon_data.tex_coord_equation.k[i] -= tc_min * depth_equation.k;
		}
	}

	fn build_polygons_surfaces(&mut self)
	{
		// TODO - avoid iteration over all map polygons.
		// Remember (somehow) list of visible in current frame polygons.
		for i in 0 .. self.polygons_data.len()
		{
			if self.polygons_data[i].visible_frame == self.current_frame
			{
				self.build_polygon_surface(i);
			}
		}
	}

	fn build_polygon_surface(&mut self, polygon_index: usize)
	{
		let polygon = &self.map.polygons[polygon_index];
		let polygon_data = &self.polygons_data[polygon_index];
		let mip_texture = &self.textures[polygon.texture as usize][polygon_data.mip as usize];
		let surface_pixels_offset = polygon_data.surface_pixels_offset;
		let surface_size = polygon_data.surface_size;
		let surface_tc_min = polygon_data.surface_tc_min;

		for dst_y in 0 .. surface_size[1]
		{
			let dst_line_start = surface_pixels_offset + ((dst_y * surface_size[0]) as usize);
			let dst_line = &mut self.surfaces_pixels[dst_line_start .. dst_line_start + (surface_size[0] as usize)];

			let src_y = (surface_tc_min[1] + (dst_y as i32)).rem_euclid(mip_texture.size[1] as i32);
			let src_line_start = ((src_y as u32) * mip_texture.size[0]) as usize;
			let src_line = &mip_texture.pixels[src_line_start .. src_line_start + (mip_texture.size[0] as usize)];
			let mut src_x = surface_tc_min[0].rem_euclid(mip_texture.size[0] as i32);
			for dst_x in 0 .. surface_size[0]
			{
				dst_line[dst_x as usize] = src_line[src_x as usize];
				src_x += 1;
				if src_x == (mip_texture.size[0] as i32)
				{
					src_x = 0;
				}
			}
		}
	}

	fn draw_tree_r(
		&self,
		rasterizer: &mut Rasterizer,
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
		current_index: u32,
	)
	{
		if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
			let leaf_data = &self.leafs_data[leaf as usize];
			if leaf_data.visible_frame == self.current_frame
			{
				self.draw_leaf(
					rasterizer,
					camera_matrices,
					&leaf_data.current_frame_bounds,
					inline_models_index,
					leaf,
				);
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
					inline_models_index,
					node.children[(i ^ mask) as usize],
				);
			}
		}
	}

	fn draw_leaf(
		&self,
		rasterizer: &mut Rasterizer,
		camera_matrices: &CameraMatrices,
		bounds: &ClippingPolygon,
		inline_models_index: &InlineModelsIndex,
		leaf_index: u32,
	)
	{
		let leaf = &self.map.leafs[leaf_index as usize];

		// TODO - maybe just a little bit extend clipping polygon?
		let clip_planes = bounds.get_clip_planes();

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
				&self.surfaces_pixels[polygon_data.surface_pixels_offset ..
					polygon_data.surface_pixels_offset +
						((polygon_data.surface_size[0] * polygon_data.surface_size[1]) as usize)],
			);
		}

		// Draw models, located in this leaf, after leaf polygons.
		// TODO - sort leaf models.
		for &model_index in inline_models_index.get_leaf_models(leaf_index)
		{
			let submodel = &self.map.submodels[model_index as usize];
			for polygon_index in submodel.first_polygon .. (submodel.first_polygon + submodel.num_polygons)
			{
				self.draw_model_polygon(rasterizer, camera_matrices, &clip_planes, leaf_index, polygon_index);
			}
		}
	}

	fn draw_model_polygon(
		&self,
		rasterizer: &mut Rasterizer,
		camera_matrices: &CameraMatrices,
		clip_planes: &ClippingPolygonPlanes,
		leaf_index: u32,
		polygon_index: u32,
	)
	{
		let leaf = &self.map.leafs[leaf_index as usize];

		let polygon = &self.map.polygons[polygon_index as usize];
		let polygon_data = &self.polygons_data[polygon_index as usize];
		if polygon_data.visible_frame != self.current_frame
		{
			return;
		}

		let mut vertices_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory.
		let mut vertex_count = std::cmp::min(polygon.num_vertices as usize, MAX_VERTICES);

		vertices_clipped[.. vertex_count].copy_from_slice(
			&self.map.vertices[(polygon.first_vertex as usize) .. (polygon.first_vertex as usize) + vertex_count],
		);

		let mut vertices_temp = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory.

		// Clip model polygon by portal planes of current leaf.
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
				Plane {
					vec: -portal.plane.vec,
					dist: -portal.plane.dist,
				}
			};

			vertex_count =
				clip_3d_polygon_by_plane(&vertices_clipped[.. vertex_count], &clip_plane, &mut vertices_temp[..]);
			if vertex_count < 3
			{
				return;
			}
			vertices_clipped[.. vertex_count].copy_from_slice(&vertices_temp[.. vertex_count]);
		}

		// Clip model also by polygons of current leaf.
		for clip_polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
		{
			let clip_polygon = &self.map.polygons[clip_polygon_index as usize];

			vertex_count = clip_3d_polygon_by_plane(
				&vertices_clipped[.. vertex_count],
				&clip_polygon.plane,
				&mut vertices_temp[..],
			);
			if vertex_count < 3
			{
				return;
			}
			vertices_clipped[.. vertex_count].copy_from_slice(&vertices_temp[.. vertex_count]);
		}

		// Transform vetices after clipping.
		// TODO - perform clipping using transformed planes instead.
		for v in &mut vertices_clipped[.. vertex_count]
		{
			let vertex_transformed = camera_matrices.view_matrix * v.extend(1.0);
			*v = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
		}

		draw_polygon(
			rasterizer,
			&clip_planes,
			&vertices_clipped[.. vertex_count],
			&polygon_data.depth_equation,
			&polygon_data.tex_coord_equation,
			&polygon_data.surface_size,
			&self.surfaces_pixels[polygon_data.surface_pixels_offset ..
				polygon_data.surface_pixels_offset +
					((polygon_data.surface_size[0] * polygon_data.surface_size[1]) as usize)],
		);
	}
}

fn draw_background(pixels: &mut [Color32])
{
	for pixel in pixels.iter_mut()
	{
		*pixel = Color32::from_rgb(32, 16, 8);
	}
}

fn draw_crosshair(pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
{
	pixels[surface_info.width / 2 + surface_info.height / 2 * surface_info.pitch] = Color32::from_rgb(255, 255, 255);
}

fn load_textures(in_textures: &[bsp_map_compact::Texture]) -> Vec<TextureWithMips>
{
	let textures_dir = std::path::PathBuf::from("textures");
	let extension = ".tga";

	let mut result = Vec::new();

	for texture_name in in_textures
	{
		let null_pos = texture_name
			.iter()
			.position(|x| *x == 0_u8)
			.unwrap_or(texture_name.len());
		let range = &texture_name[0 .. null_pos];

		let texture_name_string = std::str::from_utf8(range).unwrap_or("").to_string();
		let texture_name_with_extension = texture_name_string + extension;

		let mut texture_path = textures_dir.clone();
		texture_path.push(texture_name_with_extension);

		let mip0 = if let Some(image) = image::load(&texture_path)
		{
			image
		}
		else
		{
			println!("Failed to load texture {:?}", texture_path);
			make_stub_texture()
		};

		result.push(build_mips(mip0));
	}

	result
}

fn make_stub_texture() -> image::Image
{
	let mut result = image::Image {
		size: [16, 16],
		pixels: vec![Color32::from_rgb(255, 0, 255); 16 * 16],
	};

	for y in 0 .. result.size[1]
	{
		for x in 0 .. result.size[0]
		{
			let color = if (((x >> 3) ^ (y >> 3)) & 1) != 0
			{
				Color32::from_rgb(255, 0, 255)
			}
			else
			{
				Color32::from_rgb(128, 128, 128)
			};
			result.pixels[(x + y * result.size[0]) as usize] = color;
		}
	}

	result
}

fn build_mips(mip0: image::Image) -> TextureWithMips
{
	// This function requires input texture with size multiple of ( 1 << MAX_MIP ).

	let mut result = TextureWithMips::default();

	result[0] = mip0;
	for i in 1 .. NUM_MIPS
	{
		let prev_mip = &mut result[i - 1];
		let mut mip = image::Image {
			size: [prev_mip.size[0] >> 1, prev_mip.size[1] >> 1],
			pixels: Vec::new(),
		};

		mip.pixels = vec![Color32::from_rgb(0, 0, 0); (mip.size[0] * mip.size[1]) as usize];
		for y in 0 .. mip.size[1] as usize
		{
			for x in 0 .. mip.size[0] as usize
			{
				mip.pixels[x + y * (mip.size[0] as usize)] = Color32::get_average_4([
					prev_mip.pixels[(x * 2) + (y * 2) * (prev_mip.size[0] as usize)],
					prev_mip.pixels[(x * 2) + (y * 2 + 1) * (prev_mip.size[0] as usize)],
					prev_mip.pixels[(x * 2 + 1) + (y * 2) * (prev_mip.size[0] as usize)],
					prev_mip.pixels[(x * 2 + 1) + (y * 2 + 1) * (prev_mip.size[0] as usize)],
				]);
			}
		}
		result[i] = mip;
	}

	result
}

fn precalculate_polygons_tex_coords_bounds(map: &bsp_map_compact::BSPMap, polygons_data: &mut [DrawPolygonData])
{
	for (polygon, polygon_data) in map.polygons.iter().zip(polygons_data.iter_mut())
	{
		let inf = (1 << 29) as f32;
		let mut tc_min = [inf, inf];
		let mut tc_max = [-inf, -inf];

		for vertex in
			&map.vertices[(polygon.first_vertex as usize) .. ((polygon.first_vertex + polygon.num_vertices) as usize)]
		{
			for i in 0 .. 2
			{
				let tc = polygon.tex_coord_equation[i].vec.dot(*vertex) + polygon.tex_coord_equation[i].dist;
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

		// Limit result by very large values that still fits inside integer.
		// Thisi is needed to avoid integer overflows if something is wrong with texture coordinates.
		for i in 0 .. 2
		{
			if tc_min[i] < -inf
			{
				tc_min[i] = inf;
			}
			if tc_max[i] > inf
			{
				tc_max[i] = inf;
			}
		}

		polygon_data.tc_min = tc_min;
		polygon_data.tc_max = tc_max;
	}
}

// TODO - get rid of debug code.
#[derive(Default)]
struct DebugStats
{
	num_reachable_leafs_search_calls: usize,
	reachable_leafs_search_calls_depth: usize,
	reachable_leafs_search_max_wave_size: usize,
}

fn mark_reachable_leafs_recursive(
	leaf: u32,
	map: &bsp_map_compact::BSPMap,
	current_frame: FrameNumber,
	camera_matrices: &CameraMatrices,
	depth: u32,
	bounds: &ClippingPolygon,
	leafs_data: &mut [DrawLeafData],
	portals_data: &mut [DrawPortalData],
	debug_stats: &mut DebugStats,
)
{
	debug_stats.num_reachable_leafs_search_calls += 1;

	let max_depth = 1024; // Prevent stack overflow in case of broken graph.
	if depth > max_depth
	{
		return;
	}

	let leaf_data = &mut leafs_data[leaf as usize];

	if leaf_data.visible_frame != current_frame
	{
		leaf_data.visible_frame = current_frame;
		leaf_data.current_frame_bounds = *bounds;
		leaf_data.num_search_visits = 1;
	}
	else
	{
		leaf_data.num_search_visits += 1;

		// If we visit this leaf not first time, check if bounds is inside current.
		// If so - we can skip it.
		if leaf_data.current_frame_bounds.contains(bounds)
		{
			return;
		}
		// Perform clipping of portals of this leaf using combined bounds to ensure that we visit all possible paths with such bounds.
		leaf_data.current_frame_bounds.extend(bounds);
	}
	let bound_for_portals_clipping = leaf_data.current_frame_bounds;

	let leaf_value = map.leafs[leaf as usize];
	for portal in &map.leafs_portals[(leaf_value.first_leaf_portal as usize) ..
		((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
	{
		let portal_value = &map.portals[(*portal) as usize];

		// Do not look through portals that are facing from camera.
		let portal_plane_pos =
			(camera_matrices.planes_matrix * portal_value.plane.vec.extend(-portal_value.plane.dist)).w;

		let next_leaf;
		if portal_value.leafs[0] == leaf
		{
			if portal_plane_pos <= 0.0
			{
				continue;
			}
			next_leaf = portal_value.leafs[1];
		}
		else
		{
			if portal_plane_pos >= 0.0
			{
				continue;
			}
			next_leaf = portal_value.leafs[0];
		}

		// Same portal may be visited multiple times.
		// So, cache calculation of portal bounds.
		let portal_data = &mut portals_data[(*portal) as usize];
		if portal_data.visible_frame != current_frame
		{
			portal_data.visible_frame = current_frame;
			portal_data.current_frame_projection = project_portal(portal_value, map, &camera_matrices.view_matrix);
		}

		let mut bounds_intersection = if let Some(b) = portal_data.current_frame_projection
		{
			b
		}
		else
		{
			continue;
		};
		bounds_intersection.intersect(&bound_for_portals_clipping);
		if bounds_intersection.is_empty_or_invalid()
		{
			continue;
		}

		mark_reachable_leafs_recursive(
			next_leaf,
			map,
			current_frame,
			camera_matrices,
			depth + 1,
			&bounds_intersection,
			leafs_data,
			portals_data,
			debug_stats,
		);
	}
}

fn project_portal(
	portal: &bsp_map_compact::Portal,
	map: &bsp_map_compact::BSPMap,
	view_matrix: &Mat4f,
) -> Option<ClippingPolygon>
{
	const MAX_VERTICES: usize = 24;
	let mut vertex_count = std::cmp::min(portal.num_vertices as usize, MAX_VERTICES);

	// Perform initial matrix tranformation, obtain 3d vertices in camera-aligned space.
	let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	for (in_vertex, out_vertex) in map.vertices
		[(portal.first_vertex as usize) .. (portal.first_vertex as usize) + vertex_count]
		.iter()
		.zip(vertices_transformed.iter_mut())
	{
		let vertex_transformed = view_matrix * in_vertex.extend(1.0);
		*out_vertex = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
	}

	// Perform z_near clipping. Use very small z_near to avoid clipping portals.
	let mut vertices_transformed_z_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	const Z_NEAR: f32 = 1.0 / 4096.0;
	vertex_count = clip_3d_polygon_by_z_plane(
		&vertices_transformed[.. vertex_count],
		Z_NEAR,
		&mut vertices_transformed_z_clipped,
	);
	if vertex_count < 3
	{
		return None;
	}

	let mut portal_polygon_bounds = ClippingPolygon::from_point(
		&(vertices_transformed_z_clipped[0].truncate() / vertices_transformed_z_clipped[0].z),
	);
	for vertex_transformed in &vertices_transformed_z_clipped[1 .. vertex_count]
	{
		portal_polygon_bounds.extend_with_point(&(vertex_transformed.truncate() / vertex_transformed.z));
	}

	Some(portal_polygon_bounds)
}

fn draw_polygon(
	rasterizer: &mut Rasterizer,
	clip_planes: &ClippingPolygonPlanes,
	vertices_transformed: &[Vec3f],
	depth_equation: &DepthEquation,
	tex_coord_equation: &TexCoordEquation,
	texture_size: &[u32; 2],
	texture_data: &[Color32],
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
	for (index, vertex_2d) in vertices_2d.iter().enumerate().take(vertex_count)
	{
		vertices_for_rasterizer[index] = PolygonPointProjected {
			x: f32_to_fixed16(vertex_2d.x),
			y: f32_to_fixed16(vertex_2d.y),
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
		rasterizer.fill_polygon::<RasterizerSettingsAffine>(
			&vertices_for_rasterizer[0 .. vertex_count],
			&depth_equation,
			&tex_coord_equation,
			&texture_info,
			texture_data,
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
			rasterizer.fill_polygon::<RasterizerSettingsLineZCorrection>(
				&vertices_for_rasterizer[0 .. vertex_count],
				&depth_equation_scaled,
				&tex_coord_equation_scaled,
				&texture_info,
				texture_data,
			);
		}
		else
		{
			rasterizer.fill_polygon::<RasterizerSettingsFullPerspective>(
				&vertices_for_rasterizer[0 .. vertex_count],
				&depth_equation_scaled,
				&tex_coord_equation_scaled,
				&texture_info,
				texture_data,
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

const MAX_VERTICES: usize = 24;

// Returns number of result vertices. < 3 if polygon is clipped.
fn project_and_clip_polygon(
	clip_planes: &ClippingPolygonPlanes,
	vertices_transformed: &[Vec3f],
	out_vertices: &mut [Vec2f],
) -> usize
{
	let mut vertex_count = std::cmp::min(vertices_transformed.len(), MAX_VERTICES);

	// Perform z_near clipping.
	let mut vertices_transformed_z_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	const Z_NEAR: f32 = 1.0;
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

struct RasterizerSettingsFullPerspective;
impl RasterizerSettings for RasterizerSettingsFullPerspective
{
	const TEXTURE_COORDINATES_INTERPOLATION_MODE: TetureCoordinatesInterpolationMode =
		TetureCoordinatesInterpolationMode::FullPerspective;
}

struct RasterizerSettingsLineZCorrection;
impl RasterizerSettings for RasterizerSettingsLineZCorrection
{
	const TEXTURE_COORDINATES_INTERPOLATION_MODE: TetureCoordinatesInterpolationMode =
		TetureCoordinatesInterpolationMode::LineZCorrection;
}

struct RasterizerSettingsAffine;
impl RasterizerSettings for RasterizerSettingsAffine
{
	const TEXTURE_COORDINATES_INTERPOLATION_MODE: TetureCoordinatesInterpolationMode =
		TetureCoordinatesInterpolationMode::Affine;
}
