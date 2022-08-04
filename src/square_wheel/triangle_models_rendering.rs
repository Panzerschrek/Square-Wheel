use super::{fast_math::*, frame_info::*, triangle_model::*};
use common::{bbox::*, bsp_map_compact, clipping::*, clipping_polygon::*, math_types::*, plane::*};

pub fn animate_and_transform_triangle_mesh_vertices(
	model: &TriangleModel,
	mesh: &TriangleModelMesh,
	animation: &AnimationPoint,
	light: &bsp_map_compact::LightGridElement,
	model_matrix: &Mat4f,
	model_view_matrix: &Mat4f,
	tc_scale: &Vec2f,
	tc_shift: &Vec2f,
	dst_vertices: &mut [ModelVertex3d],
)
{
	let normals_matrix = get_normals_matrix(model_matrix);

	let frame0 = animation.frames[0] as usize;
	let frame1 = animation.frames[1] as usize;
	let lerp0 = animation.lerp.max(0.0).min(1.0);
	let lerp1 = 1.0 - lerp0;

	let perform_lerp = lerp0 > 0.01 && lerp0 < 0.99;

	match &mesh.vertex_data
	{
		VertexData::VertexAnimated { constant, variable } =>
		{
			let frame_vertex_data0 = &variable[frame0 * constant.len() .. (frame0 + 1) * constant.len()];
			let frame_vertex_data1 = &variable[frame1 * constant.len() .. (frame1 + 1) * constant.len()];

			if perform_lerp
			{
				// Perform smooth interpolation.
				for (((v_v0, v_v1), v_c), dst_v) in frame_vertex_data0
					.iter()
					.zip(frame_vertex_data1)
					.zip(constant.iter())
					.zip(dst_vertices.iter_mut())
				{
					let position_lerped = v_v0.position * lerp0 + v_v1.position * lerp1;
					let pos_transformed = model_view_matrix * position_lerped.extend(1.0);
					let normal_lerped = v_v0.normal * lerp0 + v_v1.normal * lerp1;
					let normal_transformed = normals_matrix * normal_lerped;
					*dst_v = ModelVertex3d {
						pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
						tc: Vec2f::from(v_c.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
						light: get_vertex_light(light, &normal_transformed),
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
					.zip(constant.iter())
					.zip(dst_vertices.iter_mut())
				{
					let pos_transformed = model_view_matrix * v_v.position.extend(1.0);
					let normal_transformed = normals_matrix * v_v.normal;
					*dst_v = ModelVertex3d {
						pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
						tc: Vec2f::from(v_c.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
						light: get_vertex_light(light, &normal_transformed),
					};
				}
			}
		},
		VertexData::SkeletonAnimated(v) =>
		{
			if model.frame_bones.is_empty()
			{
				// No animation - just use source vertces.
				for (v, dst_v) in v.iter().zip(dst_vertices.iter_mut())
				{
					let pos_transformed = model_view_matrix * v.position.extend(1.0);
					let normal_transformed = normals_matrix * v.normal;
					*dst_v = ModelVertex3d {
						pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
						tc: Vec2f::from(v.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
						light: get_vertex_light(light, &normal_transformed),
					};
				}
			}
			else
			{
				let frame_bones0 = &model.frame_bones[frame0 * model.bones.len() .. (frame0 + 1) * model.bones.len()];
				let frame_bones1 = &model.frame_bones[frame1 * model.bones.len() .. (frame1 + 1) * model.bones.len()];

				// TODO - use uninitialized memory.
				let mut matrices = [Mat4f::zero(); MAX_TRIANGLE_MODEL_BONES];
				// This code relies on fact that all bones are sorted in hierarchy order.
				if perform_lerp
				{
					// Interpolate between two frames.
					for (bone_index, (frame_bone0, frame_bone1)) in
						frame_bones0.iter().zip(frame_bones1.iter()).enumerate()
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

				// TODO - use uninitialized memory.
				let mut normals_matrices = [Mat3f::zero(); MAX_TRIANGLE_MODEL_BONES];

				// Calculate normals matrix based on object normals matrix and bone matrix.
				// Multiply bone matrix by model view matrix.
				let weight_scale = 1.0 / 255.0;
				let normals_matrix_weight_scaled = normals_matrix * weight_scale;
				let model_view_matrix_weight_scaled = model_view_matrix * weight_scale;
				for (bone_matrix, bone_normal_matrix) in matrices.iter_mut().zip(normals_matrices.iter_mut())
				{
					*bone_normal_matrix = normals_matrix_weight_scaled * get_normals_matrix(bone_matrix);
					*bone_matrix = model_view_matrix_weight_scaled * *bone_matrix;
				}

				for (v, dst_v) in v.iter().zip(dst_vertices.iter_mut())
				{
					let i0 = v.bones_description[0].bone_index as usize;
					let w0 = v.bones_description[0].weight as f32;
					let mut mat = matrices[i0] * w0;
					let mut normal_mat = normals_matrices[i0] * w0;
					for i in 1 .. 4
					{
						// Avoid costly matrix operation if weight is zero.
						if v.bones_description[i].weight > 0
						{
							let ii = v.bones_description[i].bone_index as usize;
							let wi = v.bones_description[i].weight as f32;
							mat += matrices[ii] * wi;
							normal_mat += normals_matrices[ii] * wi;
						}
					}

					let pos_transformed = mat * v.position.extend(1.0);
					let normal_transformed = normal_mat * v.normal;
					*dst_v = ModelVertex3d {
						pos: Vec3f::new(pos_transformed.x, pos_transformed.y, pos_transformed.w),
						tc: Vec2f::from(v.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
						light: get_vertex_light(light, &normal_transformed),
					};
				}
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

pub fn calculate_triangle_model_screen_polygon(model_bbox_vertices_transformed: &[Vec3f; 8])
	-> Option<ClippingPolygon>
{
	let mut clipping_polyogn: Option<ClippingPolygon> = None;
	let mut num_front_vertices = 0;
	for v in model_bbox_vertices_transformed
	{
		if v.z > 0.0
		{
			num_front_vertices += 1;

			let point = v.truncate() / v.z;
			if let Some(clipping_polyogn) = &mut clipping_polyogn
			{
				clipping_polyogn.extend_with_point(&point);
			}
			else
			{
				clipping_polyogn = Some(ClippingPolygon::from_point(&point))
			}
		}
	}

	if num_front_vertices == 8 || num_front_vertices == 0
	{
		// Simple case - all bbox vertices are on one side of projection plane.
		return clipping_polyogn;
	}

	let mut clipping_polyogn = if let Some(p) = clipping_polyogn
	{
		p
	}
	else
	{
		return None;
	};

	// Perform z_near clipping of all possible edges between bbox vertices.
	const Z_NEAR: f32 = 1.0 / 4096.0;

	for i in 0 .. 7
	{
		let v_i = &model_bbox_vertices_transformed[i];
		for j in i + 1 .. 8
		{
			let v_j = &model_bbox_vertices_transformed[j];

			if v_i.z <= Z_NEAR && v_j.z <= Z_NEAR
			{
				continue;
			}
			if v_i.z >= Z_NEAR && v_j.z >= Z_NEAR
			{
				continue;
			}

			// This edge is splitted by z_near plane. Clp edge and use intersection point to extend clipping polygon.
			let v = get_line_z_intersection(v_i, v_j, Z_NEAR);
			let point = v.truncate() / v.z;
			clipping_polyogn.extend_with_point(&point);
		}
	}

	Some(clipping_polyogn)
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

fn get_triangle_plane(transformed_vertices: &[ModelVertex3d], triangle: &Triangle) -> Plane
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

pub fn get_model_light(map: &bsp_map_compact::BSPMap, model: &ModelEntity) -> bsp_map_compact::LightGridElement
{
	match model.lighting
	{
		ModelLighting::Default => fetch_light_from_grid(map, &model.position),
		ModelLighting::ConstantLight(l) => bsp_map_compact::LightGridElement {
			light_cube: [l; 6],
			light_direction_vector_scaled: Vec3f::zero(),
			directional_light_color: [0.0; 3],
		},
		ModelLighting::AdvancedLight {
			grid_light_scale,
			light_add,
			position,
		} =>
		{
			let mut result = bsp_map_compact::LightGridElement {
				light_cube: [light_add; 6],
				light_direction_vector_scaled: Vec3f::zero(),
				directional_light_color: [0.0; 3],
			};

			if grid_light_scale > 0.0
			{
				let grid_light = fetch_light_from_grid(map, &position);
				for i in 0 .. 6
				{
					for j in 0 .. 3
					{
						result.light_cube[i][j] += grid_light_scale * grid_light.light_cube[i][j];
					}
				}
				result.light_direction_vector_scaled = grid_light_scale * grid_light.light_direction_vector_scaled;
				result.directional_light_color = grid_light.directional_light_color;
			}

			result
		},
	}
}

fn fetch_light_from_grid(map: &bsp_map_compact::BSPMap, pos: &Vec3f) -> bsp_map_compact::LightGridElement
{
	let zero_light = bsp_map_compact::LightGridElement::default();

	let light_grid_header = &map.light_grid_header;
	if light_grid_header.grid_size[0] == 0 ||
		light_grid_header.grid_size[1] == 0 ||
		light_grid_header.grid_size[2] == 0 ||
		light_grid_header.grid_cell_size[0] == 0.0 ||
		light_grid_header.grid_cell_size[1] == 0.0 ||
		light_grid_header.grid_cell_size[2] == 0.0 ||
		map.light_grid_samples.is_empty() ||
		map.light_grid_columns.is_empty()
	{
		return zero_light;
	}

	let grid_pos = (pos - Vec3f::from(light_grid_header.grid_start))
		.div_element_wise(Vec3f::from(light_grid_header.grid_cell_size));

	let grid_pos_i = [
		grid_pos.x.floor() as i32,
		grid_pos.y.floor() as i32,
		grid_pos.z.floor() as i32,
	];

	// Perform linear interpolation of light grid values.
	// We need to read 8 values in order to do this.
	// Ignore non-existing values and absolute zero values and perform result renormalization.
	let mut total_light = bsp_map_compact::LightGridElement::default();
	let mut total_factor = 0.0;
	for dx in 0 ..= 1
	{
		let x = grid_pos_i[0] + dx;
		if x < 0 || x >= (light_grid_header.grid_size[0] as i32)
		{
			continue;
		}
		let factor_x = 1.0 - (grid_pos.x - (x as f32)).abs();

		for dy in 0 ..= 1
		{
			let y = grid_pos_i[1] + dy;
			if y < 0 || y >= (light_grid_header.grid_size[1] as i32)
			{
				continue;
			}

			let column = map.light_grid_columns[((x as u32) + (y as u32) * light_grid_header.grid_size[0]) as usize];
			if column.num_samples == 0
			{
				continue;
			}

			let factor_y = 1.0 - (grid_pos.y - (y as f32)).abs();

			for dz in 0 ..= 1
			{
				let z = grid_pos_i[2] + dz;
				if z < 0 || z >= (light_grid_header.grid_size[2] as i32)
				{
					continue;
				}

				if (z as u32) < column.start_z || (z as u32) >= column.start_z + column.num_samples
				{
					continue;
				}

				let sample_address_in_column = (z as u32) - column.start_z;
				let sample_value = map.light_grid_samples[(column.first_sample + sample_address_in_column) as usize];
				if sample_value == zero_light
				{
					continue;
				}

				let factor_z = 1.0 - (grid_pos.z - (z as f32)).abs();

				let cur_sample_factor = factor_x * factor_y * factor_z;
				for i in 0 .. 3
				{
					for cube_side in 0 .. 6
					{
						total_light.light_cube[cube_side][i] +=
							sample_value.light_cube[cube_side][i] * cur_sample_factor;
					}
					total_light.directional_light_color[i] +=
						sample_value.directional_light_color[i] * cur_sample_factor;
				}
				// TODO - maybe use different approach to interpolate this vector?
				total_light.light_direction_vector_scaled +=
					sample_value.light_direction_vector_scaled * cur_sample_factor;

				total_factor += cur_sample_factor;
			} // for dz
		} // for dy
	} // for dx

	if total_factor <= 0.0
	{
		return zero_light;
	}
	if total_factor < 0.995
	{
		// Perform normalization in case if same sample points were rejected.
		let inv_total_factor = 1.0 / total_factor;
		for i in 0 .. 3
		{
			for cube_side in 0 .. 6
			{
				total_light.light_cube[cube_side][i] *= inv_total_factor;
			}
			total_light.directional_light_color[i] *= inv_total_factor;
		}
		total_light.light_direction_vector_scaled *= inv_total_factor;
	}

	total_light
}

fn get_vertex_light(light: &bsp_map_compact::LightGridElement, normal_tranformed: &Vec3f) -> [f32; 3]
{
	// After transformation normal may be unnormalized. Renormalize it.
	let normal_normalized = normal_tranformed * inv_sqrt_fast(normal_tranformed.magnitude2().max(0.00000001));

	let mut total_light = [0.0, 0.0, 0.0];
	// Fetch light from cube.
	if normal_normalized.x <= 0.0
	{
		for i in 0 .. 3
		{
			total_light[i] += light.light_cube[0][i] * (-normal_normalized.x);
		}
	}
	else
	{
		for i in 0 .. 3
		{
			total_light[i] += light.light_cube[1][i] * normal_normalized.x;
		}
	}
	if normal_normalized.y <= 0.0
	{
		for i in 0 .. 3
		{
			total_light[i] += light.light_cube[2][i] * (-normal_normalized.y);
		}
	}
	else
	{
		for i in 0 .. 3
		{
			total_light[i] += light.light_cube[3][i] * normal_normalized.y;
		}
	}
	if normal_normalized.z <= 0.0
	{
		for i in 0 .. 3
		{
			total_light[i] += light.light_cube[4][i] * (-normal_normalized.z);
		}
	}
	else
	{
		for i in 0 .. 3
		{
			total_light[i] += light.light_cube[5][i] * normal_normalized.z;
		}
	}

	// Use directional component.
	let light_dir_dot = normal_normalized.dot(light.light_direction_vector_scaled).max(0.0);
	for i in 0 .. 3
	{
		total_light[i] += light.directional_light_color[i] * light_dir_dot;
	}

	total_light
}

fn get_normals_matrix(model_matrix: &Mat4f) -> Mat3f
{
	// TODO - check thid
	let axis_matrix = Mat3f::from_cols(
		model_matrix.x.truncate(),
		model_matrix.y.truncate(),
		model_matrix.z.truncate(),
	);
	axis_matrix.transpose().invert().unwrap_or_else(Mat3f::identity)
}
