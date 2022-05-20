use super::{bbox::*, bsp_map_compact, clipping, math_types::*, plane::*};

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
	let mut visible_leafs_bit_set = vec![false; map.leafs.len()];

	let leaf = &map.leafs[leaf_index as usize];
	for &portal_index in
		&map.leafs_portals[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
	{
		calculate_pvs_for_leaf_portal(map, leaf_index, portal_index, &mut visible_leafs_bit_set)
	}

	visible_leafs_bit_set_to_leafs_list(&visible_leafs_bit_set)
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

fn calculate_pvs_for_leaf_portal(
	map: &bsp_map_compact::BSPMap,
	leaf_index: u32,
	portal_index: u32,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
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

	// Leaf next to current is always visible.
	visible_leafs_bit_set[next_leaf_index as usize] = true;

	let portal_box = vis_box_from_map_portal(map, portal);

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

		mark_visible_leafs_r(
			map,
			&portal_box,
			&next_leaf_portal_box,
			next_leaf_portal_index,
			next_next_leaf_index,
			visible_leafs_bit_set,
			0,
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

fn mark_visible_leafs_r(
	map: &bsp_map_compact::BSPMap,
	start_portal_box: &VisBox,
	prev_portal_box: &VisBox,
	prev_portal_index: u32,
	leaf_index: u32,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
	recursion_depth: usize,
)
{
	if recursion_depth > 16
	{
		return;
	}

	visible_leafs_bit_set[leaf_index as usize] = true;

	let leaf = &map.leafs[leaf_index as usize];

	for &leaf_portal_index in
		&map.leafs_portals[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
	{
		if leaf_portal_index == prev_portal_index
		{
			continue;
		}
		let leaf_portal = &map.portals[leaf_portal_index as usize];

		let next_leaf_index = if leaf_portal.leafs[0] == leaf_index
		{
			leaf_portal.leafs[1]
		}
		else
		{
			leaf_portal.leafs[0]
		};

		let mut leaf_portal_box = vis_box_from_map_portal(map, leaf_portal);

		// Cut leaf portal using start portal and prev portal.
		leaf_portal_box = if let Some(b) =
			cut_view_box_by_view_through_two_previous_boxes(start_portal_box, prev_portal_box, leaf_portal_box, false)
		{
			b
		}
		else
		{
			continue;
		};

		// Cut start portal using leaf portal and prev portal.
		let start_box_clipped = if let Some(b) = cut_view_box_by_view_through_two_previous_boxes(
			&leaf_portal_box,
			prev_portal_box,
			start_portal_box.clone(),
			true,
		)
		{
			b
		}
		else
		{
			continue;
		};

		mark_visible_leafs_r(
			map,
			&start_box_clipped,
			&leaf_portal_box,
			leaf_portal_index,
			next_leaf_index,
			visible_leafs_bit_set,
			recursion_depth + 1,
		);
	}
}

fn cut_view_box_by_view_through_two_previous_boxes(
	portal0: &VisBox,
	portal1: &VisBox,
	mut portal2: VisBox,
	reverse: bool,
) -> Option<VisBox>
{
	// TODO
	Some(portal2)
}

#[derive(PartialEq, Eq)]
enum PortalPolygonPositionRelativePlane
{
	Front,
	Back,
	Coplanar,
	Splitted,
}

fn get_view_box_position_relative_plane(vis_box: &VisBox, plane: &Plane) -> PortalPolygonPositionRelativePlane
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
