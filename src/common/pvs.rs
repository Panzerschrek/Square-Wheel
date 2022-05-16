use super::{bsp_map_compact, math_types::*};

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

fn calculate_pvs_for_leaf(map: &bsp_map_compact::BSPMap, leaf_index: u32) -> VisibleLeafsList
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

	let portal_polygon = portal_polygon_from_map_portal(map, portal);

	let next_leaf = &map.leafs[next_leaf_index as usize];
	for &next_leaf_portal_index in &map.leafs_portals
		[next_leaf.first_leaf_portal as usize .. (next_leaf.first_leaf_portal + next_leaf.num_leaf_portals) as usize]
	{
		if next_leaf_portal_index == portal_index
		{
			continue;
		}
		let next_leaf_portal = &map.portals[next_leaf_portal_index as usize];

		let next_leaf_portal_polygon = portal_polygon_from_map_portal(map, next_leaf_portal);

		let next_next_leaf_index = if next_leaf_portal.leafs[0] == next_leaf_index
		{
			next_leaf_portal.leafs[1]
		}
		else
		{
			next_leaf_portal.leafs[0]
		};

		mark_visible_leafs_r(
			map,
			&portal_polygon,
			&next_leaf_portal_polygon,
			next_next_leaf_index,
			visible_leafs_bit_set,
		);
	}
}

// TODO - use more advanced struct.
type PortalPolygon = Vec<Vec3f>;

fn portal_polygon_from_map_portal(map: &bsp_map_compact::BSPMap, portal: &bsp_map_compact::Portal) -> PortalPolygon
{
	map.vertices[portal.first_vertex as usize .. (portal.first_vertex + portal.num_vertices) as usize]
		.iter()
		.cloned()
		.collect()
}

fn mark_visible_leafs_r(
	map: &bsp_map_compact::BSPMap,
	start_portal_polygon: &PortalPolygon,
	cur_portal_polygon: &PortalPolygon,
	leaf_index: u32,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
)
{
	// TODO
}
