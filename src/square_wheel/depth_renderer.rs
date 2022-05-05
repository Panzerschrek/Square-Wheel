use super::{
	clipping_polygon::*,
	map_visibility_calculator::*,
	rasterizer::*,
	renderer::{project_and_clip_polygon, MAX_VERTICES},
};
use common::{bsp_map_compact, fixed_math::*, math_types::*, matrix::*};
use std::rc::Rc;

pub struct DepthRenderer
{
	map: Rc<bsp_map_compact::BSPMap>,
	visibility_calculator: MapVisibilityCalculator,
}

impl DepthRenderer
{
	pub fn new(map: Rc<bsp_map_compact::BSPMap>) -> Self
	{
		Self {
			visibility_calculator: MapVisibilityCalculator::new(map.clone()),
			map,
		}
	}

	pub fn draw_map(&mut self, pixels: &mut [f32], width: u32, height: u32, camera_matrices: &CameraMatrices)
	{
		let mut rasterizer = DepthRasterizer::new(pixels, width, height);
		let root_node = (self.map.nodes.len() - 1) as u32;

		// TODO - before preparing frame try to shift camera a little bit away from all planes of BSP nodes before current leaf.
		// This is needed to fix possible z_near clipping of current leaf portals.

		let frame_bounds = ClippingPolygon::from_box(0.0, 0.0, width as f32, height as f32);
		self.visibility_calculator
			.update_visibility(camera_matrices, &frame_bounds);

		// Draw BSP tree in back to front order, skip unreachable leafs.
		self.draw_tree_r(&mut rasterizer, camera_matrices, root_node);
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

		let viewport_half_size = [
			rasterizer.get_width() as f32 * 0.5,
			rasterizer.get_height() as f32 * 0.5,
		];
		for polygon_index in leaf.first_polygon .. (leaf.first_polygon + leaf.num_polygons)
		{
			let polygon = &self.map.polygons[polygon_index as usize];

			let plane_transformed = camera_matrices.planes_matrix * polygon.plane.vec.extend(-polygon.plane.dist);
			// Cull back faces.
			if plane_transformed.w <= 0.0
			{
				continue;
			}

			let plane_transformed_w = -plane_transformed.w;
			let d_inv_z_dx = plane_transformed.x / plane_transformed_w;
			let d_inv_z_dy = plane_transformed.y / plane_transformed_w;
			// Use depth bias in order to avoid self-shadowing.
			const DEPTH_BIAS_CONST: f32 = -1.0 / ((1 << 20) as f32);
			const DEPTH_BIAS_SLOPE: f32 = -1.0;
			let depth_equation = DepthEquation {
				d_inv_z_dx,
				d_inv_z_dy,
				k: plane_transformed.z / plane_transformed_w -
					d_inv_z_dx * viewport_half_size[0] -
					d_inv_z_dy * viewport_half_size[1] +
					DEPTH_BIAS_CONST + DEPTH_BIAS_SLOPE * (d_inv_z_dx.abs() + d_inv_z_dy.abs()),
			};

			let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory.
			let vertex_count = std::cmp::min(polygon.num_vertices as usize, MAX_VERTICES);
			for (in_vertex, out_vertex) in self.map.vertices
				[(polygon.first_vertex as usize) .. (polygon.first_vertex as usize) + vertex_count]
				.iter()
				.zip(vertices_transformed[.. vertex_count].iter_mut())
			{
				let vertex_transformed = camera_matrices.view_matrix * in_vertex.extend(1.0);
				*out_vertex = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
			}

			draw_depth_polygon(
				rasterizer,
				&clip_planes,
				&vertices_transformed[.. vertex_count],
				&depth_equation,
			);
		}
	}
}

fn draw_depth_polygon(
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

	rasterizer.fill_polygon(&vertices_for_rasterizer[0 .. vertex_count], &depth_equation);
}
