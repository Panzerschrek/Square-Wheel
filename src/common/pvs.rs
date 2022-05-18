use super::{bsp_map_compact, clipping, math_types::*, plane::*};

// List of visible BSP leafs tree for each leaf.
pub type LeafsVisibilityInfo = Vec<VisibleLeafsList>;
pub type VisibleLeafsList = Vec<u32>;

pub fn caclulate_pvs(map: &bsp_map_compact::BSPMap) -> LeafsVisibilityInfo
{
	let mut result = LeafsVisibilityInfo::with_capacity(map.leafs.len());
	for leaf_index in 0 .. map.leafs.len() as u32
	{
		result.push(calculate_pvs_for_leaf(map, leaf_index));
	}
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

	let portal_polygon = portal_polygon_from_map_portal(map, portal, portal.leafs[0] == leaf_index);

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

		let mut next_leaf_portal_polygon =
			portal_polygon_from_map_portal(map, next_leaf_portal, next_leaf_portal.leafs[0] == next_leaf_index);

		let portal_polygon_pos =
			get_portal_polygon_position_relative_plane(&next_leaf_portal_polygon, &portal_polygon.plane);
		if portal_polygon_pos == PortalPolygonPositionRelativePlane::Back ||
			portal_polygon_pos == PortalPolygonPositionRelativePlane::Coplanar
		{
			continue;
		}
		if portal_polygon_pos == PortalPolygonPositionRelativePlane::Splitted
		{
			next_leaf_portal_polygon = cut_portal_polygon_by_plane(&next_leaf_portal_polygon, &portal_polygon.plane);
			if next_leaf_portal_polygon.vertices.len() < 3
			{
				continue;
			}
		}

		mark_visible_leafs_r(
			map,
			&portal_polygon,
			&next_leaf_portal_polygon,
			next_leaf_portal_index,
			next_next_leaf_index,
			visible_leafs_bit_set,
		);
	}
}

#[derive(Clone)]
struct PortalPolygon
{
	plane: Plane,
	vertices: Vec<Vec3f>,
}

fn portal_polygon_from_map_portal(
	map: &bsp_map_compact::BSPMap,
	portal: &bsp_map_compact::Portal,
	invert: bool,
) -> PortalPolygon
{
	let mut portal_polygon = PortalPolygon {
		vertices: map.vertices[portal.first_vertex as usize .. (portal.first_vertex + portal.num_vertices) as usize]
			.iter()
			.cloned()
			.collect(),
		plane: portal.plane,
	};

	if invert
	{
		portal_polygon.plane = portal_polygon.plane.get_inverted();
		portal_polygon.vertices.reverse();
	}

	portal_polygon
}

fn mark_visible_leafs_r(
	map: &bsp_map_compact::BSPMap,
	start_portal_polygon: &PortalPolygon,
	prev_portal_polygon: &PortalPolygon,
	prev_portal_index: u32,
	leaf_index: u32,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
)
{
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

		let mut leaf_portal_polygon =
			portal_polygon_from_map_portal(map, leaf_portal, leaf_portal.leafs[0] == leaf_index);

		// Cut portal polygon using start portal plane.
		let portal_polygon_pos =
			get_portal_polygon_position_relative_plane(&leaf_portal_polygon, &start_portal_polygon.plane);
		if portal_polygon_pos == PortalPolygonPositionRelativePlane::Back ||
			portal_polygon_pos == PortalPolygonPositionRelativePlane::Coplanar
		{
			continue;
		}
		if portal_polygon_pos == PortalPolygonPositionRelativePlane::Splitted
		{
			leaf_portal_polygon = cut_portal_polygon_by_plane(&leaf_portal_polygon, &start_portal_polygon.plane);
			if leaf_portal_polygon.vertices.len() < 3
			{
				continue;
			}
		}

		// Cut portal polygon using prev portal plane
		let portal_polygon_pos =
			get_portal_polygon_position_relative_plane(&leaf_portal_polygon, &prev_portal_polygon.plane);
		if portal_polygon_pos == PortalPolygonPositionRelativePlane::Back ||
			portal_polygon_pos == PortalPolygonPositionRelativePlane::Coplanar
		{
			continue;
		}
		if portal_polygon_pos == PortalPolygonPositionRelativePlane::Splitted
		{
			leaf_portal_polygon = cut_portal_polygon_by_plane(&leaf_portal_polygon, &prev_portal_polygon.plane);
			if leaf_portal_polygon.vertices.len() < 3
			{
				continue;
			}
		}

		leaf_portal_polygon = cut_portal_polygon_by_view_through_two_previous_portals(
			start_portal_polygon,
			prev_portal_polygon,
			leaf_portal_polygon,
		);
		if leaf_portal_polygon.vertices.len() < 3
		{
			continue;
		}

		// TODO - cut also start polygon?

		mark_visible_leafs_r(
			map,
			start_portal_polygon,
			&leaf_portal_polygon,
			leaf_portal_index,
			next_leaf_index,
			visible_leafs_bit_set,
		);
	}
}

// Returns polygon with less than 3 vertices is completely clipped.
fn cut_portal_polygon_by_view_through_two_previous_portals(
	portal0: &PortalPolygon,
	portal1: &PortalPolygon,
	mut portal2: PortalPolygon,
) -> PortalPolygon
{
	// Check all combonation of planes, based on portal0 edge and portal1 vertex.
	// If portal0 is at back of this plane and portal1 is at front of this plane - perform portal2 clipping.
	let prev_edge_v = portal0.vertices.last().unwrap();
	for edge_v in &portal0.vertices
	{
		// TODO - check plane direction.
		let vec0 = edge_v - prev_edge_v;

		for v in &portal1.vertices
		{
			let vec1 = v - edge_v;
			let plane_vec = vec0.cross(vec1);
			let cut_plane = Plane {
				vec: plane_vec,
				dist: plane_vec.dot(*v),
			};

			if get_portal_polygon_position_relative_plane(portal0, &cut_plane) !=
				PortalPolygonPositionRelativePlane::Back
			{
				continue;
			}
			if get_portal_polygon_position_relative_plane(portal1, &cut_plane) !=
				PortalPolygonPositionRelativePlane::Front
			{
				continue;
			}

			portal2 = cut_portal_polygon_by_plane(&portal2, &cut_plane);
			if portal2.vertices.len() < 3
			{
				return portal2;
			}
		}
	}

	portal2
}

#[derive(PartialEq, Eq)]
enum PortalPolygonPositionRelativePlane
{
	Front,
	Back,
	Coplanar,
	Splitted,
}

fn get_portal_polygon_position_relative_plane(
	portal_polygon: &PortalPolygon,
	plane: &Plane,
) -> PortalPolygonPositionRelativePlane
{
	let mut vertices_front = 0;
	let mut vertices_back = 0;
	let normal_inv_len = 1.0 / plane.vec.magnitude();
	for v in &portal_polygon.vertices
	{
		let dist = (v.dot(plane.vec) - plane.dist) * normal_inv_len;
		if dist > PLANE_DIST_EPS
		{
			vertices_front += 1;
		}
		else if dist < -PLANE_DIST_EPS
		{
			vertices_back += 1;
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

// Returns polygon with less than 3 vertices is completely clipped.
fn cut_portal_polygon_by_plane(portal_polygon: &PortalPolygon, plane: &Plane) -> PortalPolygon
{
	let mut result = PortalPolygon {
		vertices: Vec::new(),
		plane: portal_polygon.plane,
	};

	let normal_inv_len = 1.0 / plane.vec.magnitude();

	let mut prev_v = portal_polygon.vertices.last().unwrap();
	let mut prev_dist = (plane.vec.dot(*prev_v) - plane.dist) * normal_inv_len;
	for v in &portal_polygon.vertices
	{
		let dist = (plane.vec.dot(*v) - plane.dist) * normal_inv_len;
		if dist > PLANE_DIST_EPS
		{
			if prev_dist < -PLANE_DIST_EPS
			{
				result
					.vertices
					.push(clipping::get_line_plane_intersection(prev_v, v, plane));
			}
			result.vertices.push(*v);
		}
		else if dist > -PLANE_DIST_EPS
		{
			result.vertices.push(*v);
		}
		else if prev_dist > PLANE_DIST_EPS
		{
			result
				.vertices
				.push(clipping::get_line_plane_intersection(prev_v, v, plane));
		}

		prev_v = v;
		prev_dist = dist;
	}

	result
}

const PLANE_DIST_EPS: f32 = 1.0 / 16.0;
