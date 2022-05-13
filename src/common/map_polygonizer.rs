use super::{map_file_q1, map_file_q4, math_types::*, plane::Plane};

#[derive(Debug, Clone)]
pub struct TextureInfo
{
	pub tex_coord_equation: [Plane; 2],
	pub texture: String,
}

#[derive(Debug, Clone)]
pub struct Polygon
{
	pub plane: Plane,
	pub texture_info: TextureInfo,
	pub vertices: Vec<Vec3f>,
}

#[derive(Debug, Clone)]
pub struct Entity
{
	pub polygons: Vec<Polygon>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapPolygonized = Vec<Entity>;

pub fn polygonize_map(input_map: &[map_file_q1::Entity]) -> MapPolygonized
{
	input_map.iter().map(polygonize_entity).collect()
}

pub fn polygonize_map_q4<TextureSizeGetter: FnMut(&str) -> [u32; 2]>(
	input_map: &[map_file_q4::Entity],
	texture_size_getter: &mut TextureSizeGetter,
) -> MapPolygonized
{
	let mut result = input_map.iter().map(polygonize_entity_q4).collect();
	for entity in &mut result
	{
		correct_texture_basis_scale_q4(entity, texture_size_getter);
	}

	result
}

fn polygonize_entity(input_entity: &map_file_q1::Entity) -> Entity
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

fn polygonize_entity_q4(input_entity: &map_file_q4::Entity) -> Entity
{
	let mut polygons = Vec::new();
	for brush in &input_entity.brushes
	{
		polygons.append(&mut polygonize_brush_q4(brush));
	}

	Entity {
		polygons,
		keys: input_entity.keys.clone(),
	}
}

fn correct_texture_basis_scale_q4<TextureSizeGetter: FnMut(&str) -> [u32; 2]>(
	entity: &mut Entity,
	texture_size_getter: &mut TextureSizeGetter,
)
{
	// Quake IV uses normailzed texture coordinates, but we need to use absolute coordinates.
	// So, perform such conversion.
	for polygon in &mut entity.polygons
	{
		let texture_size = texture_size_getter(&polygon.texture_info.texture);
		for i in 0 .. 2
		{
			polygon.texture_info.tex_coord_equation[i].vec *= texture_size[i] as f32;
			polygon.texture_info.tex_coord_equation[i].dist *= texture_size[i] as f32;
		}
	}
}

fn polygonize_brush(brush: &[map_file_q1::BrushPlane]) -> Vec<Polygon>
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

			for k in j + 1 .. brush.len()
			{
				if k == i
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

					if intersection_point.dot(plane_l.vec) > plane_l.dist
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
			println!("Wrong polygon with only {} vertices", vertices.len());
			continue;
		}

		let vertices_sorted = sort_convex_polygon_vertices(vertices, &plane_i);
		if vertices_sorted.len() < 3
		{
			println!("Wrong polygon with only {} vertices_sorted", vertices_sorted.len());
			continue;
		}

		result.push(Polygon {
			plane: plane_i,
			texture_info: get_polygon_texture_info(&brush[i], &plane_i.vec),
			vertices: vertices_sorted,
		});
	} // for i

	result
}

fn polygonize_brush_q4(brush: &[map_file_q4::BrushPlane]) -> Vec<Polygon>
{
	let mut result = Vec::new();

	// Iterate over all brush planes "i".
	// For each brush plane iterate over all possible pairs of planes and build point of intersection.
	// Than check if this point is lies behind brush plane. If so - add point to result.
	for i in 0 .. brush.len()
	{
		let plane_i = &brush[i].plane;

		let mut vertices = Vec::new();
		for j in 0 .. brush.len()
		{
			if j == i
			{
				continue;
			}
			let plane_j = &brush[j].plane;

			for k in j + 1 .. brush.len()
			{
				if k == i
				{
					continue;
				}
				let plane_k = &brush[k].plane;

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
					let plane_l = &brush[l].plane;

					if intersection_point.dot(plane_l.vec) > plane_l.dist
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
			println!("Wrong polygon with only {} vertices", vertices.len());
			continue;
		}

		let vertices_sorted = sort_convex_polygon_vertices(vertices, &plane_i);
		if vertices_sorted.len() < 3
		{
			println!("Wrong polygon with only {} vertices_sorted", vertices_sorted.len());
			continue;
		}

		result.push(Polygon {
			plane: *plane_i,
			texture_info: get_polygon_texture_info_q4(&brush[i]),
			vertices: vertices_sorted,
		});
	} // for i

	result
}

fn get_brush_side_plane(brush_side: &map_file_q1::BrushPlane) -> Option<Plane>
{
	let vec = (brush_side.vertices[0] - brush_side.vertices[1]).cross(brush_side.vertices[2] - brush_side.vertices[1]);
	if vec.is_zero()
	{
		return None;
	}

	Some(Plane {
		vec,
		dist: vec.dot(brush_side.vertices[0]),
	})
}

pub fn remove_duplicate_vertices(in_vertices: &[Vec3f]) -> Vec<Vec3f>
{
	const DIST_EPS: f32 = 1.0 / 16.0;

	let mut result = Vec::<Vec3f>::new();
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
			let square_dist = (existing_vertex - in_vertex).magnitude2();
			if square_dist <= DIST_EPS * DIST_EPS
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

pub fn sort_convex_polygon_vertices(mut in_vertices: Vec<Vec3f>, plane: &Plane) -> Vec<Vec3f>
{
	// First, find average vertex. For convex polygon it is always inside it.
	let mut vertitces_sum = Vec3f::zero();
	for v in &in_vertices
	{
		vertitces_sum += *v;
	}
	let middle_vertex = vertitces_sum / (in_vertices.len() as f32);

	// Select first vertex.
	let mut result = vec![in_vertices.pop().unwrap()];

	while !in_vertices.is_empty()
	{
		// Search for vertex with smallest angle relative to vector from middle to last vertex.
		let v0 = result.last().unwrap() - middle_vertex;

		let mut smallest_cotan_vert = None;
		for i in 0 .. in_vertices.len()
		{
			let v1 = in_vertices[i] - middle_vertex;

			let dot = v0.dot(v1);
			let cross = v0.cross(v1);
			let cross_plane_vec_dot = cross.dot(plane.vec);
			if cross_plane_vec_dot >= 0.0
			{
				continue; // Wrong direction.
			}
			let scaled_angle_cotan = dot / cross_plane_vec_dot; // Should be equal to angle cotangent multiplied by plane vector length.
			if let Some((_, prev_cotan)) = smallest_cotan_vert
			{
				if scaled_angle_cotan < prev_cotan
				{
					smallest_cotan_vert = Some((i, scaled_angle_cotan));
				}
			}
			else
			{
				smallest_cotan_vert = Some((i, scaled_angle_cotan));
			}
		}
		if let Some((index, _)) = smallest_cotan_vert
		{
			result.push(in_vertices.remove(index));
		}
		else
		{
			// WTF?
			// println!(
			// "Can't find best vertex for sorting. Vertices produced: {}, left : {}",
			// result.len(),
			// in_vertices.len()
			// );
			break;
		}
	}

	result
}

fn get_polygon_texture_info(brush_plane: &map_file_q1::BrushPlane, polygon_normal: &Vec3f) -> TextureInfo
{
	let basis = get_texture_basis(polygon_normal);

	let angle_rad = brush_plane.tc_angle * (3.1415926535 / 180.0);
	let angle_cos = angle_rad.cos();
	let angle_sin = angle_rad.sin();
	let basis_rotated = [
		basis[0] * angle_cos - basis[1] * angle_sin,
		basis[0] * angle_sin + basis[1] * angle_cos,
	];

	TextureInfo {
		tex_coord_equation: [
			Plane {
				vec: basis_rotated[0] / brush_plane.tc_scale[0],
				dist: brush_plane.tc_offset[0],
			},
			Plane {
				vec: basis_rotated[1] / brush_plane.tc_scale[1],
				dist: brush_plane.tc_offset[1],
			},
		],
		texture: brush_plane.texture.clone(),
	}
}

fn get_polygon_texture_info_q4(brush_plane: &map_file_q4::BrushPlane) -> TextureInfo
{
	let basis = get_texture_basis(&brush_plane.plane.vec);

	TextureInfo {
		tex_coord_equation: [
			Plane {
				vec: basis[0] * brush_plane.tex_axis[0].scale.x + basis[1] * brush_plane.tex_axis[0].scale.y,
				dist: brush_plane.tex_axis[0].offset,
			},
			Plane {
				vec: basis[0] * brush_plane.tex_axis[1].scale.x + basis[1] * brush_plane.tex_axis[1].scale.y,
				dist: brush_plane.tex_axis[1].offset,
			},
		],
		texture: brush_plane.texture.clone(),
	}
}

// See QBSP/MAP.C: TextureAxisFromPlane
fn get_texture_basis(polygon_normal: &Vec3f) -> [Vec3f; 2]
{
	const BASISES: [[Vec3f; 3]; 6] = [
		[
			Vec3f::new(0.0, 0.0, 1.0),
			Vec3f::new(1.0, 0.0, 0.0),
			Vec3f::new(0.0, -1.0, 0.0),
		], // floor
		[
			Vec3f::new(0.0, 0.0, -1.0),
			Vec3f::new(1.0, 0.0, 0.0),
			Vec3f::new(0.0, -1.0, 0.0),
		], // ceiling
		[
			Vec3f::new(1.0, 0.0, 0.0),
			Vec3f::new(0.0, 1.0, 0.0),
			Vec3f::new(0.0, 0.0, -1.0),
		], // west wall
		[
			Vec3f::new(-1.0, 0.0, 0.0),
			Vec3f::new(0.0, 1.0, 0.0),
			Vec3f::new(0.0, 0.0, -1.0),
		], // east wall
		[
			Vec3f::new(0.0, 1.0, 0.0),
			Vec3f::new(1.0, 0.0, 0.0),
			Vec3f::new(0.0, 0.0, -1.0),
		], // south wall
		[
			Vec3f::new(0.0, -1.0, 0.0),
			Vec3f::new(1.0, 0.0, 0.0),
			Vec3f::new(0.0, 0.0, -1.0),
		], // north wall
	];

	let mut best_dot = 0.0;
	let mut best_basis: &[Vec3f; 3] = &BASISES[0];
	for basis in &BASISES
	{
		let dot = polygon_normal.dot(basis[0]);
		if dot > best_dot
		{
			best_basis = basis;
			best_dot = dot;
		}
	}

	[best_basis[1], best_basis[2]]
}
