use super::triangle_model::*;
use common::{clipping::*, math_types::*, plane::*};

pub fn reject_triangle_model_back_faces(
	transformed_vertices: &[ModelVertex3d],
	triangles: &[Triangle],
	out_triangles: &mut [Triangle],
) -> usize
{
	let mut num_visible_triangles = 0;
	for triangle in triangles
	{
		// TODO - maybe also reject triangles outside screen borders?
		if get_triangle_plane(&transformed_vertices, triangle).dist > 0.0
		{
			out_triangles[num_visible_triangles] = *triangle;
			num_visible_triangles += 1;
		}
	}

	num_visible_triangles
}

pub fn sort_model_triangles(transformed_vertices: &[ModelVertex3d], triangles: &mut [Triangle])
{
	// Dumb triangles sorting, using Z coordinate.
	// TODO - try to use other criterias - min_z, center_z, min_z + max_z ...

	triangles.sort_by(|a, b| {
		let a_z = triangle_vertex_debug_checked_fetch(transformed_vertices, a[0])
			.pos
			.z
			.max(triangle_vertex_debug_checked_fetch(transformed_vertices, a[1]).pos.z)
			.max(triangle_vertex_debug_checked_fetch(transformed_vertices, a[2]).pos.z);
		let b_z = triangle_vertex_debug_checked_fetch(transformed_vertices, b[0])
			.pos
			.z
			.max(triangle_vertex_debug_checked_fetch(transformed_vertices, b[1]).pos.z)
			.max(triangle_vertex_debug_checked_fetch(transformed_vertices, b[2]).pos.z);
		// TODO - avoid unwrap.
		b_z.partial_cmp(&a_z).unwrap()
	});
}

pub fn get_triangle_plane(transformed_vertices: &[ModelVertex3d], triangle: &Triangle) -> Plane
{
	let v0 = triangle_vertex_debug_checked_fetch(transformed_vertices, triangle[0]);
	let v1 = triangle_vertex_debug_checked_fetch(transformed_vertices, triangle[1]);
	let v2 = triangle_vertex_debug_checked_fetch(transformed_vertices, triangle[2]);
	let vec = (v1.pos - v0.pos).cross(v2.pos - v1.pos);
	let dist = vec.dot(v0.pos);
	Plane { vec, dist }
}

pub fn triangle_vertex_debug_checked_fetch<VertexT: Copy>(vertices: &[VertexT], index: VertexIndex) -> VertexT
{
	let index_s = index as usize;
	#[cfg(debug_assertions)]
	{
		vertices[index_s]
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		*vertices.get_unchecked(index_s)
	}
}
