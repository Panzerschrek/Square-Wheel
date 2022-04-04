// This file contains declaration of compact BSP map representation.
// Such representation allows to process BSP map in order to draw it or in order to save or load it.

pub use super::map_polygonizer::Plane;

pub struct BSPMap
{
	// Node #0 is tree root.
	pub nodes: Vec<BSPNode>,
	pub leafs: Vec<BSPLeaf>,
	pub polygons: Vec<Polygon>,
	pub portals: Vec<Portal>,
	pub leaf_portals: Vec<u32>,
}

#[repr(C)]
pub struct BSPNode
{
	// If child index is greater or equal than FIRST_LEAF_INDEX - child is leaf.
	pub children: [u32; 2],
	pub plane : Plane,
}

pub const FIRST_LEAF_INDEX : u32 = 1 << 31;

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
	pub plane : Plane,
}

#[repr(C)]
pub struct Portal
{
	pub leafs: [u32; 2],
	pub first_vertex: u32,
	pub num_vertices: u32,
}
