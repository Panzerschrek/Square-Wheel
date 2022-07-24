use super::{frame_info::*, triangle_model::*};
use common::{bbox::*, clipping::*, math_types::*, plane::*};

pub fn animate_and_transform_triangle_mesh_vertices(
	model: &TriangleModel,
	mesh: &TriangleModelMesh,
	animation: &AnimationPoint,
	matrix: &Mat4f,
	tc_scale: &Vec2f,
	tc_shift: &Vec2f,
	dst_vertices: &mut [ModelVertex3d],
)
{
	let frame0 = animation.frames[0] as usize;
	let frame1 = animation.frames[1] as usize;
	let lerp0 = animation.lerp.max(0.0).min(1.0);
	let lerp1 = 1.0 - lerp0;

	let perform_lerp = lerp0 > 0.01 && lerp0 < 0.99;

	match &mesh.vertex_data
	{
		VertexData::VertexAnimated(va) =>
		{
			let frame_vertex_data0 = &va.variable[frame0 * va.constant.len() .. (frame0 + 1) * va.constant.len()];
			let frame_vertex_data1 = &va.variable[frame1 * va.constant.len() .. (frame1 + 1) * va.constant.len()];

			if perform_lerp
			{
				// Perfomr smooth interpolation.
				for (((v_v0, v_v1), v_c), dst_v) in frame_vertex_data0
					.iter()
					.zip(frame_vertex_data1)
					.zip(va.constant.iter())
					.zip(dst_vertices.iter_mut())
				{
					let position_lerped = v_v0.position * lerp0 + v_v1.position * lerp1;
					let pos_transformed = matrix * position_lerped.extend(1.0);
					*dst_v = ModelVertex3d {
						pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
						tc: Vec2f::from(v_c.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
					};
				}
			}
			else
			{
				// Use single frame.
				let frame_vertex_data = if lerp0 > lerp1
				{
					frame_vertex_data0
				}
				else
				{
					frame_vertex_data1
				};
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
			}
		},
		VertexData::SkeletonAnimated(v) =>
		{
			let frame_bones0 = &model.frame_bones[frame0 * model.bones.len() .. (frame0 + 1) * model.bones.len()];
			let frame_bones1 = &model.frame_bones[frame1 * model.bones.len() .. (frame1 + 1) * model.bones.len()];

			// TODO - use uninitialized memory.
			let mut matrices = [Mat4f::zero(); MAX_TRIANGLE_MODEL_BONES];
			// This code relies on fact that all bones are sorted in hierarchy order.
			if perform_lerp
			{
				// Interpolate between two frames.
				for (bone_index, (frame_bone0, frame_bone1)) in frame_bones0.iter().zip(frame_bones1.iter()).enumerate()
				{
					let parent = model.bones[bone_index].parent as usize;
					// TODO - shouldn't we fix this matrix somehow?
					let mat = frame_bone0.matrix * lerp0 + frame_bone1.matrix * lerp1;
					if parent < model.bones.len()
					{
						matrices[bone_index] = matrices[parent] * mat;
					}
					else
					{
						matrices[bone_index] = mat;
					}
				}
			}
			else
			{
				// Use single frame.
				let frame_bones = if lerp0 > lerp1 { frame_bones0 } else { frame_bones1 };
				for (bone_index, frame_bone) in frame_bones.iter().enumerate()
				{
					let parent = model.bones[bone_index].parent as usize;
					if parent < model.bones.len()
					{
						matrices[bone_index] = matrices[parent] * frame_bone.matrix;
					}
					else
					{
						matrices[bone_index] = frame_bone.matrix;
					}
				}
			}

			let matrix_weight_scaled = matrix * (1.0 / 255.0);
			for bone_index in 0 .. model.bones.len()
			{
				matrices[bone_index] = matrix_weight_scaled * matrices[bone_index];
			}

			for (v, dst_v) in v.iter().zip(dst_vertices.iter_mut())
			{
				let mut mat =
					matrices[v.bones_description[0].bone_index as usize] * (v.bones_description[0].weight as f32);
				for i in 1 .. 4
				{
					mat +=
						matrices[v.bones_description[i].bone_index as usize] * (v.bones_description[i].weight as f32);
				}

				let pos_transformed = mat * v.position.extend(1.0);
				*dst_v = ModelVertex3d {
					pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
					tc: Vec2f::from(v.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
				};
			}
		},
	}
}

pub fn get_current_triangle_model_bbox(model: &TriangleModel, animation: &AnimationPoint) -> BBox
{
	// Just use maximum bbox.
	// TODO - maybe interpolate it instead?
	let mut bbox = model.frames_info[animation.frames[0] as usize].bbox;
	bbox.extend(&model.frames_info[animation.frames[1] as usize].bbox);
	bbox
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
