use super::{bsp_builder, math_types::*};

// This file contains declaration of compact BSP map representation.
// Such representation allows to process BSP map in order to draw it or in order to save or load it.

pub use super::map_polygonizer::Plane;

#[derive(Default)]
pub struct BSPMap
{
	// Last node is tree root.
	pub nodes: Vec<BSPNode>,
	pub leafs: Vec<BSPLeaf>,
	pub polygons: Vec<Polygon>,
	pub portals: Vec<Portal>,
	pub leafs_portals: Vec<u32>,
	// Both polygon and portal vertices.
	pub vertices: Vec<Vec3f>,
}

#[repr(C)]
pub struct BSPNode
{
	// If child index is greater or equal than FIRST_LEAF_INDEX - child is leaf.
	pub children: [u32; 2],
	pub plane: Plane,
}

pub const FIRST_LEAF_INDEX: u32 = 1 << 31;

#[repr(C)]
pub struct BSPLeaf
{
	pub first_polygon: u32,
	pub num_polygons: u32,
	pub first_leaf_portal: u32,
	pub num_leaf_portals: u32,
}

#[repr(C)]
pub struct Polygon
{
	pub first_vertex: u32,
	pub num_vertices: u32,
	pub plane: Plane,
}

#[repr(C)]
pub struct Portal
{
	pub leafs: [u32; 2],
	pub first_vertex: u32,
	pub num_vertices: u32,
}

// Conversion functions.
//

pub fn convert_bsp_map_to_compact_format(bsp_tree: &bsp_builder::BSPTree) -> BSPMap
{
	let mut out_map = BSPMap::default();

	let mut portal_ptr_to_index_map = PortalPtrToIndexMap::new();
	convert_portals_to_compact_format(&bsp_tree.portals, &mut out_map, &mut portal_ptr_to_index_map);

	let mut leaf_ptr_to_index_map = LeafPtrToIndexMap::new();
	convert_node_child_to_compact_format(
		&bsp_tree.root,
		&portal_ptr_to_index_map,
		&mut out_map,
		&mut leaf_ptr_to_index_map,
	);

	fill_portals_leafs(&bsp_tree.portals, &leaf_ptr_to_index_map, &mut out_map);

	out_map
}

type PortalPtrToIndexMap = std::collections::HashMap<*const bsp_builder::LeafsPortal, u32>;

fn convert_portals_to_compact_format(
	portals: &[bsp_builder::LeafsPortalPtr],
	out_map: &mut BSPMap,
	portal_ptr_to_index_map: &mut PortalPtrToIndexMap,
)
{
	for portal_ptr in portals
	{
		let portal_index = out_map.portals.len() as u32;
		let portal = portal_ptr.borrow();
		let portal_raw_ptr = (&*portal) as *const bsp_builder::LeafsPortal;
		portal_ptr_to_index_map.insert(portal_raw_ptr, portal_index);

		let portal_converted = convert_portal_to_compact_format(portal_ptr, out_map);
		out_map.portals.push(portal_converted);
	}
}

fn convert_portal_to_compact_format(portal_ptr: &bsp_builder::LeafsPortalPtr, out_map: &mut BSPMap) -> Portal
{
	let portal = portal_ptr.borrow();
	let first_vertex = out_map.vertices.len() as u32;
	out_map.vertices.extend_from_slice(&portal.vertices);
	Portal {
		first_vertex,
		num_vertices: portal.vertices.len() as u32,
		// Fill leafs later.
		leafs: [0, 0],
	}
}

type LeafPtrToIndexMap = std::collections::HashMap<*const bsp_builder::BSPLeaf, u32>;

// Returns index of child.
fn convert_node_child_to_compact_format(
	node_child: &bsp_builder::BSPNodeChild,
	portal_ptr_to_index_map: &PortalPtrToIndexMap,
	out_map: &mut BSPMap,
	leaf_ptr_to_index_map: &mut LeafPtrToIndexMap,
) -> u32
{
	match node_child
	{
		bsp_builder::BSPNodeChild::NodeChild(node_ptr) =>
		{
			let node_index = out_map.nodes.len();
			let node_converted =
				convert_node_to_compact_format(node_ptr, portal_ptr_to_index_map, out_map, leaf_ptr_to_index_map);
			out_map.nodes.push(node_converted);
			node_index as u32
		},
		bsp_builder::BSPNodeChild::LeafChild(leaf_ptr) =>
		{
			let leaf_index = out_map.leafs.len();
			let leaf_converted = convert_leaf_to_compact_format(leaf_ptr, portal_ptr_to_index_map, out_map);
			out_map.leafs.push(leaf_converted);

			let leaf = leaf_ptr.borrow();
			let leaf_raw_ptr = (&*leaf) as *const bsp_builder::BSPLeaf;
			leaf_ptr_to_index_map.insert(leaf_raw_ptr, leaf_index as u32);

			(leaf_index as u32) + FIRST_LEAF_INDEX
		},
	}
}

fn convert_node_to_compact_format(
	node_ptr: &bsp_builder::BSPNodePtr,
	portal_ptr_to_index_map: &PortalPtrToIndexMap,
	out_map: &mut BSPMap,
	leaf_ptr_to_index_map: &mut LeafPtrToIndexMap,
) -> BSPNode
{
	let node = node_ptr.borrow();
	let child0 = convert_node_child_to_compact_format(
		&node.children[0],
		portal_ptr_to_index_map,
		out_map,
		leaf_ptr_to_index_map,
	);
	let child1 = convert_node_child_to_compact_format(
		&node.children[1],
		portal_ptr_to_index_map,
		out_map,
		leaf_ptr_to_index_map,
	);
	BSPNode {
		children: [child0, child1],
		plane: node.plane,
	}
}

fn convert_leaf_to_compact_format(
	leaf_ptr: &bsp_builder::BSPLeafPtr,
	portal_ptr_to_index_map: &PortalPtrToIndexMap,
	out_map: &mut BSPMap,
) -> BSPLeaf
{
	let leaf = leaf_ptr.borrow();

	let first_polygon = out_map.polygons.len() as u32;
	for polygon in &leaf.polygons
	{
		let polygon_converted = convert_polygon_to_compact_format(&polygon, out_map);
		out_map.polygons.push(polygon_converted);
	}

	let first_leaf_portal = out_map.leafs_portals.len() as u32;
	for portal_weak_ptr in &leaf.portals
	{
		let portal_ptr = portal_weak_ptr.upgrade().unwrap();
		let portal = portal_ptr.borrow();
		let portal_raw_ptr = (&*portal) as *const bsp_builder::LeafsPortal;
		let portal_index = portal_ptr_to_index_map.get(&portal_raw_ptr).unwrap();
		out_map.leafs_portals.push(*portal_index);
	}

	BSPLeaf {
		first_polygon,
		num_polygons: leaf.polygons.len() as u32,
		first_leaf_portal,
		num_leaf_portals: leaf.portals.len() as u32,
	}
}

fn convert_polygon_to_compact_format(polygon: &bsp_builder::Polygon, out_map: &mut BSPMap) -> Polygon
{
	let first_vertex = out_map.vertices.len() as u32;
	out_map.vertices.extend_from_slice(&polygon.vertices);
	Polygon {
		first_vertex,
		num_vertices: polygon.vertices.len() as u32,
		plane: polygon.plane,
	}
}

fn fill_portals_leafs(
	portals: &[bsp_builder::LeafsPortalPtr],
	leaf_to_index_map: &LeafPtrToIndexMap,
	out_map: &mut BSPMap,
)
{
	if portals.len() != out_map.portals.len()
	{
		panic!("Portal count mismatch!");
	}

	for (portal_index, out_portal) in out_map.portals.iter_mut().enumerate()
	{
		let portal_ptr = &portals[portal_index];
		let portal = portal_ptr.borrow();

		out_portal.leafs[0] = get_leaf_index(&portal.leaf_front, leaf_to_index_map);
		out_portal.leafs[1] = get_leaf_index(&portal.leaf_back, leaf_to_index_map);
	}
}

fn get_leaf_index(leaf_ptr: &bsp_builder::BSPLeafPtr, leaf_to_index_map: &LeafPtrToIndexMap) -> u32
{
	let leaf = leaf_ptr.borrow();
	let leaf_raw_ptr = (&*leaf) as *const bsp_builder::BSPLeaf;
	*leaf_to_index_map.get(&leaf_raw_ptr).unwrap()
}
