use super::{
	clipping_bsp::*,
	map_polygonizer::{Brush, MapPolygonized, Polygon},
	material,
	math_types::*,
	plane::*,
};

#[derive(Debug, Clone)]
pub struct Entity
{
	pub polygons: Vec<Polygon>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapCSGProcessed = Vec<Entity>;

pub fn perform_no_csg_for_map_brushes(map: &MapPolygonized) -> MapCSGProcessed
{
	map.iter()
		.map(|e| {
			let mut polygons = Vec::new();
			for brush in &e.brushes
			{
				polygons.extend_from_slice(&brush);
			}
			Entity {
				polygons,
				keys: e.keys.clone(),
			}
		})
		.collect()
}

pub fn perform_csg_for_map_brushes(map: &MapPolygonized, materials: &material::MaterialsMap) -> MapCSGProcessed
{
	map.iter()
		.map(|e| Entity {
			polygons: perform_csg_for_entity_brushes(&e.brushes, materials),
			keys: e.keys.clone(),
		})
		.collect()
}

// CSG allows to remove polygons that are lying between two brushes, or polygons of one brushes, lying inside another brush.
// This prepass allows to reduce number of polygons and simplify further BSP building.
pub fn perform_csg_for_entity_brushes(brushes: &[Brush], materials: &material::MaterialsMap) -> Vec<Polygon>
{
	let mut result_polygons = Vec::new();

	// Fill table with solid flag for brushes.
	// Brush is solid if all faces are solid.
	let solid_flags = brushes
		.iter()
		.map(|brush| {
			for polygon in brush
			{
				if let Some(material) = materials.get(&polygon.texture_info.texture)
				{
					if !material.blocks_view
					{
						return false;
					}
				}
			}
			true
		})
		.collect::<Vec<_>>();

	for brush in brushes
	{
		if brush.is_empty()
		{
			continue;
		}

		let mut brush_polygons = brush.clone();

		// Set this flag to true for all brushes after this.
		// This is needed in order to choose only one polygons of two coplanar polygons of two intersecting brushes.
		let mut preserve_coplanar = false;

		for (other_brush, solid_flag) in brushes.iter().zip(solid_flags.iter())
		{
			// TODO - speed-up this, perform bbox check.
			if other_brush as *const Vec<Polygon> == brush as *const Vec<Polygon>
			{
				preserve_coplanar = true;
				continue;
			}

			if !solid_flag
			{
				continue;
			}

			let mut polygons_clipped = Vec::new();
			for polygon in brush_polygons.drain(..)
			{
				polygons_clipped.append(&mut cut_polygon_by_brush_planes(
					polygon,
					other_brush,
					preserve_coplanar,
				));
			}
			brush_polygons = polygons_clipped;
		}

		result_polygons.append(&mut brush_polygons);
	}

	result_polygons
}

fn cut_polygon_by_brush_planes(polygon: Polygon, brush: &Vec<Polygon>, preserve_coplanar: bool) -> Vec<Polygon>
{
	// Check if this polygon is trivially outisde.
	for brush_polygon in brush
	{
		if get_polygon_position_relative_plane(&polygon, &brush_polygon.plane) == PolygonPositionRelativePlane::Front
		{
			return vec![polygon];
		}
	}

	// Cut polygon into pieces by sides of this brush.
	let mut result_polygons = Vec::new();
	let mut leftover_polygon = polygon.clone();

	for brush_polygon in brush
	{
		match get_polygon_position_relative_plane(&leftover_polygon, &brush_polygon.plane)
		{
			PolygonPositionRelativePlane::Front =>
			{
				// Leftover polygon is outside the brush - splitting was unnecessary - return initial polygon.
				return vec![polygon];
			},
			PolygonPositionRelativePlane::Back =>
			{
				// Leftover polygon is possible inside the brush - continue splitting.
			},
			PolygonPositionRelativePlane::CoplanarFront =>
			{
				// We need to save polygon only if same polygon of other brush was previously skipped.
				if preserve_coplanar
				{
					// Leftover polygon is outside the brush - splitting was unnecessary - return initial polygon.
					return vec![polygon];
				}
			},
			PolygonPositionRelativePlane::CoplanarBack =>
			{
				// Preserve coplanar leftover polygon.
			},
			PolygonPositionRelativePlane::Splitted =>
			{
				let (front_polygon, back_polygon) = split_polygon(&leftover_polygon, &brush_polygon.plane);
				if front_polygon.vertices.len() >= 3
				{
					result_polygons.push(front_polygon); // Front polygon piece is outside brush - preserve it.
				}

				if back_polygon.vertices.len() >= 3
				{
					// Continue clipping of inside piese.
					leftover_polygon = back_polygon;
				}
				else
				{
					// Leftover polygon is outside the brush - splitting was unnecessary - return initial polygon.
					return vec![polygon];
				}
			},
		};
	} // for brush planes.

	if result_polygons.len() <= 1
	{
		// Cut polygon completely or leave one piece. Preserve this result.
		return result_polygons;
	}

	// We can't just return result pieces, because they are cuttet (potentially) very crudely.
	// We need to cut source polygon, using leftover polygon, preserving cut directions orthogonal.
	return make_hole_in_polygon(polygon, &leftover_polygon);
}

// Hole must be inside polygon.
fn make_hole_in_polygon(polygon: Polygon, hole: &Polygon) -> Vec<Polygon>
{
	// Perform cuts from vertices of hole polygon.
	// Perform cuts only along texture axis.

	// Collect cuts.
	let possible_cut_plane_normals = [
		polygon.texture_info.tex_coord_equation[0].vec,
		-polygon.texture_info.tex_coord_equation[0].vec,
		polygon.texture_info.tex_coord_equation[1].vec,
		-polygon.texture_info.tex_coord_equation[1].vec,
	];

	let num_hole_vertices = hole.vertices.len();
	let rotation_half_pi = QuaternionF::from_axis_angle(hole.plane.vec.normalize(), Rad(std::f32::consts::PI * 0.5));
	let mut vertex_cuts = Vec::with_capacity(num_hole_vertices);
	for i in 0 .. num_hole_vertices
	{
		let v_prev = hole.vertices[(i + num_hole_vertices - 1) % num_hole_vertices];
		let v = hole.vertices[i];
		let v_next = hole.vertices[(i + 1) % num_hole_vertices];

		let mut selected_cut_plane_vec = None;
		for possible_cut_plane_normal in &possible_cut_plane_normals
		{
			let possible_cut_plane = Plane {
				vec: *possible_cut_plane_normal,
				dist: possible_cut_plane_normal.dot(v),
			};
			if possible_cut_plane.vec.dot(v_prev) <= possible_cut_plane.dist &&
				possible_cut_plane.vec.dot(v_next) >= possible_cut_plane.dist
			{
				selected_cut_plane_vec = Some(*possible_cut_plane_normal);
				break;
			}
		}

		if let Some(selected_cut_plane_vec) = selected_cut_plane_vec
		{
			// Ok - select single cut plane for this vertex.
			vertex_cuts.push(VertexCut::Single(selected_cut_plane_vec));
		}
		else
		{
			let mut selected_cut_plane_vec_edge_prev = None;
			let mut selected_cut_plane_vec_edge = None;
			for possible_cut_plane_normal in &possible_cut_plane_normals
			{
				let possible_cut_plane = Plane {
					vec: *possible_cut_plane_normal,
					dist: possible_cut_plane_normal.dot(v),
				};
				if possible_cut_plane.vec.dot(v_prev) <= possible_cut_plane.dist &&
					selected_cut_plane_vec_edge_prev.is_none()
				{
					selected_cut_plane_vec_edge_prev = Some(*possible_cut_plane_normal);
				}
				if possible_cut_plane.vec.dot(v_next) >= possible_cut_plane.dist &&
					selected_cut_plane_vec_edge.is_none()
				{
					selected_cut_plane_vec_edge = Some(*possible_cut_plane_normal);
				}
			}

			if let (Some(vec_prev), Some(vec)) = (selected_cut_plane_vec_edge_prev, selected_cut_plane_vec_edge)
			{
				vertex_cuts.push(VertexCut::Double(vec_prev, vec));
			}
			else
			{
				// Something went really wrong.
				return vec![polygon];
			}
		}
	}

	debug_assert!(vertex_cuts.len() == num_hole_vertices);

	// Perform cuts.
	let mut result_polygons = Vec::new();
	for i in 0 .. num_hole_vertices
	{
		let v = hole.vertices[i];
		let v_next = hole.vertices[(i + 1) % num_hole_vertices];
		let edge = v_next - v;
		let edge_normal = rotation_half_pi.rotate_vector(edge);

		if let VertexCut::Double(vec_prev, vec) = vertex_cuts[i]
		{
			// TODO - check this.
			// Perform double cut for this vertex.
			let corner_polygon = cut_polygon_by_planes(
				&polygon,
				&[
					Plane {
						vec: vec_prev,
						dist: vec_prev.dot(v),
					},
					Plane {
						vec: -vec,
						dist: -vec.dot(v),
					},
				],
			);
			if corner_polygon.vertices.len() >= 3
			{
				result_polygons.push(corner_polygon);
			}
		}

		let cut_vec = match vertex_cuts[i]
		{
			VertexCut::Single(vec) => vec,
			VertexCut::Double(_vec_prev, vec) => vec,
		};
		let cut_vec_next = match vertex_cuts[(i + 1) % num_hole_vertices]
		{
			VertexCut::Single(vec) => -vec,
			VertexCut::Double(vec_prev, _vec) => -vec_prev,
		};

		let edge_polygon = cut_polygon_by_planes(
			&polygon,
			&[
				Plane {
					vec: cut_vec,
					dist: cut_vec.dot(v),
				},
				Plane {
					vec: cut_vec_next,
					dist: cut_vec_next.dot(v_next),
				},
				Plane {
					vec: edge_normal,
					dist: edge_normal.dot(v),
				},
			],
		);

		if edge_polygon.vertices.len() >= 3
		{
			result_polygons.push(edge_polygon);
		}
	}

	if result_polygons.is_empty()
	{
		// Something went wrong.
		return vec![polygon];
	}

	result_polygons
}

fn cut_polygon_by_planes(polygon: &Polygon, planes: &[Plane]) -> Polygon
{
	let mut result = polygon.clone();
	for plane in planes
	{
		let (front_polygon, _back_polygon) = split_polygon(&result, plane);
		if front_polygon.vertices.len() < 3
		{
			return front_polygon;
		}
		result = front_polygon;
	}

	result
}

#[derive(Copy, Clone)]
enum VertexCut
{
	Single(Vec3f),
	Double(Vec3f, Vec3f),
}
