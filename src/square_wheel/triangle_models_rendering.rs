use super::{fast_math::*, frame_info::*, light::*, textures::*, triangle_model::*};
use crate::common::{
	bbox::*, bsp_map_compact, clipping::*, clipping_polygon::*, light_cube::*, math_types::*, matrix::*, plane::*,
};
use std::mem::MaybeUninit;

pub fn animate_and_transform_triangle_mesh_vertices(
	model: &TriangleModel,
	mesh: &TriangleModelMesh,
	animation: &AnimationPoint,
	light: &ModelLightData,
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
		VertexData::NonAnimated(v) =>
		{
			for (v, dst_v) in v.iter().zip(dst_vertices.iter_mut())
			{
				let normal_transformed = normals_matrix * v.normal;
				*dst_v = ModelVertex3d {
					pos: view_matrix_transform_vertex(model_view_matrix, &v.position),
					tc: Vec2f::from(v.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
					light: get_vertex_light(light, &normal_transformed),
				};
			}
		},
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
					let normal_lerped = v_v0.normal * lerp0 + v_v1.normal * lerp1;
					let normal_transformed = normals_matrix * normal_lerped;
					*dst_v = ModelVertex3d {
						pos: view_matrix_transform_vertex(model_view_matrix, &position_lerped),
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
					let normal_transformed = normals_matrix * v_v.normal;
					*dst_v = ModelVertex3d {
						pos: view_matrix_transform_vertex(model_view_matrix, &v_v.position),
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
					let normal_transformed = normals_matrix * v.normal;
					*dst_v = ModelVertex3d {
						pos: view_matrix_transform_vertex(model_view_matrix, &v.position),
						tc: Vec2f::from(v.tex_coord).mul_element_wise(*tc_scale) + tc_shift,
						light: get_vertex_light(light, &normal_transformed),
					};
				}
			}
			else
			{
				let num_bones = model.bones.len();
				debug_assert!(num_bones <= MAX_TRIANGLE_MODEL_BONES);
				let frame_bones0 = &model.frame_bones[frame0 * num_bones .. (frame0 + 1) * num_bones];
				let frame_bones1 = &model.frame_bones[frame1 * num_bones .. (frame1 + 1) * num_bones];

				// Use uninitialized memory for buffers of matrices.
				// It is important, since it is is too costly to use safe zeroing,
				// because size of these buffers is currently HUGE (25600 bytes).
				// TODO - check this is a proper way to create really-uninitialized stack buffers.
				let mut matrices = [MaybeUninit::<Mat4f>::uninit(); MAX_TRIANGLE_MODEL_BONES];
				let mut normals_matrices = [MaybeUninit::<Mat3f>::uninit(); MAX_TRIANGLE_MODEL_BONES];

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
							debug_assert!(parent < bone_index);
							matrices[bone_index].write(unsafe { matrices[parent].assume_init_ref() * mat });
						}
						else
						{
							matrices[bone_index].write(mat);
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
							debug_assert!(parent < bone_index);
							matrices[bone_index]
								.write(unsafe { matrices[parent].assume_init_ref() * frame_bone.matrix });
						}
						else
						{
							matrices[bone_index].write(frame_bone.matrix);
						}
					}
				}

				// Calculate normals matrix based on object normals matrix and bone matrix.
				// Multiply bone matrix by model view matrix.
				let weight_scale = 1.0 / 255.0;
				let normals_matrix_weight_scaled = normals_matrix * weight_scale;
				let model_view_matrix_weight_scaled = model_view_matrix * weight_scale;
				for (bone_matrix, bone_normal_matrix) in
					matrices.iter_mut().zip(normals_matrices.iter_mut()).take(num_bones)
				{
					bone_normal_matrix.write(unsafe {
						normals_matrix_weight_scaled * get_normals_matrix(bone_matrix.assume_init_ref())
					});
					bone_matrix.write(unsafe { model_view_matrix_weight_scaled * bone_matrix.assume_init_ref() });
				}

				for (v, dst_v) in v.iter().zip(dst_vertices.iter_mut())
				{
					let i0 = v.bones_description[0].bone_index as usize;
					let w0 = v.bones_description[0].weight as f32;
					let mut mat = unsafe { matrices[i0].assume_init_ref() * w0 };
					let mut normal_mat = unsafe { normals_matrices[i0].assume_init_ref() * w0 };
					for i in 1 .. 4
					{
						// Avoid costly matrix operation if weight is zero.
						if v.bones_description[i].weight > 0
						{
							let ii = v.bones_description[i].bone_index as usize;
							let wi = v.bones_description[i].weight as f32;
							mat += unsafe { matrices[ii].assume_init_ref() * wi };
							normal_mat += unsafe { normals_matrices[ii].assume_init_ref() * wi };
						}
					}

					let normal_transformed = normal_mat * v.normal;
					*dst_v = ModelVertex3d {
						pos: view_matrix_transform_vertex(&mat, &v.position),
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

	let mut clipping_polyogn = clipping_polyogn?;

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

			// This edge is splitted by z_near plane. Cilp edge and use intersection point to extend clipping polygon.
			let v = get_line_z_intersection(v_i, v_j, Z_NEAR);
			let point = v.truncate() / v.z;
			clipping_polyogn.extend_with_point(&point);
		}
	}

	Some(clipping_polyogn)
}

pub fn calculate_triangle_model_texture_mip(
	model_view_matrix: &Mat4f,
	model_bbox: &BBox,
	texture_size: [u32; 2],
	mip_bias: f32,
) -> u32
{
	// Make a cube from bbox n order to fix problems with large mip selected for thin model viewed from side.
	// This method is not so accuratr but it produces good enough results.
	let bbox_center = model_bbox.get_center();
	let bbox_half_size = model_bbox.get_size() * 0.5;

	let cube_half_size = (bbox_half_size.x + bbox_half_size.y + bbox_half_size.z) / 3.0;
	let cube_half_size_vec = Vec3f::new(cube_half_size, cube_half_size, cube_half_size);
	let cube_bbox = BBox::from_min_max(bbox_center - cube_half_size_vec, bbox_center + cube_half_size_vec);

	let mut bbox_vertices_projected = [Vec2f::zero(); 8];
	for (dst, src) in bbox_vertices_projected.iter_mut().zip(cube_bbox.get_corners_vertices())
	{
		let vertex_projected = model_view_matrix * src.extend(1.0);
		if vertex_projected.w <= 0.0
		{
			return 0;
		}
		*dst = Vec2f::new(vertex_projected.x, vertex_projected.y) / vertex_projected.w;
	}

	let mut min_x = bbox_vertices_projected[0].x;
	let mut max_x = min_x;
	let mut min_y = bbox_vertices_projected[0].y;
	let mut max_y = min_y;
	for v in &bbox_vertices_projected[1 ..]
	{
		min_x = min_x.min(v.x);
		max_x = max_x.max(v.x);
		min_y = min_y.min(v.y);
		max_y = max_y.max(v.y);
	}

	let max_dimension = (max_x - min_x).max(max_y - min_y);
	if max_dimension <= 0.0
	{
		return MAX_MIP as u32;
	}

	let approximate_texels_per_pixel = (texture_size[0].max(texture_size[1]) as f32) / max_dimension;

	((approximate_texels_per_pixel.log2() + mip_bias).floor().max(0.0) as u32).min(MAX_MIP as u32)
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
		if get_triangle_plane(transformed_vertices, triangle).dist > 0.0
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

	// Use unstable sorting, since it does not allocate.
	// It is fine to reorder vertices with equal Z because this is almost impossible to have same Z.

	triangles.sort_unstable_by(|a, b| {
		// Compare max z of two triangles.
		// TODO - try to use other criterias - min_z, center_z, min_z + max_z ...
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
	unsafe { debug_only_checked_fetch(vertices, index as usize) }
}

pub struct ModelLightData
{
	// Same as in LightGridElement
	light_cube: [[f32; 3]; 6],
	// Store two directional components - for static and dynamic light.
	directional_components: [ModeLightDirectionalComponent; 2],
}

impl ModelLightData
{
	fn from_two_light_grid_elements(l: &[bsp_map_compact::LightGridElement; 2]) -> Self
	{
		// Combine light cubes together.
		let mut light_cube = l[0].light_cube;
		for side in 0 .. 6
		{
			for c in 0 .. 3
			{
				light_cube[side][c] += l[1].light_cube[side][c];
			}
		}

		Self {
			light_cube,
			directional_components: [
				ModeLightDirectionalComponent::from_light_grid_directional_component(&l[0]),
				ModeLightDirectionalComponent::from_light_grid_directional_component(&l[1]),
			],
		}
	}
}

struct ModeLightDirectionalComponent
{
	vector_scaled: Vec3f,
	color: [f32; 3],
}

impl ModeLightDirectionalComponent
{
	fn from_light_grid_directional_component(l: &bsp_map_compact::LightGridElement) -> Self
	{
		Self {
			vector_scaled: l.light_direction_vector_scaled,
			color: l.directional_light_color,
		}
	}
}

pub fn get_model_light(
	map: &bsp_map_compact::BSPMap,
	dynamic_lights: &[DynamicLightWithShadow],
	model: &ModelEntity,
	model_matrix: &Mat4f,
) -> ModelLightData
{
	ModelLightData::from_two_light_grid_elements(&[
		get_model_static_light(map, model),
		get_model_dynamic_light(dynamic_lights, model, model_matrix),
	])
}

fn get_model_dynamic_light(
	lights: &[DynamicLightWithShadow],
	model: &ModelEntity,
	model_matrix: &Mat4f,
) -> bsp_map_compact::LightGridElement
{
	let mut light_cube = LightCube::new();
	match model.lighting
	{
		ModelLighting::Default =>
		{
			calculate_model_dynamic_light_cube(lights, model, model_matrix, &mut light_cube);
		},
		ModelLighting::ConstantLight(l) =>
		{
			light_cube.add_constant_light(&l);
		},
		ModelLighting::AdvancedLight {
			grid_light_scale,
			light_add,
			position: _,
		} =>
		{
			// Do not use custom position here, because it is nearly useless for dynamic lighting.

			calculate_model_dynamic_light_cube(lights, model, model_matrix, &mut light_cube);
			light_cube.scale(grid_light_scale);
			light_cube.add_constant_light(&light_add);
		},
	}

	light_cube.convert_into_light_grid_sample()
}

fn calculate_model_dynamic_light_cube(
	lights: &[DynamicLightWithShadow],
	model: &ModelEntity,
	model_matrix: &Mat4f,
	light_cube: &mut LightCube,
)
{
	let bbox = get_current_triangle_model_bbox(&model.model, &model.animation);
	let bbox_center = bbox.get_center();

	// Calculage light for several positions within model bbox.
	// Obtain positions in between bbox center and bbox vertices.
	// TODO - use also bbox  center as sample position.
	let sample_positions = bbox
		.get_corners_vertices()
		.map(|v| (model_matrix * ((v + bbox_center) * 0.5).extend(1.0)).truncate());

	let min_square_distance = bbox.get_size().magnitude2() * 0.25;

	for light in lights
	{
		for position in &sample_positions
		{
			let vec_to_light = light.position - position;
			let square_dist = vec_to_light.magnitude2().max(min_square_distance);
			let inv_square_dist = 1.0 / square_dist;
			if inv_square_dist < light.inv_square_radius
			{
				continue;
			}
			let shadow_factor = get_light_shadow_factor(light, &vec_to_light);
			if shadow_factor <= 0.0
			{
				continue;
			}

			let scale = shadow_factor * (inv_square_dist - light.inv_square_radius);
			light_cube.add_light_sample(
				&vec_to_light,
				&[light.color[0] * scale, light.color[1] * scale, light.color[2] * scale],
			);
		}
	}

	light_cube.scale(1.0 / (sample_positions.len() as f32));
}

fn get_model_static_light(map: &bsp_map_compact::BSPMap, model: &ModelEntity) -> bsp_map_compact::LightGridElement
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

pub fn fetch_light_from_grid(map: &bsp_map_compact::BSPMap, pos: &Vec3f) -> bsp_map_compact::LightGridElement
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

fn get_vertex_light(light: &ModelLightData, normal_tranformed: &Vec3f) -> [f32; 3]
{
	// After transformation normal may be unnormalized. Renormalize it.
	let normal_normalized = normal_tranformed * inv_sqrt_fast(normal_tranformed.magnitude2().max(0.00000001));

	// Fetch light from cube.
	let mut total_light = get_light_cube_light(&light.light_cube, &normal_normalized);

	for directional_component in &light.directional_components
	{
		let light_dir_dot = normal_normalized.dot(directional_component.vector_scaled).max(0.0);
		for i in 0 .. 3
		{
			total_light[i] += directional_component.color[i] * light_dir_dot;
		}
	}

	total_light
}

fn get_normals_matrix(model_matrix: &Mat4f) -> Mat3f
{
	// TODO - check this
	let axis_matrix = Mat3f::from_cols(
		model_matrix.x.truncate(),
		model_matrix.y.truncate(),
		model_matrix.z.truncate(),
	);
	axis_matrix.transpose().invert().unwrap_or_else(Mat3f::identity)
}
