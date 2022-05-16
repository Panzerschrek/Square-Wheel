use super::bsp_map_compact;

// List of visible BSP leafs tree for each leaf.
pub type LeafsVisibilityInfo = Vec<VisibleLeafsList>;
pub type VisibleLeafsList = Vec<u32>;

pub fn caclulate_pvs(map: &bsp_map_compact::BSPMap) -> LeafsVisibilityInfo
{
	let mut result = LeafsVisibilityInfo::with_capacity(map.leafs.len());
	for leaf in &map.leafs
	{
		result.push(calculate_pvs_for_leaf(map, leaf));
	}
	result
}

fn calculate_pvs_for_leaf(map: &bsp_map_compact::BSPMap, leaf: &bsp_map_compact::BSPLeaf) -> VisibleLeafsList
{
	let mut visible_leafs_bit_set = vec![false; map.leafs.len()];

	for portal in
		&map.portals[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
	{
		calculate_pvs_for_leaf_portal(map, leaf, portal, &mut visible_leafs_bit_set)
	}

	visible_leafs_bit_set_to_leafs_list(&visible_leafs_bit_set)
}

// TODO - use more advanced collection, like bitset.
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
	leaf: &bsp_map_compact::BSPLeaf,
	portal: &bsp_map_compact::Portal,
	visible_leafs_bit_set: &mut VisibleLeafsBitSet,
)
{
}
