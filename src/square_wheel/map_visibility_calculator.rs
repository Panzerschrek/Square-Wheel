use super::{frame_number::*, renderer_utils::*};
use crate::common::{bsp_map_compact, clipping::*, clipping_polygon::*, math_types::*, matrix::*};
use std::sync::Arc;

pub struct MapVisibilityCalculator
{
	current_frame: FrameNumber,
	map: Arc<bsp_map_compact::BSPMap>,
	leafs_data: Vec<LeafData>,
	portals_data: Vec<PortalData>,
	leafs_search_waves: LeafsSearchWavesPair,
	is_inside_leaf_volume: bool,
}

#[derive(Default, Copy, Clone)]
struct LeafData
{
	// Frame last time this leaf was visible.
	visible_frame: FrameNumber,
	// Bounds, combined from all paths through portals.
	current_frame_bounds: ClippingPolygon,
}

#[derive(Default, Copy, Clone)]
struct PortalData
{
	// Frame last time this portal was visible.
	visible_frame: FrameNumber,
	// None if behind camera.
	current_frame_projection: Option<ClippingPolygon>,
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
			leafs_search_waves: LeafsSearchWavesPair::default(),
			map,
			is_inside_leaf_volume: true,
		}
	}

	pub fn update_visibility(&mut self, camera_matrices: &CameraMatrices, frame_bounds: &ClippingPolygon)
	{
		self.current_frame.next();
		let root_node = bsp_map_compact::get_root_node_index(&self.map);
		let current_leaf = self.find_current_leaf(root_node, &camera_matrices.planes_matrix);

		// Start search with current leaf and all adjusted leafs, that are too close to camera.
		// Doing so we prevent some artifacts when camera lies (almost) on portal plane.
		const MAX_START_LEAFS: usize = 32;
		let mut start_leafs = [0; MAX_START_LEAFS];
		start_leafs[0] = current_leaf;
		let mut num_start_leafs = 1;

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
				// Assume, that leaft behind this portal is fully visible.
				let next_leaf = if portal_value.leafs[0] == current_leaf
				{
					portal_value.leafs[1]
				}
				else
				{
					portal_value.leafs[0]
				};

				start_leafs[num_start_leafs] = next_leaf;
				num_start_leafs += 1;
				if num_start_leafs == MAX_START_LEAFS
				{
					break;
				}
			}
		}

		self.mark_reachable_leafs_iterative(&start_leafs[.. num_start_leafs], camera_matrices, frame_bounds);

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
		self.mark_reachable_leafs_iterative(start_leafs, camera_matrices, frame_bounds);

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
			Some(leaf_data.current_frame_bounds)
		}
	}

	pub fn is_current_camera_inside_leaf_volume(&self) -> bool
	{
		self.is_inside_leaf_volume
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
		start_leafs: &[u32],
		camera_matrices: &CameraMatrices,
		start_bounds: &ClippingPolygon,
	)
	{
		let cur_wave = &mut self.leafs_search_waves.0;
		let next_wave = &mut self.leafs_search_waves.1;

		cur_wave.clear();
		next_wave.clear();

		for &start_leaf in start_leafs
		{
			cur_wave.push(start_leaf);
			self.leafs_data[start_leaf as usize].current_frame_bounds = *start_bounds;
			self.leafs_data[start_leaf as usize].visible_frame = self.current_frame;
		}

		let mut depth = 0;
		while !cur_wave.is_empty()
		{
			for &leaf in cur_wave.iter()
			{
				let leaf_bounds = self.leafs_data[leaf as usize].current_frame_bounds;

				let leaf_value = self.map.leafs[leaf as usize];
				for &portal in &self.map.leafs_portals[(leaf_value.first_leaf_portal as usize) ..
					((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
				{
					let portal_value = &self.map.portals[portal as usize];

					// Do not look through portals that are facing from camera.
					let portal_plane_pos =
						(camera_matrices.planes_matrix * portal_value.plane.vec.extend(-portal_value.plane.dist)).w;

					let next_leaf = if portal_value.leafs[0] == leaf
					{
						if portal_plane_pos <= 0.0
						{
							continue;
						}
						portal_value.leafs[1]
					}
					else
					{
						if portal_plane_pos >= 0.0
						{
							continue;
						}
						portal_value.leafs[0]
					};

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
					}
					else
					{
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

			cur_wave.clear();
			std::mem::swap(cur_wave, next_wave);

			depth += 1;
			if depth > 1024
			{
				// Prevent infinite loop in case of broken graph.
				break;
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
