use super::{frame_number::*, renderer_utils::*};
use crate::common::{bsp_map_compact, clipping::*, clipping_polygon::*, math_types::*, matrix::*};
use std::sync::Arc;

pub struct MapVisibilityCalculator
{
	current_frame: FrameNumber,
	map: Arc<bsp_map_compact::BSPMap>,
	leafs_data: Vec<LeafData>,
	portals_data: Vec<PortalData>,
	is_inside_leaf_volume: bool,
}

#[derive(Default, Copy, Clone)]
struct LeafData
{
	// Frame last time this leaf was visible.
	visible_frame: FrameNumber,
	// Bounds combined from all paths through input portals.
	bounds: ClippingPolygon,
}

#[derive(Default, Copy, Clone)]
struct PortalData
{
	// Frame last time this portal was visible.
	visible_frame: FrameNumber,
	bounds: ClippingPolygon,
}

type LeafsSearchWaveElement = u32; // Leaf index
type LeafsSearchWave = Vec<LeafsSearchWaveElement>;
#[derive(Default)]
struct LeafsSearchWavesPair(LeafsSearchWave, LeafsSearchWave);

impl MapVisibilityCalculator
{
	pub fn new(map: Arc<bsp_map_compact::BSPMap>) -> Self
	{
		Self {
			current_frame: FrameNumber::default(),
			leafs_data: vec![LeafData::default(); map.leafs.len()],
			portals_data: vec![PortalData::default(); map.portals.len()],
			map,
			is_inside_leaf_volume: true,
		}
	}

	pub fn update_visibility(&mut self, camera_matrices: &CameraMatrices, frame_bounds: &ClippingPolygon)
	{
		self.current_frame.next();

		let current_leaf = self.find_current_leaf(camera_matrices);

		let current_leaf_data = &mut self.leafs_data[current_leaf as usize];
		current_leaf_data.visible_frame = self.current_frame;
		current_leaf_data.bounds = *frame_bounds;

		let leaf_value = self.map.leafs[current_leaf as usize];
		for &portal in &self.map.leafs_portals[leaf_value.first_leaf_portal as usize ..
			((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
		{
			let portal_value = &self.map.portals[portal as usize];

			let scaled_dist = portal_value.plane.vec.dot(camera_matrices.position) - portal_value.plane.dist;
			let eps = Z_NEAR * 2.0;
			if scaled_dist.abs() <= eps * portal_value.plane.vec.magnitude()
			{
				// Camera is too close to plane of this portal.
				// Assume, that leaf behind this portal is fully visible.
				let next_leaf = if portal_value.leafs[0] == current_leaf
				{
					portal_value.leafs[1]
				}
				else
				{
					portal_value.leafs[0]
				};

				let leaf_data = &mut self.leafs_data[next_leaf as usize];
				leaf_data.visible_frame = self.current_frame;
				leaf_data.bounds = *frame_bounds;
			}
		}

		self.update_visibility_impl(camera_matrices);

		self.is_inside_leaf_volume = self.is_inside_leaf_volume(camera_matrices, current_leaf);
	}

	// Use this method for portals or mirrors
	// - where camera position can be far away from actual visibility search start point (position of portal or mirror).
	pub fn update_visibility_with_start_leafs(
		&mut self,
		camera_matrices: &CameraMatrices,
		frame_bounds: &ClippingPolygon,
		start_leafs: &[u32],
	)
	{
		self.current_frame.next();

		for leaf in start_leafs
		{
			let leaf_data = &mut self.leafs_data[*leaf as usize];
			leaf_data.visible_frame = self.current_frame;
			leaf_data.bounds = *frame_bounds;
		}

		self.update_visibility_impl(camera_matrices);

		// Can't properly determine this.
		self.is_inside_leaf_volume = true;
	}

	pub fn get_current_frame_leaf_bounds(&self, leaf_index: u32) -> Option<ClippingPolygon>
	{
		let leaf_data = &self.leafs_data[leaf_index as usize];
		if leaf_data.visible_frame != self.current_frame
		{
			None
		}
		else
		{
			Some(leaf_data.bounds)
		}
	}

	pub fn is_current_camera_inside_leaf_volume(&self) -> bool
	{
		self.is_inside_leaf_volume
	}

	fn find_current_leaf(&self, camera_matrices: &CameraMatrices) -> u32
	{
		let mut index = bsp_map_compact::get_root_node_index(&self.map);
		let planes_matrix_w_row = camera_matrices.planes_matrix.row(3);
		loop
		{
			if index >= bsp_map_compact::FIRST_LEAF_INDEX
			{
				return index - bsp_map_compact::FIRST_LEAF_INDEX;
			}

			let node = &self.map.nodes[index as usize];
			let plane_transformed_w = planes_matrix_w_row.dot(node.plane.vec.extend(-node.plane.dist));
			index = if plane_transformed_w >= 0.0
			{
				node.children[0]
			}
			else
			{
				node.children[1]
			};
		}
	}

	fn update_visibility_impl(&mut self, camera_matrices: &CameraMatrices)
	{
		// Iterate over the whole BSP tree in front to back order.
		// For each leaf we calculate its bounds based on bounds of input portals (visited before).
		// Then we update bounds of output portals, which will be input portals of leafs visited later.
		//
		// This approach has guaranted linear time complexity (even in worst cases) proportional to size of the BSP tree.
		// TODO - find a way to optimize this even further and avoid visitng the whole BSP tree.

		let planes_matrix_w_row = camera_matrices.planes_matrix.row(3);

		// Use iterative approach of BSP tree traverse.
		// This is more effective way to do this,
		// because compiler can't properly optimize recursive calls and saves all arguments on stack,
		// which is unnecessary, since all arguments except one are the same.

		// Stack size must be greater or equal, than maximum BSP tree depth.
		const MAX_STACK_SIZE: usize = bsp_map_compact::MAX_BSP_TREE_DEPTH;
		let mut nodes_stack = [0; MAX_STACK_SIZE];
		nodes_stack[0] = bsp_map_compact::get_root_node_index(&self.map);
		let mut num_nodes_on_stack = 1;

		while num_nodes_on_stack > 0
		{
			let node_index = nodes_stack[num_nodes_on_stack - 1];
			if node_index >= bsp_map_compact::FIRST_LEAF_INDEX
			{
				// Leaf - pop current node from stack.
				num_nodes_on_stack -= 1;

				let leaf_index = node_index - bsp_map_compact::FIRST_LEAF_INDEX;

				let leaf_value = &self.map.leafs[leaf_index as usize];
				let leaf_data = &mut self.leafs_data[leaf_index as usize];

				let mut leaf_bounds_opt = if leaf_data.visible_frame == self.current_frame
				{
					Some(leaf_data.bounds)
				}
				else
				{
					None
				};

				// Visit all portals of this leaf.
				// If at least one portal is visible in this frame, assume this leaf to be visible.
				// Build clipping bounds as intersection of all portal clipping bounds.
				for &portal in &self.map.leafs_portals[(leaf_value.first_leaf_portal as usize) ..
					((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
				{
					let portal_data = &self.portals_data[portal as usize];

					if portal_data.visible_frame == self.current_frame
					{
						if let Some(leaf_bounds) = &mut leaf_bounds_opt
						{
							leaf_bounds.extend(&portal_data.bounds)
						}
						else
						{
							leaf_bounds_opt = Some(portal_data.bounds);
						}
					}
				}

				if let Some(leaf_bounds) = &leaf_bounds_opt
				{
					leaf_data.visible_frame = self.current_frame;
					leaf_data.bounds = *leaf_bounds;

					// Iterate over all out portals of the leaf.
					// If it's an optput portal (with no bounds set yet) - calculate its bounds as intersection of its own polygon bounds and input leaf bounds.
					for &portal in &self.map.leafs_portals[(leaf_value.first_leaf_portal as usize) ..
						((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
					{
						let portal_value = &self.map.portals[portal as usize];
						let portal_data = &mut self.portals_data[portal as usize];

						if portal_data.visible_frame != self.current_frame
						{
							if let Some(mut portal_projection) =
								project_portal(portal_value, &self.map, &camera_matrices.view_matrix)
							{
								portal_projection.intersect(leaf_bounds);
								if portal_projection.is_valid_and_non_empty()
								{
									portal_data.visible_frame = self.current_frame;
									portal_data.bounds = portal_projection;
								}
							}
						}
					}
				}
			}
			else if num_nodes_on_stack < MAX_STACK_SIZE
			{
				// Node - remove current node, push back and front nodes.

				let node = &self.map.nodes[node_index as usize];
				let plane_transformed_w = planes_matrix_w_row.dot(node.plane.vec.extend(-node.plane.dist));
				if plane_transformed_w >= 0.0
				{
					nodes_stack[num_nodes_on_stack - 1] = node.children[1];
					nodes_stack[num_nodes_on_stack] = node.children[0];
				}
				else
				{
					nodes_stack[num_nodes_on_stack - 1] = node.children[0];
					nodes_stack[num_nodes_on_stack] = node.children[1];
				}
				num_nodes_on_stack += 1;
			}
		}
	}

	fn is_inside_leaf_volume(&self, camera_matrices: &CameraMatrices, leaf_index: u32) -> bool
	{
		let leaf = &self.map.leafs[leaf_index as usize];
		for polygon in
			&self.map.polygons[leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize]
		{
			let plane_transformed = camera_matrices.planes_matrix * polygon.plane.vec.extend(-polygon.plane.dist);
			if plane_transformed.w < 0.0
			{
				return false;
			}
		}

		true
	}
}

fn project_portal(
	portal: &bsp_map_compact::Portal,
	map: &bsp_map_compact::BSPMap,
	view_matrix: &Mat4f,
) -> Option<ClippingPolygon>
{
	let mut vertex_count = std::cmp::min(portal.num_vertices as usize, MAX_VERTICES);

	// Perform initial matrix tranformation, obtain 3d vertices in camera-aligned space.
	let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	for (in_vertex, out_vertex) in map.vertices
		[(portal.first_vertex as usize) .. (portal.first_vertex as usize) + vertex_count]
		.iter()
		.zip(vertices_transformed.iter_mut())
	{
		*out_vertex = view_matrix_transform_vertex(view_matrix, in_vertex);
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
