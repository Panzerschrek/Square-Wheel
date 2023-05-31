use super::{
	equations::*, inline_models_index::*, map_visibility_calculator::*, rasterizer::*, renderer_utils::*,
	resources_manager::*,
};
use crate::common::{bsp_map_compact, clipping_polygon::*, fixed_math::*, material::*, math_types::*, matrix::*};
use std::sync::Arc;

pub struct DepthRenderer
{
	map: Arc<bsp_map_compact::BSPMap>,
	visibility_calculator: MapVisibilityCalculator,
	// true - casts shadow, false - otherwice.
	materials_shadow_table: Vec<bool>,
}

impl DepthRenderer
{
	pub fn new(resources_manager: ResourcesManagerSharedPtr, map: Arc<bsp_map_compact::BSPMap>) -> Self
	{
		let mut r = resources_manager.lock().unwrap();
		let all_materials = r.get_materials();

		let materials_shadow_table = map
			.textures
			.iter()
			.map(|texture_name| {
				let material_name_string = bsp_map_compact::get_texture_string(texture_name);
				if let Some(material) = all_materials.get(material_name_string)
				{
					// Disable shadows if shadows are explicitely enabled in material settings or if some blending is enabled.
					material.shadow && material.blending_mode == BlendingMode::None
				}
				else
				{
					true
				}
			})
			.collect();

		Self {
			visibility_calculator: MapVisibilityCalculator::new(map.clone()),
			map,
			materials_shadow_table,
		}
	}

	pub fn draw_map(
		&mut self,
		pixels: &mut [f32],
		width: u32,
		height: u32,
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
	)
	{
		let mut rasterizer = DepthRasterizer::new(pixels, width, height);
		let root_node = bsp_map_compact::get_root_node_index(&self.map);

		// TODO - before preparing frame try to shift camera a little bit away from all planes of BSP nodes before current leaf.
		// This is needed to fix possible z_near clipping of current leaf portals.

		let frame_bounds = ClippingPolygon::from_box(0.0, 0.0, width as f32, height as f32);
		self.visibility_calculator
			.update_visibility(camera_matrices, &frame_bounds);

		// Draw BSP tree in back to front order, skip unreachable leafs.
		self.draw_tree_r(&mut rasterizer, camera_matrices, root_node);

		// Draw submodels atop of static map geometry (using depth test).
		self.draw_submodels(&mut rasterizer, camera_matrices, inline_models_index);
	}

	fn draw_tree_r(&self, rasterizer: &mut DepthRasterizer, camera_matrices: &CameraMatrices, current_index: u32)
	{
		if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
			if let Some(leaf_bounds) = self.visibility_calculator.get_current_frame_leaf_bounds(leaf)
			{
				self.draw_leaf(rasterizer, camera_matrices, &leaf_bounds, leaf);
			}
		}
		else
		{
			let node = &self.map.nodes[current_index as usize];
			let plane_transformed = camera_matrices.planes_matrix * node.plane.vec.extend(-node.plane.dist);
			let mask = if plane_transformed.w >= 0.0 { 1 } else { 0 };
			for i in 0 .. 2
			{
				self.draw_tree_r(rasterizer, camera_matrices, node.children[(i ^ mask) as usize]);
			}
		}
	}

	fn draw_leaf(
		&self,
		rasterizer: &mut DepthRasterizer,
		camera_matrices: &CameraMatrices,
		bounds: &ClippingPolygon,
		leaf_index: u32,
	)
	{
		let leaf = &self.map.leafs[leaf_index as usize];

		// TODO - maybe just a little bit extend clipping polygon?
		let clip_planes = bounds.get_clip_planes();

		for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
		{
			// Draw leaf polyfons without depth test since we use back to front order.
			self.draw_polygon::<false>(rasterizer, camera_matrices, &clip_planes, polygon_index);
		}
	}

	fn draw_submodels(
		&self,
		rasterizer: &mut DepthRasterizer,
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
	)
	{
		for submodel_index in 0 .. self.map.submodels.len() as u32
		{
			self.draw_submodel(rasterizer, camera_matrices, inline_models_index, submodel_index);
		}
	}

	fn draw_submodel(
		&self,
		rasterizer: &mut DepthRasterizer,
		camera_matrices: &CameraMatrices,
		inline_models_index: &InlineModelsIndex,
		submodel_index: u32,
	)
	{
		let model_matrix = if let Some(m) = inline_models_index.get_model_matrix(submodel_index)
		{
			m
		}
		else
		{
			return;
		};

		let mut bounds: Option<ClippingPolygon> = None;
		for &leaf_index in inline_models_index.get_model_leafs(submodel_index as u32)
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
			// This submodel is located in leafs invisible from current position.
			return;
		};

		let model_matrix_inverse = model_matrix.transpose().invert().unwrap();
		let submodel_camera_matrices = CameraMatrices {
			view_matrix: camera_matrices.view_matrix * model_matrix,
			planes_matrix: camera_matrices.planes_matrix * model_matrix_inverse,
			position: camera_matrices.position,
		};

		let submodel = &self.map.submodels[submodel_index as usize];
		for polygon_index in submodel.first_polygon .. submodel.first_polygon + submodel.num_polygons
		{
			// Draw with depth test.
			self.draw_polygon::<true>(rasterizer, &submodel_camera_matrices, &clip_planes, polygon_index);
		}
	}

	fn draw_polygon<const DEPTH_TEST: bool>(
		&self,
		rasterizer: &mut DepthRasterizer,
		camera_matrices: &CameraMatrices,
		clip_planes: &ClippingPolygonPlanes,
		polygon_index: u32,
	)
	{
		let polygon = &self.map.polygons[polygon_index as usize];
		if !self.materials_shadow_table[polygon.texture as usize]
		{
			return;
		}

		let plane_transformed = camera_matrices.planes_matrix * polygon.plane.vec.extend(-polygon.plane.dist);
		// Cull back faces.
		if plane_transformed.w <= 0.0
		{
			return;
		}

		let plane_transformed_w = -plane_transformed.w;
		let d_inv_z_dx = plane_transformed.x / plane_transformed_w;
		let d_inv_z_dy = plane_transformed.y / plane_transformed_w;
		// Use depth bias in order to avoid self-shadowing.
		const DEPTH_BIAS_CONST: f32 = -1.0 / ((1 << 20) as f32);
		const DEPTH_BIAS_SLOPE: f32 = -1.0;
		// Scale whole depth equation a bit in order to compensate depth calculation errors in surfaces preparation code.
		const DEPTH_EQUATION_SCALE: f32 = 1.0 - 1.0 / 1024.0;
		let depth_equation = DepthEquation {
			d_inv_z_dx: DEPTH_EQUATION_SCALE * d_inv_z_dx,
			d_inv_z_dy: DEPTH_EQUATION_SCALE * d_inv_z_dy,
			k: DEPTH_EQUATION_SCALE *
				(plane_transformed.z / plane_transformed_w +
					DEPTH_BIAS_CONST + DEPTH_BIAS_SLOPE * (d_inv_z_dx.abs() + d_inv_z_dy.abs())),
		};

		let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory.
		let vertex_count = std::cmp::min(polygon.num_vertices as usize, MAX_VERTICES);
		for (in_vertex, out_vertex) in bsp_map_compact::get_polygon_vertices(&self.map, polygon)
			.iter()
			.zip(vertices_transformed[.. vertex_count].iter_mut())
		{
			*out_vertex = view_matrix_transform_vertex(&camera_matrices.view_matrix, in_vertex);
		}

		draw_depth_polygon::<DEPTH_TEST>(
			rasterizer,
			clip_planes,
			&vertices_transformed[.. vertex_count],
			&depth_equation,
		);
	}
}

fn draw_depth_polygon<const DEPTH_TEST: bool>(
	rasterizer: &mut DepthRasterizer,
	clip_planes: &ClippingPolygonPlanes,
	vertices_transformed: &[Vec3f],
	depth_equation: &DepthEquation,
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

	// Perform f32 to Fixed16 conversion.
	let mut vertices_for_rasterizer = [PolygonPointProjected { x: 0, y: 0 }; MAX_VERTICES]; // TODO - use uninitialized memory
	for (index, vertex_2d) in vertices_2d.iter().enumerate().take(vertex_count)
	{
		vertices_for_rasterizer[index] = PolygonPointProjected {
			x: f32_to_fixed16(vertex_2d.x),
			y: f32_to_fixed16(vertex_2d.y),
		};
	}

	rasterizer.fill_polygon::<DEPTH_TEST>(&vertices_for_rasterizer[0 .. vertex_count], depth_equation);
}
