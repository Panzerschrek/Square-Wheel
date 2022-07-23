use super::triangle_model::*;
use common::{clipping::*, math_types::*, plane::*};

pub fn animate_and_transform_triangle_mesh_vertices(
	mesh: &TriangleModelMesh,
	frame: usize,
	matrix: &Mat4f,
	tc_scale: &Vec2f,
	tc_shift: &Vec2f,
	dst_vertices: &mut [ModelVertex3d],
)
{
	match &mesh.vertex_data
	{
		VertexData::VertexAnimated(va) =>
		{
			let frame_vertex_data = &va.variable[frame * va.constant.len() .. (frame + 1) * va.constant.len()];

			for ((v_v, v_c), dst_v) in frame_vertex_data
				.iter()
				.zip(va.constant.iter())
				.zip(dst_vertices.iter_mut())
			{
				let pos_transformed = matrix * v_v.position.extend(1.0);
				*dst_v = ModelVertex3d {
					pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
					tc: Vec2f::from(v_c.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
				};
			}
		},
		VertexData::SkeletonAnimated(v) =>
		{
			// TODO - perform proper animation.
			for (v, dst_v) in v.iter().zip(dst_vertices.iter_mut())
			{
				let pos_transformed = matrix * v.position.extend(1.0);
				*dst_v = ModelVertex3d {
					pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
					tc: Vec2f::from(v.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
				};
			}
		},
	}
}

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
