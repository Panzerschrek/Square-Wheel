use super::{map_file, math_types::*};

#[derive(Debug)]
pub struct Plane
{
	pub vec: Vec3f, // Unnormalized direction
	pub dist: f32,  // for point on plane dot(vec, point) = dist
}

#[derive(Debug)]
pub struct Polygon
{
	pub plane: Plane,
	pub vertices: Vec<Vec3f>,
}

#[derive(Debug)]
pub struct Entity
{
	pub polygons: Vec<Polygon>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapPolygonized = Vec<Entity>;

pub fn polygonize_map(input_map: &map_file::MapFileParsed) -> MapPolygonized
{
	input_map.iter().map(polygonize_entity).collect()
}

fn polygonize_entity(input_entity: &map_file::Entity) -> Entity
{
	let mut polygons = Vec::new();
	for brush in &input_entity.brushes
	{
		polygons.append(&mut polygonize_brush(brush));
	}

	Entity {
		polygons,
		keys: input_entity.keys.clone(),
	}
}

fn polygonize_brush(brush: &map_file::Brush) -> Vec<Polygon>
{
	let mut result = Vec::new();

	// Iterate over all brush planes "i".
	// For each brush plane iterate over all possible pairs of planes and build point of intersection.
	// Than check if this point is lies behind brush plane. If so - add point to result.
	for i in 0 .. brush.len()
	{
		let plane_i_opt = get_brush_side_plane(&brush[i]);
		if plane_i_opt.is_none()
		{
			continue;
		}
		let plane_i = plane_i_opt.unwrap();

		let mut vertices = Vec::new();
		for j in 0 .. brush.len()
		{
			if j == i
			{
				continue;
			}
			let plane_j_opt = get_brush_side_plane(&brush[j]);
			if plane_j_opt.is_none()
			{
				continue;
			}
			let plane_j = plane_j_opt.unwrap();

			for k in 0 .. brush.len()
			{
				if k == i || k == j
				{
					continue;
				}
				let plane_k_opt = get_brush_side_plane(&brush[k]);
				if plane_k_opt.is_none()
				{
					continue;
				}
				let plane_k = plane_k_opt.unwrap();

				// Find intersection point by solving system of 3 linear equations.
				// Do this using approach with inverse matrix calculation.
				let mat = Mat3f::from_cols(plane_i.vec, plane_j.vec, plane_k.vec).transpose();
				let inv_mat_opt = mat.invert();
				if inv_mat_opt.is_none()
				{
					continue; // No solution - some planes are parallel.
				}
				let intersection_point = inv_mat_opt.unwrap() * Vec3f::new(plane_i.dist, plane_j.dist, plane_k.dist);

				let mut is_behind_another_plane = false;
				for l in 0 .. brush.len()
				{
					if l == i || l == j || l == k
					{
						continue;
					}
					let plane_l_opt = get_brush_side_plane(&brush[l]);
					if plane_l_opt.is_none()
					{
						continue;
					}
					let plane_l = plane_l_opt.unwrap();

					if intersection_point.dot(plane_l.vec) < plane_l.dist
					{
						is_behind_another_plane = true;
						break;
					}
				} // for l

				if !is_behind_another_plane
				{
					vertices.push(intersection_point);
				}
			} // for k
		} // for j

		vertices = remove_duplicate_vertices(&vertices);
		if vertices.len() < 3
		{
			continue;
		}

		// TODO - sort vertices - make correct (clockwise/anticlockwise order).
		result.push(Polygon {
			plane: plane_i,
			vertices: vertices,
		});
	} // for i

	result
}

fn get_brush_side_plane(brush_side: &map_file::BrushPlane) -> Option<Plane>
{
	let vec = (brush_side.vertices[0] - brush_side.vertices[1]).cross(brush_side.vertices[1] - brush_side.vertices[2]);
	if vec.is_zero()
	{
		return None;
	}

	Some(Plane {
		vec: vec,
		dist: vec.dot(brush_side.vertices[0]),
	})
}

fn remove_duplicate_vertices(in_vertices: &[Vec3f]) -> Vec<Vec3f>
{
	let mut result = Vec::new();
	for in_vertex in in_vertices
	{
		let mut duplicate = false;
		for existing_vertex in &result
		{
			if existing_vertex == in_vertex
			{
				duplicate = true;
				break;
			}
		}
		if !duplicate
		{
			result.push(*in_vertex);
		}
	}
	result
}
