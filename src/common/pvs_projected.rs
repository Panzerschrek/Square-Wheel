use super::{bsp_map_compact, clipping::*, clipping_polygon::*, math_types::*, matrix, plane::*};

// List of visible BSP leafs tree for each leaf.
pub type LeafsVisibilityInfo = Vec<VisibleLeafsList>;
pub type VisibleLeafsList = Vec<u32>;

// leafs.len() * leafs.len() elements.
// TODO - use more compact form.
pub type VisibilityMatrix = Vec<bool>;

pub fn caclulate_pvs(map: &bsp_map_compact::BSPMap) -> LeafsVisibilityInfo
{
	let mut result = LeafsVisibilityInfo::with_capacity(map.leafs.len());
	for leaf_index in 0 .. map.leafs.len() as u32
	{
		result.push(calculate_pvs_for_leaf(map, leaf_index));

		let ratio_before = leaf_index * 256 / (map.leafs.len() as u32);
		let ratio_after = (leaf_index + 1) * 256 / (map.leafs.len() as u32);
		if ratio_after != ratio_before
		{
			print!(
				"\r{:03.2}% complete ({} of {} leafs)",
				((leaf_index + 1) as f32) * 100.0 / (map.leafs.len() as f32),
				leaf_index + 1,
				map.leafs.len()
			);
		}
	}
	println!("\nDone!");
	result
}

pub fn calculate_visibility_matrix(map: &bsp_map_compact::BSPMap) -> VisibilityMatrix
{
	let mut mat = vec![false; map.leafs.len() * map.leafs.len()];

	let mut bit_sets = Vec::with_capacity(map.leafs.len());
	for leaf_index in 0 .. map.leafs.len() as u32
	{
		bit_sets.push(calculate_pvs_bit_set_for_leaf(map, leaf_index));

		let ratio_before = leaf_index * 256 / (map.leafs.len() as u32);
		let ratio_after = (leaf_index + 1) * 256 / (map.leafs.len() as u32);
		if ratio_after != ratio_before
		{
			print!(
				"\r{:03.2}% complete ({} of {} leafs)",
				((leaf_index + 1) as f32) * 100.0 / (map.leafs.len() as f32),
				leaf_index + 1,
				map.leafs.len()
			);
		}
	}

	println!("\nCaclulating final visibility matrix");
	for x in 0 .. map.leafs.len()
	{
		for y in 0 .. map.leafs.len()
		{
			// Set visibility to "true" only if visibility is "true" in both directions.
			// Such approach allows to reject some false-positives.
			mat[x + y * map.leafs.len()] = bit_sets[x][y] & bit_sets[y][x];
		}
	}

	let mut num_non_zero_visibility = 0;
	for &v in &mat
	{
		if v
		{
			num_non_zero_visibility += 1;
		}
	}
	println!("Done!");
	println!(
		"Average visibility {}% ({} leafs)",
		100.0 * (num_non_zero_visibility as f32) / (mat.len() as f32),
		num_non_zero_visibility / map.leafs.len()
	);

	mat
}

pub fn calculate_pvs_for_leaf(map: &bsp_map_compact::BSPMap, leaf_index: u32) -> VisibleLeafsList
{
	visible_leafs_bit_set_to_leafs_list(&calculate_pvs_bit_set_for_leaf(map, leaf_index))
}

fn calculate_pvs_bit_set_for_leaf(map: &bsp_map_compact::BSPMap, leaf_index: u32) -> VisibleLeafsBitSet
{
	let mut visible_leafs_bit_set = vec![false; map.leafs.len()];

	visible_leafs_bit_set[leaf_index as usize] = true;

	let leaf = &map.leafs[leaf_index as usize];
	for &portal_index in
		&map.leafs_portals[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
	{
		calculate_pvs_for_leaf_portal(map, leaf_index, portal_index, &mut visible_leafs_bit_set);
	}

	visible_leafs_bit_set
}

// TODO - use more advanced collection.
type VisibleLeafsBitSet = Vec<bool>;

fn visible_leafs_bit_set_to_leafs_list(visible_leafs_bit_set: &VisibleLeafsBitSet) -> VisibleLeafsList
{
	let mut result = VisibleLeafsList::new();
	for (i, &visible) in visible_leafs_bit_set.iter().enumerate()
	{
		if visible
		{
			result.push(i as u32);
		}
	}
	result
}

#[derive(Default, Copy, Clone)]
struct VisLeafData
{
	bounds: Option<ClippingPolygon>,
	last_push_iteration: usize,
}

type SearchWaveElement = u32; // Leaf index.
type SearchWave = Vec<SearchWaveElement>;

fn calculate_pvs_for_leaf_portal(
	map: &bsp_map_compact::BSPMap,
	start_leaf_index: u32,
	start_portal_index: u32,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
)
{
	let portal = &map.portals[start_portal_index as usize];
	let next_leaf_index = if portal.leafs[0] == start_leaf_index
	{
		portal.leafs[1]
	}
	else
	{
		portal.leafs[0]
	};

	let mut portal_polygon = PortalPolygon {
		vertices: map.vertices[portal.first_vertex as usize .. (portal.first_vertex + portal.num_vertices) as usize]
			.iter()
			.cloned()
			.collect(),
		plane: portal.plane,
	};
	if portal.leafs[0] == start_leaf_index
	{
		portal_polygon.plane = portal_polygon.plane.get_inverted();
	}

	// Split portal polygon into smaller pieces and perform search individually for each piece.
	// This is needed to avoid big rate of false-positives for large portals.

	let mut portal_polygon_tesselated = Vec::new();
	split_portal_polygons_r(portal_polygon, &mut portal_polygon_tesselated, 0);

	for portal_polygon in &portal_polygon_tesselated
	{
		calculate_pvs_for_leaf_portal_polygon(map, portal_polygon, next_leaf_index, visible_leafs_bit_set);
	}
}

fn calculate_pvs_for_leaf_portal_polygon(
	map: &bsp_map_compact::BSPMap,
	portal_polygon: &PortalPolygon,
	next_leaf_index: u32,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
)
{
	// Use tricky projection-based algorithm.
	// For each leaf portal build set of projection matrices (for each vertex) facing away from leaf.
	// Calculate projection of portals using these matrices and calculate combined projected polygon from projected polygons for each matrix.
	// Perform boolean operations for projection polygons.
	//
	// Such approach may produce some false-positives, but it is pretty fast.
	// Some false-positive cases may be fixed by checking visibility in both directions.

	// TODO - split too big portals and perform search idividually for each portal part in order to decrease false-positive rate.

	let mut cur_wave = SearchWave::new();
	let mut next_wave = SearchWave::new();

	cur_wave.push(next_leaf_index);

	let mut vis_leafs_data = vec![VisLeafData::default(); map.leafs.len()];

	let inf = 1e36;
	vis_leafs_data[next_leaf_index as usize].bounds = Some(ClippingPolygon::from_box(-inf, -inf, inf, inf));

	let view_matrices = calculate_portal_polygon_view_matrices(portal_polygon);

	let max_itertions = 256;
	let mut num_iterations = 1;
	while !cur_wave.is_empty()
	{
		for &leaf_index in &cur_wave
		{
			let prev_leaf_bounds = vis_leafs_data[leaf_index as usize].bounds.unwrap();

			let leaf = &map.leafs[leaf_index as usize];
			for &portal_index in &map.leafs_portals
				[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
			{
				let portal = &map.portals[portal_index as usize];

				let next_leaf_index = if portal.leafs[0] == leaf_index
				{
					portal.leafs[1]
				}
				else
				{
					portal.leafs[0]
				};

				let mut portal_bounds = if let Some(b) = project_portal(map, portal, &view_matrices)
				{
					b
				}
				else
				{
					continue;
				};

				portal_bounds.intersect(&prev_leaf_bounds);
				if portal_bounds.is_empty_or_invalid()
				{
					continue;
				}

				let vis_leaf_data = &mut vis_leafs_data[next_leaf_index as usize];
				if let Some(prev_bounds) = &mut vis_leaf_data.bounds
				{
					if prev_bounds.contains(&portal_bounds)
					{
						continue;
					}
					prev_bounds.extend(&portal_bounds);
				}
				else
				{
					vis_leaf_data.bounds = Some(portal_bounds);
				}

				if vis_leaf_data.last_push_iteration < num_iterations
				{
					vis_leaf_data.last_push_iteration = num_iterations;
					next_wave.push(next_leaf_index);
				}
			} // for portals.
		} // for wave elements.

		cur_wave.clear();
		std::mem::swap(&mut cur_wave, &mut next_wave);

		num_iterations += 1;
		if num_iterations >= max_itertions
		{
			break;
		}
	} // For wave steps.

	for (index, vis_leaf_data) in vis_leafs_data.iter().enumerate()
	{
		if vis_leaf_data.bounds.is_some()
		{
			visible_leafs_bit_set[index] = true;
		}
	}
}

fn calculate_portal_polygon_view_matrices(portal_polygon: &PortalPolygon) -> Vec<Mat4f>
{
	let dir = portal_polygon.plane.vec;

	// Build set of projection matrices for each portal vertex. Camera looks in direction of portal plane normal.

	let azimuth = (-dir.x).atan2(dir.y);
	let elevation = dir.z.atan2((dir.x * dir.x + dir.y * dir.y).sqrt());

	let mut result = Vec::with_capacity(portal_polygon.vertices.len() as usize);
	for vertex in &portal_polygon.vertices
	{
		// It's not important what exact FOV and viewport size to use.
		let viewport_size = 1.0;
		let mat = matrix::build_view_matrix(
			*vertex,
			Rad(azimuth),
			Rad(elevation),
			std::f32::consts::PI * 0.5,
			viewport_size,
			viewport_size,
		);
		result.push(mat.view_matrix);
	}

	result
}

fn project_portal(
	map: &bsp_map_compact::BSPMap,
	portal: &bsp_map_compact::Portal,
	view_matrices: &[Mat4f],
) -> Option<ClippingPolygon>
{
	const MAX_VERTICES: usize = 24;
	let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	let mut vertices_transformed_z_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory

	let mut result: Option<ClippingPolygon> = None;
	for view_matrix in view_matrices
	{
		let mut vertex_count = std::cmp::min(portal.num_vertices as usize, MAX_VERTICES);

		// Perform initial matrix tranformation, obtain 3d vertices in camera-aligned space.
		for (in_vertex, out_vertex) in map.vertices
			[(portal.first_vertex as usize) .. (portal.first_vertex as usize) + vertex_count]
			.iter()
			.zip(vertices_transformed.iter_mut())
		{
			let vertex_transformed = view_matrix * in_vertex.extend(1.0);
			*out_vertex = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
		}

		// Perform z_near clipping. Use very small z_near to avoid clipping portals.
		const Z_NEAR: f32 = 1.0 / 4096.0;
		vertex_count = clip_3d_polygon_by_z_plane(
			&vertices_transformed[.. vertex_count],
			Z_NEAR,
			&mut vertices_transformed_z_clipped,
		);
		if vertex_count < 3
		{
			continue;
		}

		for vertex_transformed in &vertices_transformed_z_clipped[.. vertex_count]
		{
			let point = vertex_transformed.truncate() / vertex_transformed.z;
			if let Some(r) = &mut result
			{
				r.extend_with_point(&point);
			}
			else
			{
				result = Some(ClippingPolygon::from_point(&point));
			}
		}
	}

	result
}

struct PortalPolygon
{
	plane: Plane,
	vertices: Vec<Vec3f>,
}

fn split_portal_polygons_r(polygon: PortalPolygon, out_polygons: &mut Vec<PortalPolygon>, recursion_depth: usize)
{
	if polygon.vertices.len() < 3
	{
		return;
	}

	if recursion_depth > 16
	{
		out_polygons.push(polygon);
		return;
	}

	const MAX_SIZE: f32 = 64.0;

	let split_planes_vecs = [Vec3f::unit_x(), Vec3f::unit_y(), Vec3f::unit_z()];
	for vec in split_planes_vecs
	{
		let inf = 1e30;
		let mut coord_min = inf;
		let mut coord_max = -inf;

		for &v in &polygon.vertices
		{
			let coord = v.dot(vec);
			if coord < coord_min
			{
				coord_min = coord;
			}
			if coord > coord_max
			{
				coord_max = coord;
			}
		}

		let size = coord_max - coord_min;
		if size < MAX_SIZE
		{
			continue;
		}

		let split_plane = Plane {
			vec: vec,
			dist: (coord_min + coord_max) * 0.5,
		};
		let (polygon_front, polygon_back) = split_portal_polygon(&polygon, &split_plane);
		split_portal_polygons_r(polygon_front, out_polygons, recursion_depth + 1);
		split_portal_polygons_r(polygon_back, out_polygons, recursion_depth + 1);
	}

	out_polygons.push(polygon);
}

// Returns pair of front and back polygons.
fn split_portal_polygon(in_polygon: &PortalPolygon, plane: &Plane) -> (PortalPolygon, PortalPolygon)
{
	let mut polygon_front = PortalPolygon {
		plane: in_polygon.plane,
		vertices: Vec::new(),
	};
	let mut polygon_back = PortalPolygon {
		plane: in_polygon.plane,
		vertices: Vec::new(),
	};

	let inv_vec_len = 1.0 / plane.vec.magnitude();

	let mut prev_vert = in_polygon.vertices.last().unwrap();
	let mut prev_vert_dist = inv_vec_len * (prev_vert.dot(plane.vec) - plane.dist);
	for vert in &in_polygon.vertices
	{
		let vert_dist = inv_vec_len * (vert.dot(plane.vec) - plane.dist);

		if vert_dist > PLANE_DIST_EPS
		{
			if prev_vert_dist < -PLANE_DIST_EPS
			{
				let intersection = get_line_plane_intersection(prev_vert, vert, plane);
				polygon_back.vertices.push(intersection);
				polygon_front.vertices.push(intersection);
			}
			polygon_front.vertices.push(*vert);
		}
		else if vert_dist < -PLANE_DIST_EPS
		{
			if prev_vert_dist > PLANE_DIST_EPS
			{
				let intersection = get_line_plane_intersection(prev_vert, vert, plane);
				polygon_front.vertices.push(intersection);
				polygon_back.vertices.push(intersection);
			}
			polygon_back.vertices.push(*vert);
		}
		else
		{
			polygon_front.vertices.push(*vert);
			polygon_back.vertices.push(*vert);
		}

		prev_vert = vert;
		prev_vert_dist = vert_dist;
	}

	(polygon_front, polygon_back)
}

const PLANE_DIST_EPS: f32 = 1.0 / 16.0;
