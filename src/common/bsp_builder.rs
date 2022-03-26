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
	let splitter_plane_opt = choose_best_splitter_plane(&in_polygons);
	if splitter_plane_opt.is_none()
	{
		// No splitter plane means this is a leaf.
		return BSPNodeChild::LeafChild(BSPLeaf { polygons: in_polygons });
	}
	let splitter_plane = splitter_plane_opt.unwrap();

	let mut polygons_front = Vec::new();
	let mut polygons_back = Vec::new();
	for polygon in in_polygons.drain(..)
	{
		match get_polygon_position_relative_plane(&polygon, &splitter_plane)
		{
			PolygonPositionRelativePlane::Front | PolygonPositionRelativePlane::CoplanarFront =>
			{
				polygons_front.push(polygon);
			},
			PolygonPositionRelativePlane::Back | PolygonPositionRelativePlane::CoplanarBack =>
			{
				polygons_back.push(polygon);
			},
			PolygonPositionRelativePlane::Splitted =>
			{
				// TODO - split this polygon.
			},
		}
	}

	// HACK! Somethhing went wrong and we processing leaf now.
	if polygons_front.is_empty()
	{
		return BSPNodeChild::LeafChild(BSPLeaf {
			polygons: polygons_back,
		});
	}
	if polygons_back.is_empty()
	{
		return BSPNodeChild::LeafChild(BSPLeaf {
			polygons: polygons_front,
		});
	}

	BSPNodeChild::NodeChild(Box::new(BSPNode {
		plane: splitter_plane,
		children: [
			build_leaf_bsp_tree_r(polygons_front),
			build_leaf_bsp_tree_r(polygons_back),
		],
	}))
}

// Returns None if can't find any situable splitter.
fn choose_best_splitter_plane(polygons: &[Polygon]) -> Option<Plane>
{
	let mut best_score_plane: Option<(f32, Plane)> = None;
	for polygon in polygons
	{
		if let Some(score) = get_splitter_plane_score(polygons, &polygon.plane)
		{
			if let Some((prev_score, _)) = best_score_plane
			{
				if score < prev_score
				{
					best_score_plane = Some((score, polygon.plane))
				}
			}
			else
			{
				best_score_plane = Some((score, polygon.plane))
			}
		}
	}

	best_score_plane.map(|x| x.1)
}

// smaller score means better
// None score means plane is not a splitter
fn get_splitter_plane_score(polygons: &[Polygon], plane: &Plane) -> Option<f32>
{
	let mut polygons_front = 0i32;
	let mut polygons_back = 0i32;
	let mut polygons_coplanar_front = 0i32;
	let mut polygons_coplanar_back = 0i32;
	let mut polygons_splitted = 0i32;
	for polygon in polygons
	{
		match get_polygon_position_relative_plane(polygon, plane)
		{
			PolygonPositionRelativePlane::Front =>
			{
				polygons_front += 1;
			},
			PolygonPositionRelativePlane::Back =>
			{
				polygons_back += 1;
			},
			PolygonPositionRelativePlane::CoplanarFront =>
			{
				polygons_coplanar_front += 1;
			},
			PolygonPositionRelativePlane::CoplanarBack =>
			{
				polygons_coplanar_back += 1;
			},
			PolygonPositionRelativePlane::Splitted =>
			{
				polygons_splitted += 1;
			},
		}
	}

	let polygons_front_total = polygons_front + polygons_coplanar_front;
	let polygons_back_total = polygons_back + polygons_coplanar_back;

	// All polygons are at one of sides. So, this is not a splitter.
	if polygons_splitted == 0 && (polygons_front_total == 0 || polygons_back_total == 0)
	{
		return None;
	}

	let base_score = (polygons_front_total - polygons_back_total).abs() + polygons_splitted;
	// TODO - scale down score for planes parallel to base planes (XY, XZ, YZ)
	Some(base_score as f32)
}

#[derive(PartialEq, Eq)]
enum PolygonPositionRelativePlane
{
	Front,
	Back,
	CoplanarFront,
	CoplanarBack,
	Splitted,
}

fn get_polygon_position_relative_plane(polygon: &Polygon, plane: &Plane) -> PolygonPositionRelativePlane
{
	let mut vertices_front = 0;
	let mut vertices_back = 0;
	for v in &polygon.vertices
	{
		match get_point_position_relative_plane(v, plane)
		{
			PointPositionRelativePlane::Front =>
			{
				vertices_front += 1;
			},
			PointPositionRelativePlane::Back =>
			{
				vertices_back += 1;
			},
			PointPositionRelativePlane::OnPlane =>
			{},
		};
	}

	if vertices_front != 0 && vertices_back != 0
	{
		PolygonPositionRelativePlane::Splitted
	}
	else if vertices_front != 0
	{
		PolygonPositionRelativePlane::Front
	}
	else if vertices_back != 0
	{
		PolygonPositionRelativePlane::Back
	}
	else if polygon.plane.vec.dot(plane.vec) >= 0.0
	{
		PolygonPositionRelativePlane::CoplanarFront
	}
	else
	{
		PolygonPositionRelativePlane::CoplanarBack
	}
}

#[derive(PartialEq, Eq)]
enum PointPositionRelativePlane
{
	Front,
	Back,
	OnPlane,
}

const POINT_POSITION_EPS: f32 = 1.0 / 16.0;

fn get_point_position_relative_plane(point: &Vec3f, plane: &Plane) -> PointPositionRelativePlane
{
	// Polygon vector is unnormalized. So, scale epsilon to length of this vector.
	let dist_scaled = point.dot(plane.vec) - plane.dist;
	let eps_scaled = POINT_POSITION_EPS * plane.vec.magnitude();
	if dist_scaled > eps_scaled
	{
		PointPositionRelativePlane::Front
	}
	else if dist_scaled < -eps_scaled
	{
		PointPositionRelativePlane::Back
	}
	else
	{
		PointPositionRelativePlane::OnPlane
	}
}
