use super::{map_polygonizer, math_types::*};

pub use map_polygonizer::{Plane, Polygon};

#[derive(Debug)]
pub struct BSPNode
{
	pub plane: Plane,
	pub children: [BSPNodeChild; 2],
}

#[derive(Debug)]
pub struct BSPLeaf
{
	pub polygons: Vec<Polygon>,
}

#[derive(Debug)]
pub enum BSPNodeChild
{
	NodeChild(Box<BSPNode>),
	LeafChild(BSPLeaf), // use "Box" here if "BSPLeaf" become too large.
}

pub type BSPTree = BSPNodeChild;

pub fn build_leaf_bsp_tree(entity: &map_polygonizer::Entity) -> BSPTree
{
	build_leaf_bsp_tree_r(entity.polygons.clone())
}

fn build_leaf_bsp_tree_r(mut in_polygons: Vec<Polygon>) -> BSPNodeChild
{
	if is_convex_set_of_polygons(&in_polygons)
	{
		return BSPNodeChild::LeafChild(BSPLeaf { polygons: in_polygons });
	}

	let splitter_plane = choose_best_splitter_plane(&in_polygons);

	let mut polygons_front = Vec::new();
	let mut polygons_back = Vec::new();
	for polygon in in_polygons.drain(..)
	{
		// TODO
	}

	// TODO
	BSPNodeChild::NodeChild(Box::new(BSPNode {
		plane: splitter_plane,
		children: [
			build_leaf_bsp_tree_r(polygons_front),
			build_leaf_bsp_tree_r(polygons_back),
		],
	}))
}

fn is_convex_set_of_polygons(polygons: &[Polygon]) -> bool
{
	if polygons.is_empty()
	{
		return true;
	}

	// TODO
	true
}

fn choose_best_splitter_plane(polygons: &[Polygon]) -> Plane
{
	for polygon in polygons
	{
		get_splitter_plane_score(polygons, &polygon.plane);
	}

	// TODO
	polygons[0].plane
}

fn get_splitter_plane_score(polygons: &[Polygon], plane: &Plane) -> f32
{
	// TODO
	for polygon in polygons
	{}

	0.0
}
