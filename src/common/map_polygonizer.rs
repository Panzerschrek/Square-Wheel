use super::{map_file, math_types::*};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Plane
{
	pub vec: Vec3f, // Unnormalized direction
	pub dist: f32,  // for point on plane dot(vec, point) = dist
}

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

fn get_brush_side_plane(brush_side: &map_file::BrushPlane) -> Option<Plane>
{
	let vec = (brush_side.vertices[0] - brush_side.vertices[1]).cross(brush_side.vertices[2] - brush_side.vertices[1]);
	if vec.is_zero()
	{
		return None;
	}

	Some(Plane {
		vec: vec,
		dist: vec.dot(brush_side.vertices[0]),
	})
}

pub fn remove_duplicate_vertices(in_vertices: &[Vec3f]) -> Vec<Vec3f>
{
	const DIST_EPS : f32 = 1.0 / 16.0;
	
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
	let mut result = Vec::new();
	result.push(in_vertices.pop().unwrap());

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

fn get_polygon_texture_info(brush_plane: &map_file::BrushPlane, polygon_normal: &Vec3f) -> TextureInfo
{
	let basis = get_texture_basis(polygon_normal);
	// TODO - apply scale, shift, angle.
	TextureInfo {
		tex_coord_equation: [
			Plane {
				vec: basis[0],
				dist: 0.0,
			},
			Plane {
				vec: basis[1],
				dist: 0.0,
			},
		],
		texture: brush_plane.texture.clone(),
	}
}

// See QBSP/MAP.C: TextureAxisFromPlane
fn get_texture_basis(polygon_normal: &Vec3f) -> [Vec3f; 2]
{
	let mut best_dot = 0.0;
	let mut best_basis = 0;

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

	for i in 0 .. 6
	{
		let dot = polygon_normal.dot(BASISES[i][0]);
		if dot > best_dot
		{
			best_basis = i;
			best_dot = dot;
		}
	}

	[BASISES[best_basis][1], BASISES[best_basis][2]]
}
