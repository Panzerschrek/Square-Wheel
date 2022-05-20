use super::{bbox::*, bsp_map_compact, math_types::*, plane::*};

// List of visible BSP leafs tree for each leaf.
pub type LeafsVisibilityInfo = Vec<VisibleLeafsList>;
pub type VisibleLeafsList = Vec<u32>;

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

pub fn calculate_pvs_for_leaf(map: &bsp_map_compact::BSPMap, leaf_index: u32) -> VisibleLeafsList
{
	let mut vis_leafs_data = vec![VisLeafData::default(); map.leafs.len()];

	let leaf = &map.leafs[leaf_index as usize];
	for &portal_index in
		&map.leafs_portals[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
	{
		calculate_pvs_for_leaf_portal(map, leaf_index, portal_index, &mut vis_leafs_data)
	}

	make_leafs_list(&vis_leafs_data)
}

#[derive(Default, Copy, Clone)]
struct VisLeafData
{
	path_element: Option<VisPathElement>,
	last_push_iteration: usize,
}

type VisLeafsData = Vec<VisLeafData>;

#[derive(Copy, Clone)]
struct VisPathElement
{
	prev_vis_box: VisBox,
	vis_box: VisBox,
}

fn make_leafs_list(vis_leafs_data: &VisLeafsData) -> VisibleLeafsList
{
	let mut result = VisibleLeafsList::new();
	for (i, &vis_leaf_data) in vis_leafs_data.iter().enumerate()
	{
		if vis_leaf_data.path_element.is_some()
		{
			result.push(i as u32);
		}
	}
	result
}

fn calculate_pvs_for_leaf_portal(
	map: &bsp_map_compact::BSPMap,
	leaf_index: u32,
	portal_index: u32,
	vis_leafs_data: &mut VisLeafsData,
)
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

	let portal_box = vis_box_from_map_portal(map, portal);

	vis_leafs_data[next_leaf_index as usize].path_element = Some(VisPathElement {
		prev_vis_box: portal_box,
		vis_box: portal_box,
	});

	let next_leaf = &map.leafs[next_leaf_index as usize];
	for &next_leaf_portal_index in &map.leafs_portals
		[next_leaf.first_leaf_portal as usize .. (next_leaf.first_leaf_portal + next_leaf.num_leaf_portals) as usize]
	{
		if next_leaf_portal_index == portal_index
		{
			continue;
		}
		let next_leaf_portal = &map.portals[next_leaf_portal_index as usize];

		let next_next_leaf_index = if next_leaf_portal.leafs[0] == next_leaf_index
		{
			next_leaf_portal.leafs[1]
		}
		else
		{
			next_leaf_portal.leafs[0]
		};

		let next_leaf_portal_box = vis_box_from_map_portal(map, next_leaf_portal);

		mark_visible_leafs_iterative(
			map,
			&portal_box,
			&next_leaf_portal_box,
			next_next_leaf_index,
			vis_leafs_data,
		);
	}
}

type VisBox = BBox;

fn vis_box_from_map_portal(map: &bsp_map_compact::BSPMap, portal: &bsp_map_compact::Portal) -> VisBox
{
	let mut bbox: Option<VisBox> = None;
	for v in &map.vertices[portal.first_vertex as usize .. (portal.first_vertex + portal.num_vertices) as usize]
	{
		if let Some(b) = &mut bbox
		{
			b.extend_with_point(v);
		}
		else
		{
			bbox = Some(VisBox::from_point(v))
		}
	}

	bbox.unwrap()
}

type SearchWaveElement = u32; // Leaf index.
type SearchWave = Vec<SearchWaveElement>;

fn mark_visible_leafs_iterative(
	map: &bsp_map_compact::BSPMap,
	start_portal_box: &VisBox,
	start_leaf_portal_box: &VisBox,
	start_leaf_index: u32,
	vis_leafs_data: &mut VisLeafsData,
)
{
	let mut cur_wave = SearchWave::new();
	let mut next_wave = SearchWave::new();

	cur_wave.push(start_leaf_index);
	vis_leafs_data[start_leaf_index as usize].path_element = Some(VisPathElement {
		prev_vis_box: *start_portal_box,
		vis_box: *start_leaf_portal_box,
	});

	let max_itertions = 32;
	let mut num_iterations = 1;
	while !cur_wave.is_empty()
	{
		for &leaf_index in &cur_wave
		{
			let prev_prev_portal_box = vis_leafs_data[leaf_index as usize].path_element.unwrap().prev_vis_box;
			let prev_portal_box = vis_leafs_data[leaf_index as usize].path_element.unwrap().vis_box;

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

				let mut portal_box = vis_box_from_map_portal(map, portal);

				// Cut leaf portal using start portal and prev portal.
				portal_box = if let Some(b) =
					cut_vis_box_by_view_through_two_previous_boxes(start_portal_box, &prev_portal_box, portal_box)
				{
					b
				}
				else
				{
					continue;
				};

				// Cut leaf portal using two previous portals.
				portal_box = if let Some(b) =
					cut_vis_box_by_view_through_two_previous_boxes(&prev_prev_portal_box, &prev_portal_box, portal_box)
				{
					b
				}
				else
				{
					continue;
				};

				if cut_vis_box_by_view_through_two_previous_boxes(&portal_box, &prev_portal_box, *start_portal_box)
					.is_none() || cut_vis_box_by_view_through_two_previous_boxes(
					&portal_box,
					&prev_prev_portal_box,
					*start_portal_box,
				)
				.is_none() || cut_vis_box_by_view_through_two_previous_boxes(
					&portal_box,
					&prev_portal_box,
					prev_prev_portal_box,
				)
				.is_none()
				{
					continue;
				}

				let vis_leaf_data = &mut vis_leafs_data[next_leaf_index as usize];
				if let Some(path_element) = &mut vis_leaf_data.path_element
				{
					if path_element.prev_vis_box.contains(&prev_portal_box) &&
						path_element.vis_box.contains(&portal_box)
					{
						// No need to continue search - it can't find more visible leafs.
						continue;
					}
					path_element.prev_vis_box.extend(&prev_portal_box);
					path_element.vis_box = portal_box;
				}
				else
				{
					vis_leaf_data.path_element = Some(VisPathElement {
						prev_vis_box: prev_portal_box,
						vis_box: portal_box,
					});
				}

				if vis_leaf_data.last_push_iteration < num_iterations
				{
					vis_leaf_data.last_push_iteration = num_iterations;
					next_wave.push(next_leaf_index);
				}
			} // for portals.
		} // for wave elements

		cur_wave.clear();
		std::mem::swap(&mut cur_wave, &mut next_wave);

		num_iterations += 1;
		if num_iterations >= max_itertions
		{
			break;
		}
	} // For wave steps.

	for vis_leaf_data in vis_leafs_data
	{
		vis_leaf_data.last_push_iteration = 0;
	}
}

fn cut_vis_box_by_view_through_two_previous_boxes(box0: &VisBox, box1: &VisBox, mut box2: VisBox) -> Option<VisBox>
{
	// Check all combinations of planes, based on parallel edges of box0 and box1.
	// Use plane as cut plane, if box0 is behind it and box1 is at front of it.

	let edges0 = [
		[
			Vec3f::new(box0.min.x, box0.min.y, box0.min.z),
			Vec3f::new(box0.min.x, box0.min.y, box0.max.z),
		],
		[
			Vec3f::new(box0.min.x, box0.max.y, box0.min.z),
			Vec3f::new(box0.min.x, box0.max.y, box0.max.z),
		],
		[
			Vec3f::new(box0.max.x, box0.min.y, box0.min.z),
			Vec3f::new(box0.max.x, box0.min.y, box0.max.z),
		],
		[
			Vec3f::new(box0.max.x, box0.max.y, box0.min.z),
			Vec3f::new(box0.max.x, box0.max.y, box0.max.z),
		],
	];

	let edges1 = [
		[
			Vec3f::new(box1.min.x, box1.min.y, box1.min.z),
			Vec3f::new(box1.min.x, box1.min.y, box1.max.z),
		],
		[
			Vec3f::new(box1.min.x, box1.max.y, box1.min.z),
			Vec3f::new(box1.min.x, box1.max.y, box1.max.z),
		],
		[
			Vec3f::new(box1.max.x, box1.min.y, box1.min.z),
			Vec3f::new(box1.max.x, box1.min.y, box1.max.z),
		],
		[
			Vec3f::new(box1.max.x, box1.max.y, box1.min.z),
			Vec3f::new(box1.max.x, box1.max.y, box1.max.z),
		],
	];

	let box0_center = box0.get_center();

	for edge0 in &edges0
	{
		let vec0 = edge0[1] - edge0[0];

		for edge1 in &edges1
		{
			let vec1 = edge0[0] - edge1[0];
			let plane_vec = vec0.cross(vec1);
			if plane_vec.magnitude2() <= 0.000000000001
			{
				continue;
			}

			let mut cut_plane = Plane {
				vec: plane_vec,
				dist: plane_vec.dot(edge0[0]),
			};
			if cut_plane.vec.dot(box0_center) > cut_plane.dist
			{
				cut_plane = cut_plane.get_inverted();
			}

			let ok = get_vis_box_position_relative_plane(box0, &cut_plane) == PortalPolygonPositionRelativePlane::Back &&
				get_vis_box_position_relative_plane(box1, &cut_plane) == PortalPolygonPositionRelativePlane::Front;
			if !ok
			{
				continue;
			}

			if let Some(b) = cut_vis_box_by_plane(&box2, &cut_plane)
			{
				box2 = b;
			}
			else
			{
				// Totally clipped.
				return None;
			}
		} // for edge1
	} // for edge0

	Some(box2)
}

#[derive(PartialEq, Eq)]
enum PortalPolygonPositionRelativePlane
{
	Front,
	Back,
	Coplanar,
	Splitted,
}

fn get_vis_box_position_relative_plane(vis_box: &VisBox, plane: &Plane) -> PortalPolygonPositionRelativePlane
{
	let mut vertices_front = 0;
	let mut vertices_back = 0;
	let normal_inv_len = 1.0 / plane.vec.magnitude();
	for i in 0 .. 2
	{
		let x = if i == 0 { vis_box.min.x } else { vis_box.max.x };
		for j in 0 .. 2
		{
			let y = if j == 0 { vis_box.min.y } else { vis_box.max.y };
			for k in 0 .. 2
			{
				let z = if k == 0 { vis_box.min.z } else { vis_box.max.z };
				let dist = (Vec3f::new(x, y, z).dot(plane.vec) - plane.dist) * normal_inv_len;
				if dist > PLANE_DIST_EPS
				{
					vertices_front += 1;
				}
				else if dist < -PLANE_DIST_EPS
				{
					vertices_back += 1;
				}
			}
		}
	}

	if vertices_front != 0 && vertices_back != 0
	{
		PortalPolygonPositionRelativePlane::Splitted
	}
	else if vertices_front != 0
	{
		PortalPolygonPositionRelativePlane::Front
	}
	else if vertices_back != 0
	{
		PortalPolygonPositionRelativePlane::Back
	}
	else
	{
		PortalPolygonPositionRelativePlane::Coplanar
	}
}

fn cut_vis_box_by_plane(vis_box: &VisBox, plane: &Plane) -> Option<VisBox>
{
	// TODO - check this
	let edges = [
		[
			Vec3f::new(vis_box.min.x, vis_box.min.y, vis_box.min.z),
			Vec3f::new(vis_box.min.x, vis_box.min.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.max.y, vis_box.min.z),
			Vec3f::new(vis_box.min.x, vis_box.max.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.max.x, vis_box.min.y, vis_box.min.z),
			Vec3f::new(vis_box.max.x, vis_box.min.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.max.x, vis_box.max.y, vis_box.min.z),
			Vec3f::new(vis_box.max.x, vis_box.max.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.min.y, vis_box.min.z),
			Vec3f::new(vis_box.min.x, vis_box.max.y, vis_box.min.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.min.y, vis_box.max.z),
			Vec3f::new(vis_box.min.x, vis_box.max.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.max.x, vis_box.min.y, vis_box.min.z),
			Vec3f::new(vis_box.max.x, vis_box.max.y, vis_box.min.z),
		],
		[
			Vec3f::new(vis_box.max.x, vis_box.min.y, vis_box.max.z),
			Vec3f::new(vis_box.max.x, vis_box.max.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.min.y, vis_box.min.z),
			Vec3f::new(vis_box.max.x, vis_box.min.y, vis_box.min.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.min.y, vis_box.max.z),
			Vec3f::new(vis_box.max.x, vis_box.min.y, vis_box.max.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.max.y, vis_box.min.z),
			Vec3f::new(vis_box.max.x, vis_box.max.y, vis_box.min.z),
		],
		[
			Vec3f::new(vis_box.min.x, vis_box.max.y, vis_box.max.z),
			Vec3f::new(vis_box.max.x, vis_box.max.y, vis_box.max.z),
		],
	];

	let mut result: Option<VisBox> = None;

	for edge in &edges
	{
		if let Some(e) = cut_edge_by_plane(edge, plane)
		{
			if let Some(r) = &mut result
			{
				r.extend_with_point(&e[0]);
				r.extend_with_point(&e[1]);
			}
			else
			{
				let mut r = VisBox::from_point(&e[0]);
				r.extend_with_point(&e[1]);
				result = Some(r);
			}
		}
	}

	result
}

type Edge = [Vec3f; 2];

fn cut_edge_by_plane(edge: &Edge, plane: &Plane) -> Option<Edge>
{
	let dist0 = edge[0].dot(plane.vec) - plane.dist;
	let dist1 = edge[1].dot(plane.vec) - plane.dist;
	if dist0 >= 0.0 && dist1 >= 0.0
	{
		return Some(*edge);
	}
	if dist0 <= 0.0 && dist1 <= 0.0
	{
		return None;
	}

	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	let middle_point = edge[0] * k1 - edge[1] * k0;

	if dist0 >= 0.0
	{
		Some([edge[0], middle_point])
	}
	else
	{
		Some([middle_point, edge[1]])
	}
}

const PLANE_DIST_EPS: f32 = 1.0 / 16.0;
