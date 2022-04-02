use super::{map_polygonizer, math_types::*};
use std::{cell, rc};

pub use map_polygonizer::{Plane, Polygon};

// Portal between two BSP leafs.
#[derive(Debug)]
pub struct LeafsPortal
{
	pub leaf_front: rc::Rc<cell::RefCell<BSPLeaf>>,
	pub leaf_back: rc::Rc<cell::RefCell<BSPLeaf>>,
	pub plane: Plane,
	pub vertices: Vec<Vec3f>,
}

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
	// TODO - fill this portals list.
	pub portals: Vec<rc::Weak<cell::RefCell<LeafsPortal>>>,
}

#[derive(Debug)]
pub enum BSPNodeChild
{
	NodeChild(rc::Rc<cell::RefCell<BSPNode>>),
	LeafChild(rc::Rc<cell::RefCell<BSPLeaf>>),
}

pub struct BSPTree
{
	pub root: BSPNodeChild,
	pub portals: Vec<rc::Rc<cell::RefCell<LeafsPortal>>>,
}

pub fn build_leaf_bsp_tree(entity: &map_polygonizer::Entity) -> BSPTree
{
	let inf = 1.0e8;
	let bbox_extend = 128.0;
	let mut bbox = MapBBox {
		min: Vec3f::new(inf, inf, inf),
		max: Vec3f::new(-inf, -inf, -inf),
	};
	for polygon in &entity.polygons
	{
		for v in &polygon.vertices
		{
			if v.x < bbox.min.x
			{
				bbox.min.x = v.x;
			}
			if v.x > bbox.max.x
			{
				bbox.max.x = v.x;
			}
			if v.y < bbox.min.y
			{
				bbox.min.y = v.y;
			}
			if v.y > bbox.max.y
			{
				bbox.max.y = v.y;
			}
			if v.z < bbox.min.z
			{
				bbox.min.z = v.z;
			}
			if v.z > bbox.max.z
			{
				bbox.max.z = v.z;
			}
		}
	}
	bbox.min -= Vec3f::new(bbox_extend, bbox_extend, bbox_extend);
	bbox.max += Vec3f::new(bbox_extend, bbox_extend, bbox_extend);

	let root = build_leaf_bsp_tree_r(entity.polygons.clone());
	let portals = build_protals(&root, &bbox);
	set_leafs_portals(&portals);
	BSPTree { root, portals }
}

fn build_leaf_bsp_tree_r(mut in_polygons: Vec<Polygon>) -> BSPNodeChild
{
	let splitter_plane_opt = choose_best_splitter_plane(&in_polygons);
	if splitter_plane_opt.is_none()
	{
		// No splitter plane means this is a leaf.
		return BSPNodeChild::LeafChild(rc::Rc::new(cell::RefCell::new(BSPLeaf {
			polygons: in_polygons,
			portals: Vec::new(),
		})));
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
				let (front_polygon, back_polygon) = split_polygon(&polygon, &splitter_plane);
				// Check for number of vertices is not needed here, but add anyway to avoid further problems if something is broken.
				if front_polygon.vertices.len() >= 3
				{
					polygons_front.push(front_polygon);
				}
				if back_polygon.vertices.len() >= 3
				{
					polygons_back.push(back_polygon);
				}
			},
		}
	}

	// HACK! Something went wrong and we processing leaf now.
	if polygons_front.is_empty()
	{
		return BSPNodeChild::LeafChild(rc::Rc::new(cell::RefCell::new(BSPLeaf {
			polygons: polygons_back,
			portals: Vec::new(),
		})));
	}
	if polygons_back.is_empty()
	{
		return BSPNodeChild::LeafChild(rc::Rc::new(cell::RefCell::new(BSPLeaf {
			polygons: polygons_front,
			portals: Vec::new(),
		})));
	}

	BSPNodeChild::NodeChild(rc::Rc::new(cell::RefCell::new(BSPNode {
		plane: splitter_plane,
		children: [
			build_leaf_bsp_tree_r(polygons_front),
			build_leaf_bsp_tree_r(polygons_back),
		],
	})))
}

// Returns pair of front and back polygons.
fn split_polygon(in_polygon: &Polygon, plane: &Plane) -> (Polygon, Polygon)
{
	let mut polygon_front = Polygon {
		plane: in_polygon.plane,
		texture_info: in_polygon.texture_info.clone(),
		vertices: Vec::new(),
	};
	let mut polygon_back = Polygon {
		plane: in_polygon.plane,
		texture_info: in_polygon.texture_info.clone(),
		vertices: Vec::new(),
	};

	let mut prev_vert = in_polygon.vertices.last().unwrap();
	let mut prev_vert_pos = get_point_position_relative_plane(&prev_vert, plane);
	for vert in &in_polygon.vertices
	{
		let vert_pos = get_point_position_relative_plane(&vert, plane);

		match vert_pos
		{
			PointPositionRelativePlane::Front =>
			{
				if prev_vert_pos == PointPositionRelativePlane::Back
				{
					let intersection = get_line_plane_intersection(prev_vert, vert, plane);
					polygon_back.vertices.push(intersection);
					polygon_front.vertices.push(intersection);
				}
				polygon_front.vertices.push(*vert);
			},
			PointPositionRelativePlane::Back =>
			{
				if prev_vert_pos == PointPositionRelativePlane::Front
				{
					let intersection = get_line_plane_intersection(prev_vert, vert, plane);
					polygon_front.vertices.push(intersection);
					polygon_back.vertices.push(intersection);
				}
				polygon_back.vertices.push(*vert);
			},
			PointPositionRelativePlane::OnPlane =>
			{
				polygon_front.vertices.push(*vert);
				polygon_back.vertices.push(*vert);
			},
		};

		prev_vert = vert;
		prev_vert_pos = vert_pos;
	}

	(polygon_front, polygon_back)
}

fn get_line_plane_intersection(v0: &Vec3f, v1: &Vec3f, plane: &Plane) -> Vec3f
{
	let dist0 = v0.dot(plane.vec) - plane.dist;
	let dist1 = v1.dot(plane.vec) - plane.dist;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
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

	// TODO - tune this carefully.
	const SPLITTED_POLYGON_SCALE: i32 = 5;
	let base_score = (polygons_front_total - polygons_back_total).abs() + SPLITTED_POLYGON_SCALE * polygons_splitted;

	// Make score greater (worse) for planes non-parallel to axis planes.
	let mut num_zero_normal_components = 0;
	let plane_vec_as_array: &[f32; 3] = plane.vec.as_ref();
	for component in plane_vec_as_array
	{
		if *component == 0.0
		{
			num_zero_normal_components += 1;
		}
	}

	let mut score_scaled = base_score as f32;
	if num_zero_normal_components == 0
	{
		score_scaled *= 2.0;
	}
	if num_zero_normal_components == 1
	{
		score_scaled *= 1.5;
	}

	Some(score_scaled)
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

fn build_protals(node: &BSPNodeChild, map_bbox: &MapBBox) -> Vec<rc::Rc<cell::RefCell<LeafsPortal>>>
{
	let mut splitter_nodes = Vec::new();
	let mut leaf_portals_by_node = LeafPortalsInitialByNode::new();
	build_protals_r(node, &mut splitter_nodes, &mut leaf_portals_by_node, map_bbox);

	let mut result = Vec::new();
	for (_node, portals) in leaf_portals_by_node
	{
		for result_portal in build_leafs_portals(&portals)
		{
			result.push(rc::Rc::new(cell::RefCell::new(result_portal)));
		}
	}
	result
}

struct MapBBox
{
	min: Vec3f,
	max: Vec3f,
}

struct NodeForPortalsBuild
{
	node: rc::Rc<cell::RefCell<BSPNode>>,
	is_front: bool,
}

struct LeafPortalInitial
{
	vertices: Vec<Vec3f>,
	node: rc::Rc<cell::RefCell<BSPNode>>,
	leaf: rc::Rc<cell::RefCell<BSPLeaf>>,
	is_front: bool,
}

type LeafPortalsInitialByNode = std::collections::HashMap<*const BSPNode, Vec<LeafPortalInitial>>;

fn build_protals_r(
	node_child: &BSPNodeChild,
	splitter_nodes: &mut Vec<NodeForPortalsBuild>,
	leaf_portals_by_node: &mut LeafPortalsInitialByNode,
	map_bbox: &MapBBox,
)
{
	match node_child
	{
		BSPNodeChild::NodeChild(node) =>
		{
			splitter_nodes.push(NodeForPortalsBuild {
				node: node.clone(),
				is_front: true,
			});
			build_protals_r(
				&node.borrow().children[0],
				splitter_nodes,
				leaf_portals_by_node,
				map_bbox,
			);
			splitter_nodes.pop();

			splitter_nodes.push(NodeForPortalsBuild {
				node: node.clone(),
				is_front: false,
			});
			build_protals_r(
				&node.borrow().children[1],
				splitter_nodes,
				leaf_portals_by_node,
				map_bbox,
			);
			splitter_nodes.pop();
		},
		BSPNodeChild::LeafChild(leaf_ptr) =>
		{
			// Build list of portals by leaf. Than group portals by node.
			for leaf_portal in build_leaf_portals(leaf_ptr, &splitter_nodes, map_bbox)
			{
				let node = leaf_portal.node.clone();
				let ptr = (&*node.borrow()) as *const BSPNode;
				if !leaf_portals_by_node.contains_key(&ptr)
				{
					leaf_portals_by_node.insert(ptr, Vec::new());
				}
				leaf_portals_by_node.get_mut(&ptr).unwrap().push(leaf_portal);
			}
		},
	}
}

fn build_leaf_portals(
	leaf_ptr: &rc::Rc<cell::RefCell<BSPLeaf>>,
	splitter_nodes: &[NodeForPortalsBuild],
	map_bbox: &MapBBox,
) -> Vec<LeafPortalInitial>
{
	let leaf = &leaf_ptr.borrow();
	// For each splitter plane create portal polygon - bounded with all other splitter planes and leaf polygons.

	let mut cut_planes = Vec::<Plane>::new();
	for splitter_node in splitter_nodes
	{
		let node = splitter_node.node.borrow();
		if splitter_node.is_front
		{
			cut_planes.push(Plane {
				vec: -node.plane.vec,
				dist: -node.plane.dist,
			});
		}
		else
		{
			cut_planes.push(node.plane);
		}
	}
	for polygon in &leaf.polygons
	{
		cut_planes.push(Plane {
			vec: -polygon.plane.vec,
			dist: -polygon.plane.dist,
		});
	}

	cut_planes.push(Plane {
		vec: Vec3f::new(1.0, 0.0, 0.0),
		dist: map_bbox.max.x,
	});
	cut_planes.push(Plane {
		vec: Vec3f::new(-1.0, 0.0, 0.0),
		dist: -map_bbox.min.x,
	});
	cut_planes.push(Plane {
		vec: Vec3f::new(0.0, 1.0, 0.0),
		dist: map_bbox.max.y,
	});
	cut_planes.push(Plane {
		vec: Vec3f::new(0.0, -1.0, 0.0),
		dist: -map_bbox.min.y,
	});
	cut_planes.push(Plane {
		vec: Vec3f::new(0.0, 0.0, 1.0),
		dist: map_bbox.max.z,
	});
	cut_planes.push(Plane {
		vec: Vec3f::new(0.0, 0.0, -1.0),
		dist: -map_bbox.min.z,
	});

	let mut portals = Vec::new();
	for splitter_node in splitter_nodes
	{
		let node = splitter_node.node.borrow();
		let portal_plane = if splitter_node.is_front
		{
			Plane {
				vec: -node.plane.vec,
				dist: -node.plane.dist,
			}
		}
		else
		{
			node.plane
		};

		let mut portal_vertices = Vec::new();
		for i in 0 .. cut_planes.len()
		{
			let cut_plane_i = cut_planes[i];
			if cut_plane_i == portal_plane
			{
				continue;
			}
			if are_planes_almost_parallel(&portal_plane, &cut_plane_i)
			{
				continue;
			}

			for j in i + 1 .. cut_planes.len()
			{
				let cut_plane_j = cut_planes[j];
				if cut_plane_j == portal_plane
				{
					continue;
				}
				if cut_plane_j == cut_plane_i
				{
					continue;
				}
				if are_planes_almost_parallel(&portal_plane, &cut_plane_j)
				{
					continue;
				}
				if are_planes_almost_parallel(&cut_plane_i, &cut_plane_j)
				{
					continue;
				}

				let mat = Mat3f::from_cols(portal_plane.vec, cut_plane_i.vec, cut_plane_j.vec).transpose();
				let inv_mat_opt = mat.invert();
				if inv_mat_opt.is_none()
				{
					continue; // No solution - some planes are parallel.
				}
				let intersection_point =
					inv_mat_opt.unwrap() * Vec3f::new(portal_plane.dist, cut_plane_i.dist, cut_plane_j.dist);

				let mut is_behind_another_plane = false;
				for k in 0 .. cut_planes.len()
				{
					if k == i || k == j
					{
						continue;
					}
					let plane_k = cut_planes[k];
					if plane_k == portal_plane
					{
						continue;
					}
					if intersection_point.dot(plane_k.vec) > plane_k.dist
					{
						is_behind_another_plane = true;
						break;
					}
				} // for k

				if !is_behind_another_plane
				{
					portal_vertices.push(intersection_point);
				}
			} // for j
		} // for i

		if portal_vertices.len() < 3
		{
			continue;
		}

		let portal_vertices_deduplicated = map_polygonizer::remove_duplicate_vertices(&portal_vertices);
		if portal_vertices_deduplicated.len() < 3
		{
			continue;
		}

		let portal_vertices_sorted =
			map_polygonizer::sort_convex_polygon_vertices(portal_vertices_deduplicated, &node.plane);
		if portal_vertices_sorted.len() < 3
		{
			continue;
		}
		portals.push(LeafPortalInitial {
			vertices: portal_vertices_sorted,
			node: splitter_node.node.clone(),
			leaf: leaf_ptr.clone(),
			is_front: splitter_node.is_front,
		});
	} // for portal planes

	portals
}

// Iterate over all pairs of portals of same node.
// Search for intersection of such portals.
fn build_leafs_portals(in_portals: &[LeafPortalInitial]) -> Vec<LeafsPortal>
{
	let mut result = Vec::new();
	for portal_front in in_portals
	{
		if !portal_front.is_front
		{
			continue;
		}

		let plane = portal_front.node.borrow().plane;
		let plane_inverted = Plane {
			vec: -plane.vec,
			dist: -plane.dist,
		};

		for portal_back in in_portals
		{
			if portal_back.is_front
			{
				continue;
			}

			let portals_intersection =
				build_portals_intersection(&plane, &portal_back.vertices, &portal_front.vertices);
			if portals_intersection.len() < 3
			{
				continue;
			}

			// TODO - enable this check.
			// if is_portal_fully_covered_by_leaf_polygons(&plane_inverted, &portals_intersection, &portal_a.leaf.borrow()) ||
			// 	is_portal_fully_covered_by_leaf_polygons(&plane, &portals_intersection, &portal_b.leaf.borrow())
			{
				// continue;
			}

			result.push(LeafsPortal {
				leaf_front: portal_front.leaf.clone(),
				leaf_back: portal_back.leaf.clone(),
				plane: plane,
				vertices: portals_intersection,
			});
		}
	}

	result
}

// Return < 3 vertices if failed.
fn build_portals_intersection(plane: &Plane, vertices0: &[Vec3f], vertices1: &[Vec3f]) -> Vec<Vec3f>
{
	let mut clip_planes = Vec::new();

	let mut prev_v = vertices0.last().unwrap();
	for v in vertices0
	{
		let vec = (prev_v - v).cross(plane.vec);
		clip_planes.push(Plane { vec, dist: vec.dot(*v) });
		prev_v = v;
	}
	let mut prev_v = vertices1.last().unwrap();
	for v in vertices1
	{
		let vec = (prev_v - v).cross(plane.vec);
		clip_planes.push(Plane { vec, dist: vec.dot(*v) });
		prev_v = v;
	}

	// Build set of vertices based on input planes.
	let mut vertices = Vec::new();
	for i in 0 .. clip_planes.len()
	{
		let plane_i = clip_planes[i];
		for j in i + 1 .. clip_planes.len()
		{
			let plane_j = clip_planes[j];
			if plane_j == plane_i
			{
				continue;
			}
			if are_planes_almost_parallel(&plane_i, &plane_j)
			{
				continue;
			}

			// Find intersection point between portal side planes and plane of portal.
			let mat = Mat3f::from_cols(plane.vec, plane_i.vec, plane_j.vec).transpose();
			let inv_mat_opt = mat.invert();
			if inv_mat_opt.is_none()
			{
				continue; // No solution - some planes are parallel.
			}
			let intersection_point = inv_mat_opt.unwrap() * Vec3f::new(plane.dist, plane_i.dist, plane_j.dist);

			let mut is_behind_another_plane = false;
			for k in 0 .. clip_planes.len()
			{
				if k == i || k == j
				{
					continue;
				}
				let plane_k = clip_planes[k];
				if plane_k == plane_i || plane_k == plane_j
				{
					continue;
				}
				const EPS: f32 = 1.0 / 16.0;
				if intersection_point.dot(plane_k.vec) > plane_k.dist + plane_k.vec.magnitude() / EPS
				{
					is_behind_another_plane = true;
					break;
				}
			} // for k

			if !is_behind_another_plane
			{
				vertices.push(intersection_point);
			}
		} // for j
	} // for i

	if vertices.len() < 3
	{
		return vertices;
	}

	let vertices_deduplicated = map_polygonizer::remove_duplicate_vertices(&vertices);
	if vertices_deduplicated.len() < 3
	{
		return vertices_deduplicated;
	}

	map_polygonizer::sort_convex_polygon_vertices(vertices_deduplicated, &plane)
}

fn is_portal_fully_covered_by_leaf_polygons(
	portal_plane_inverted: &Plane,
	portal_vertices: &[Vec3f],
	leaf: &BSPLeaf,
) -> bool
{
	// Perform basic portals filtering.
	// Remove portals that are fully covered by one of leaf polygons.
	// Generally we should check for coverage by multiple polygons, but not now.

	const PORTAL_POLYGON_COVERAGE_EPS: f32 = 0.25;

	for polygon in &leaf.polygons
	{
		// Check only polygons in same plane.
		if *portal_plane_inverted != polygon.plane
		{
			continue;
		}

		let mut prev_polygon_vertex = polygon.vertices.last().unwrap();
		let mut portal_is_inside_polygon = true;
		for polygon_vertex in &polygon.vertices
		{
			let vec = (prev_polygon_vertex - polygon_vertex).cross(polygon.plane.vec);
			let eps_scaled = PORTAL_POLYGON_COVERAGE_EPS * vec.magnitude();
			let cut_plane = Plane {
				vec: vec,
				dist: vec.dot(*polygon_vertex),
			};

			let mut all_vertices_are_inside = true;
			for portal_vertex in portal_vertices
			{
				if portal_vertex.dot(cut_plane.vec) > cut_plane.dist + eps_scaled
				{
					all_vertices_are_inside = false;
					break;
				}
			}

			prev_polygon_vertex = polygon_vertex;

			if !all_vertices_are_inside
			{
				portal_is_inside_polygon = false;
				break;
			}
		} // for polygon edges

		if portal_is_inside_polygon
		{
			return true;
		}
	} // for polygons

	false
}

fn are_planes_almost_parallel(plane0: &Plane, plane1: &Plane) -> bool
{
	(plane0.vec.cross(plane1.vec).magnitude() / plane0.vec.dot(plane1.vec)).abs() < 0.0001
}

fn set_leafs_portals(portals: &[rc::Rc<cell::RefCell<LeafsPortal>>])
{
	for portal_ptr in portals
	{
		let portal_ptr_weak = rc::Rc::downgrade(portal_ptr);
		let portal = portal_ptr.borrow();
		portal.leaf_front.borrow_mut().portals.push(portal_ptr_weak.clone());
		portal.leaf_back.borrow_mut().portals.push(portal_ptr_weak);
	}
}
