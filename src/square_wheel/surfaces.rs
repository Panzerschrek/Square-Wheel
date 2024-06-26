use super::{abstract_color::*, fast_math::*, light::*, textures};
use crate::common::{bsp_map_compact, lightmap, material::*, math_types::*, plane::*};

// Basis vecs, that are used to reconstruct world position of surface texel.
// This also includes normalized normal, that is used for dynamic lighting.
#[derive(Copy, Clone)]
pub struct PolygonBasisVecs
{
	pub u: Vec3f,
	pub v: Vec3f,
	pub start: Vec3f,
	// Normalized
	pub normal: Vec3f,
}

impl PolygonBasisVecs
{
	pub fn form_plane_and_tex_coord_equation(plane: &Plane, tex_coord_equation: &[Plane; 2]) -> Self
	{
		// Calculate inverse matrix for tex_coord equation and plane equation in order to calculate world position for UV.
		let tex_coord_basis = Mat4f::from_cols(
			tex_coord_equation[0].vec.extend(tex_coord_equation[0].dist),
			tex_coord_equation[1].vec.extend(tex_coord_equation[1].dist),
			plane.vec.extend(-plane.dist),
			Vec4f::new(0.0, 0.0, 0.0, 1.0),
		);
		let tex_coord_basis_inverted = if let Some(t) = tex_coord_basis.transpose().invert()
		{
			t
		}
		else
		{
			return PolygonBasisVecs {
				u: Vec3f::unit_x(),
				v: Vec3f::unit_y(),
				start: Vec3f::zero(),
				normal: Vec3f::unit_z(),
			};
		};

		Self {
			u: tex_coord_basis_inverted.x.truncate(),
			v: tex_coord_basis_inverted.y.truncate(),
			start: tex_coord_basis_inverted.w.truncate(),
			normal: plane.vec / plane.vec.magnitude().max(MIN_POSITIVE_VALUE),
		}
	}

	pub fn get_basis_vecs_for_mip(&self, mip: u32) -> Self
	{
		if mip == 0
		{
			return *self;
		}

		let scale = (1 << mip) as f32;
		Self {
			u: self.u * scale,
			v: self.v * scale,
			start: self.start,
			normal: self.normal,
		}
	}
}

pub fn build_surface_simple_lightmap<ColorT: AbstractColor>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_scale_log2: u32,
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	build_surface_impl_2_static_params::<ColorT, LightmapElementOpsSimple>(
		basis_vecs,
		surface_size,
		surface_tc_min,
		texture,
		lightmap_size,
		lightmap_scale_log2,
		lightmap_tc_shift,
		lightmap_data,
		dynamic_lights,
		cam_pos,
		out_surface_data,
	);
}

pub fn build_surface_directional_lightmap<ColorT: AbstractColor>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_scale_log2: u32,
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::DirectionalLightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	build_surface_impl_2_static_params::<ColorT, LightmapElementOpsDirectional>(
		basis_vecs,
		surface_size,
		surface_tc_min,
		texture,
		lightmap_size,
		lightmap_scale_log2,
		lightmap_tc_shift,
		lightmap_data,
		dynamic_lights,
		cam_pos,
		out_surface_data,
	);
}

fn build_surface_impl_2_static_params<ColorT: AbstractColor, LightmapElementOpsT: LightmapElementOps>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_scale_log2: u32,
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	// Perform call in each branch instead of assigning function to function pointer and calling it later because LLVM compiler can't inline call via pointer.
	// Proper inlining is very important here - it can reduce call overhead and merge identical code.
	if lightmap_data.is_empty()
	{
		build_surface_impl_3_static_params::<ColorT, LightmapElementOpsT, NO_LIGHTMAP_SCALE>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else if lightmap_scale_log2 == 0
	{
		build_surface_impl_3_static_params::<ColorT, LightmapElementOpsT, 0>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else if lightmap_scale_log2 == 1
	{
		build_surface_impl_3_static_params::<ColorT, LightmapElementOpsT, 1>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else if lightmap_scale_log2 == 2
	{
		build_surface_impl_3_static_params::<ColorT, LightmapElementOpsT, 2>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else if lightmap_scale_log2 == 3
	{
		build_surface_impl_3_static_params::<ColorT, LightmapElementOpsT, 3>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else if lightmap_scale_log2 == 4
	{
		build_surface_impl_3_static_params::<ColorT, LightmapElementOpsT, 4>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else
	{
		panic!("Wrong lightmap_scale_log2, expected value in range [1; 4]!");
	}
}

fn build_surface_impl_3_static_params<
	ColorT: AbstractColor,
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	if dynamic_lights.is_empty()
	{
		build_surface_impl_4_static_params::<ColorT, LightmapElementOpsT, LIGHTAP_SCALE_LOG2, false>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else
	{
		build_surface_impl_4_static_params::<ColorT, LightmapElementOpsT, LIGHTAP_SCALE_LOG2, true>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
}

fn build_surface_impl_4_static_params<
	ColorT: AbstractColor,
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_DYNAMIC_LIGHTS: bool,
>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	if texture.has_normal_map
	{
		build_surface_impl_5_static_params::<ColorT, LightmapElementOpsT, LIGHTAP_SCALE_LOG2, USE_DYNAMIC_LIGHTS, true>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
	else
	{
		build_surface_impl_5_static_params::<ColorT, LightmapElementOpsT, LIGHTAP_SCALE_LOG2, USE_DYNAMIC_LIGHTS, false>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
}

fn build_surface_impl_5_static_params<
	ColorT: AbstractColor,
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_DYNAMIC_LIGHTS: bool,
	const USE_NORMAL_MAP: bool,
>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	if texture.has_non_one_roughness
	{
		if texture.is_metal
		{
			build_surface_impl_6_static_params::<
				ColorT,
				LightmapElementOpsT,
				LIGHTAP_SCALE_LOG2,
				USE_DYNAMIC_LIGHTS,
				USE_NORMAL_MAP,
				SPECULAR_TYPE_METAL,
			>(
				basis_vecs,
				surface_size,
				surface_tc_min,
				texture,
				lightmap_size,
				lightmap_tc_shift,
				lightmap_data,
				dynamic_lights,
				cam_pos,
				out_surface_data,
			);
		}
		else
		{
			build_surface_impl_6_static_params::<
				ColorT,
				LightmapElementOpsT,
				LIGHTAP_SCALE_LOG2,
				USE_DYNAMIC_LIGHTS,
				USE_NORMAL_MAP,
				SPECULAR_TYPE_DIELECTRIC,
			>(
				basis_vecs,
				surface_size,
				surface_tc_min,
				texture,
				lightmap_size,
				lightmap_tc_shift,
				lightmap_data,
				dynamic_lights,
				cam_pos,
				out_surface_data,
			);
		}
	}
	else
	{
		build_surface_impl_6_static_params::<
			ColorT,
			LightmapElementOpsT,
			LIGHTAP_SCALE_LOG2,
			USE_DYNAMIC_LIGHTS,
			USE_NORMAL_MAP,
			SPECULAR_TYPE_NONE,
		>(
			basis_vecs,
			surface_size,
			surface_tc_min,
			texture,
			lightmap_size,
			lightmap_tc_shift,
			lightmap_data,
			dynamic_lights,
			cam_pos,
			out_surface_data,
		);
	}
}

pub const SPECULAR_TYPE_NONE: u32 = 0;
pub const SPECULAR_TYPE_DIELECTRIC: u32 = 1;
pub const SPECULAR_TYPE_METAL: u32 = 2;

pub const NO_LIGHTMAP_SCALE: u32 = 31;

// Specify various settings as template params in order to get most efficient code for current combination of params.
// Use chained dispatch in order to convert dynamic params into static.
fn build_surface_impl_6_static_params<
	ColorT: AbstractColor,
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_DYNAMIC_LIGHTS: bool,
	const USE_NORMAL_MAP: bool,
	const SPECULAR_TYPE: u32,
>(
	basis_vecs: &PolygonBasisVecs,
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[&DynamicLightWithShadow],
	cam_pos: &Vec3f,
	out_surface_data: &mut [ColorT],
)
{
	let u_vec = &basis_vecs.u;
	let v_vec = &basis_vecs.v;
	// Add 0.5 to get position for pixel centers.
	let start_pos =
		basis_vecs.start + u_vec * ((surface_tc_min[0]) as f32 + 0.5) + v_vec * ((surface_tc_min[1]) as f32 + 0.5);

	// Use texture basis vectors as basis for normal transformation.
	// This may be inaccurate if texture is non-uniformly stretched or shifted, but it is still fine for most cases.
	let u_vec_normalized = u_vec * inv_sqrt_fast(vec3_len2(u_vec).max(MIN_POSITIVE_VALUE));
	let v_vec_normalized = v_vec * inv_sqrt_fast(vec3_len2(v_vec).max(MIN_POSITIVE_VALUE));

	// TODO - use uninitialized memory instead.
	let mut line_lightmap = unsafe {
		std::mem::zeroed::<[LightmapElementOpsT::LightmapElement; (lightmap::MAX_LIGHTMAP_SIZE + 2) as usize]>()
	};
	let lightmap_scale_f = (1 << LIGHTAP_SCALE_LOG2) as f32;
	let inv_lightmap_scale_f = 1.0 / lightmap_scale_f;
	let k_shift = 0.5 * inv_lightmap_scale_f;
	let lightmap_fetch_mask = (1 << LIGHTAP_SCALE_LOG2) - 1;

	for dst_v in 0 .. surface_size[1]
	{
		// Prepare interpolated lightmap values for current line.
		// TODO - skip samples outside current surface borders.
		if LIGHTAP_SCALE_LOG2 != NO_LIGHTMAP_SCALE
		{
			let lightmap_base_v = dst_v + lightmap_tc_shift[1];
			let lightmap_v = lightmap_base_v >> LIGHTAP_SCALE_LOG2;
			let lightmap_v_plus_one = lightmap_v + 1;
			debug_assert!(lightmap_v_plus_one < lightmap_size[1]);
			let k = f32_mul_add(
				(lightmap_base_v & lightmap_fetch_mask) as f32,
				inv_lightmap_scale_f,
				k_shift,
			);
			let base_lightmap_address = lightmap_v * lightmap_size[0];
			for ((dst, l0), l1) in line_lightmap
				.iter_mut()
				.zip(
					&lightmap_data
						[base_lightmap_address as usize .. (base_lightmap_address + lightmap_size[0]) as usize],
				)
				.zip(
					&lightmap_data[(base_lightmap_address + lightmap_size[0]) as usize ..
						(base_lightmap_address + 2 * lightmap_size[0]) as usize],
				)
			{
				*dst = LightmapElementOpsT::mix(l1, l0, k);
			}
		}

		let dst_line_start = (dst_v * surface_size[0]) as usize;
		let dst_line = &mut out_surface_data[dst_line_start .. dst_line_start + (surface_size[0] as usize)];

		let src_v = (surface_tc_min[1] + (dst_v as i32)).rem_euclid(texture.size[1] as i32);
		let src_line_start = ((src_v as u32) * texture.size[0]) as usize;
		let src_line = &texture.pixels[src_line_start .. src_line_start + (texture.size[0] as usize)];
		let mut src_u = surface_tc_min[0].rem_euclid(texture.size[0] as i32);
		let mut dst_u = 0;
		let start_pos_v = vec3_scalar_mul_add(v_vec, dst_v as f32, &start_pos);
		for dst_texel in dst_line.iter_mut()
		{
			let pos = vec3_scalar_mul_add(u_vec, dst_u as f32, &start_pos_v);

			let texel_value = unsafe { debug_only_checked_fetch(src_line, src_u as usize) };
			let (texel_normal, texel_roughness) = if USE_NORMAL_MAP
			{
				texel_value.packed_normal_roughness.unpack()
			}
			else
			{
				if SPECULAR_TYPE == SPECULAR_TYPE_NONE
				{
					(Vec3f::unit_z(), 0.0)
				}
				else
				{
					(Vec3f::unit_z(), texel_value.packed_normal_roughness.unpack_roughness())
				}
			};

			let mut total_light_albedo_modulated = ColorVec::zero();
			let mut total_light_direct = ColorVec::zero();
			if LIGHTAP_SCALE_LOG2 != NO_LIGHTMAP_SCALE
			{
				let lightmap_base_u = dst_u + lightmap_tc_shift[0];
				let lightmap_u = lightmap_base_u >> LIGHTAP_SCALE_LOG2;
				let lightmap_u_plus_one = lightmap_u + 1;
				debug_assert!(lightmap_u_plus_one < lightmap_size[0]);
				let l0 = unsafe { debug_only_checked_fetch(&line_lightmap, lightmap_u as usize) };
				let l1 = unsafe { debug_only_checked_fetch(&line_lightmap, lightmap_u_plus_one as usize) };
				let k = f32_mul_add(
					(lightmap_base_u & lightmap_fetch_mask) as f32,
					inv_lightmap_scale_f,
					k_shift,
				);
				let l_mixed = LightmapElementOpsT::mix(&l1, &l0, k);

				if SPECULAR_TYPE == SPECULAR_TYPE_NONE
				{
					total_light_albedo_modulated =
						ColorVec::from_color_f32x3_with_one(&LightmapElementOpsT::get_constant_component(&l_mixed));
					if let Some(directional_component) = LightmapElementOpsT::get_directional_component(&l_mixed)
					{
						let dot = if USE_NORMAL_MAP
						{
							vec3_dot(&directional_component.vector_scaled, &texel_normal).max(0.0)
						}
						else
						{
							directional_component.vector_scaled.z
						};

						total_light_albedo_modulated = ColorVec::mul_scalar_add(
							&ColorVec::from_color_f32x3(&directional_component.color),
							dot,
							&total_light_albedo_modulated,
						);
					}
				}
				else
				{
					let normal = if USE_NORMAL_MAP
					{
						texel_normal
					}
					else
					{
						Vec3f::new(0.0, 0.0, 1.0)
					};

					let vec_to_camera = cam_pos - pos;
					let vec_to_camera_texture_space = Vec3f::new(
						vec3_dot(&vec_to_camera, &u_vec_normalized),
						vec3_dot(&vec_to_camera, &v_vec_normalized),
						vec3_dot(&vec_to_camera, &basis_vecs.normal),
					);

					let vec_to_camera_normal_dot = vec3_dot(&vec_to_camera_texture_space, &normal);
					let vec_to_camera_reflected =
						normal * (2.0 * vec_to_camera_normal_dot) - vec_to_camera_texture_space;
					let vec_to_camera_len2 = vec3_len2(&vec_to_camera_reflected).max(MIN_POSITIVE_VALUE);

					let vec_to_camera_normal_angle_cos =
						(vec_to_camera_normal_dot * inv_sqrt_fast(vec_to_camera_len2)).max(0.0);
					let fresnel_factor_base = get_fresnel_factor_base(vec_to_camera_normal_angle_cos);

					let specular_k = if SPECULAR_TYPE == SPECULAR_TYPE_DIELECTRIC
					{
						get_specular_k_dielectric(fresnel_factor_base, texel_roughness)
					}
					else if SPECULAR_TYPE == SPECULAR_TYPE_METAL
					{
						get_specular_k_metal(fresnel_factor_base, texel_roughness)
					}
					else
					{
						0.0
					};
					let one_minus_specular_k = 1.0 - specular_k;

					let constant_component =
						ColorVec::from_color_f32x3(&LightmapElementOpsT::get_constant_component(&l_mixed));

					total_light_albedo_modulated = ColorVec::scalar_mul(&constant_component, one_minus_specular_k);
					total_light_direct = ColorVec::scalar_mul(&constant_component, specular_k);

					// Set alpha component to one to preserve alpha.
					total_light_albedo_modulated.insert::<3>(1.0);

					if let Some(directional_component) = LightmapElementOpsT::get_directional_component(&l_mixed)
					{
						let direction_vec_len2 = vec3_len2(&directional_component.vector_scaled);
						let direction_vec_len = direction_vec_len2 * inv_sqrt_fast(direction_vec_len2);

						let vec_to_camera_reflected_light_angle_cos =
							vec3_dot(&vec_to_camera_reflected, &directional_component.vector_scaled) *
								inv_sqrt_fast(vec_to_camera_len2 * direction_vec_len2);

						// Make roughness greater  for light with large deviation.
						let inv_roughness_corrected =
							inv_fast(texel_roughness + directional_component.deviation).max(0.75);

						let specular_intensity =
							get_specular_intensity(vec_to_camera_reflected_light_angle_cos, inv_roughness_corrected);

						let directional_component_color = ColorVec::from_color_f32x3(&directional_component.color);
						if SPECULAR_TYPE == SPECULAR_TYPE_DIELECTRIC
						{
							let diffuse_intensity =
								vec3_dot(&directional_component.vector_scaled, &texel_normal).max(0.0);

							let light_intensity_diffuse = diffuse_intensity * one_minus_specular_k;
							total_light_albedo_modulated = ColorVec::mul_scalar_add(
								&directional_component_color,
								light_intensity_diffuse,
								&total_light_albedo_modulated,
							);

							let light_intensity_specular = specular_intensity * specular_k * direction_vec_len;
							total_light_direct = ColorVec::mul_scalar_add(
								&directional_component_color,
								light_intensity_specular,
								&total_light_direct,
							);
						}
						else if SPECULAR_TYPE == SPECULAR_TYPE_METAL
						{
							let specular_intensity_scale_factor = specular_intensity * direction_vec_len;

							let light_intensity_modulated = one_minus_specular_k * specular_intensity_scale_factor;
							total_light_albedo_modulated = ColorVec::mul_scalar_add(
								&directional_component_color,
								light_intensity_modulated,
								&total_light_albedo_modulated,
							);

							let light_intensity_direct = specular_k * specular_intensity_scale_factor;
							total_light_direct = ColorVec::mul_scalar_add(
								&directional_component_color,
								light_intensity_direct,
								&total_light_direct,
							);
						}
					} // If has directional component.
				} // If specular material.
			} // If has lightmap.

			if USE_DYNAMIC_LIGHTS
			{
				let normal = if USE_NORMAL_MAP
				{
					// Normal transformed to world space.
					Vec3f::new(
						f32_mul_add(
							texel_normal.x,
							u_vec_normalized.x,
							f32_mul_add(texel_normal.y, v_vec_normalized.x, texel_normal.z * basis_vecs.normal.x),
						),
						f32_mul_add(
							texel_normal.x,
							u_vec_normalized.y,
							f32_mul_add(texel_normal.y, v_vec_normalized.y, texel_normal.z * basis_vecs.normal.y),
						),
						f32_mul_add(
							texel_normal.x,
							u_vec_normalized.z,
							f32_mul_add(texel_normal.y, v_vec_normalized.z, texel_normal.z * basis_vecs.normal.z),
						),
					)
				}
				else
				{
					basis_vecs.normal
				};

				let vec_to_camera_reflected;
				let vec_to_camera_len2;
				let specular_k;
				let inv_roughness;

				if SPECULAR_TYPE != SPECULAR_TYPE_NONE
				{
					// Calculate reflected view angle and fresnel factor based on it.
					// Use these data later for calculation o specular light for all dynamic lights.
					let vec_to_camera = cam_pos - pos;
					let vec_to_camera_normal_dot = vec3_dot(&vec_to_camera, &normal);
					vec_to_camera_reflected = normal * (2.0 * vec_to_camera_normal_dot) - vec_to_camera;
					vec_to_camera_len2 = vec3_len2(&vec_to_camera_reflected).max(MIN_POSITIVE_VALUE);

					let vec_to_camera_normal_angle_cos =
						(vec_to_camera_normal_dot * inv_sqrt_fast(vec_to_camera_len2)).max(0.0);

					let fresnel_factor_base = get_fresnel_factor_base(vec_to_camera_normal_angle_cos);
					specular_k = if SPECULAR_TYPE == SPECULAR_TYPE_DIELECTRIC
					{
						get_specular_k_dielectric(fresnel_factor_base, texel_roughness)
					}
					else if SPECULAR_TYPE == SPECULAR_TYPE_METAL
					{
						get_specular_k_metal(fresnel_factor_base, texel_roughness)
					}
					else
					{
						0.0
					};

					inv_roughness = inv_fast(texel_roughness)
				}
				else
				{
					vec_to_camera_reflected = Vec3f::zero();
					vec_to_camera_len2 = MIN_POSITIVE_VALUE;
					specular_k = 0.0;
					inv_roughness = 0.0;
				}

				for light in dynamic_lights
				{
					let vec_to_light = light.position - pos;

					let shadow_factor = get_light_shadow_factor(light, &vec_to_light);
					let vec_to_light_len2 = vec3_len2(&vec_to_light).max(MIN_POSITIVE_VALUE);
					// Limit light radius by subtracting intensity at maximum radius.
					// This is needed to avoid work with lights of infinite radius and instead work with lights with limited size.
					let distance_factor = (inv_fast(vec_to_light_len2) - light.inv_square_radius).max(0.0);
					let shadow_distance_factor = shadow_factor * distance_factor;

					let diffuse_intensity = if SPECULAR_TYPE == SPECULAR_TYPE_METAL
					{
						// No diffuse light for metalls.
						0.0
					}
					else
					{
						(vec3_dot(&normal, &vec_to_light) * inv_sqrt_fast(vec_to_light_len2)).max(0.0)
					};

					let specular_intensity = if SPECULAR_TYPE == SPECULAR_TYPE_NONE
					{
						// No specular for surfaces without specular.
						0.0
					}
					else
					{
						let vec_to_camera_reflected_light_angle_cos = vec3_dot(&vec_to_camera_reflected, &vec_to_light) *
							inv_sqrt_fast(vec_to_camera_len2 * vec_to_light_len2);

						get_specular_intensity(vec_to_camera_reflected_light_angle_cos, inv_roughness)
					};

					let light_color = ColorVec::from_color_f32x3(&light.color);
					match SPECULAR_TYPE
					{
						SPECULAR_TYPE_NONE =>
						{
							total_light_albedo_modulated = ColorVec::mul_scalar_add(
								&light_color,
								diffuse_intensity * shadow_distance_factor,
								&total_light_albedo_modulated,
							);
						},
						SPECULAR_TYPE_DIELECTRIC =>
						{
							let light_intensity_diffuse =
								diffuse_intensity * (1.0 - specular_k) * shadow_distance_factor;
							total_light_albedo_modulated = ColorVec::mul_scalar_add(
								&light_color,
								light_intensity_diffuse,
								&total_light_albedo_modulated,
							);

							let light_intensity_specular = specular_intensity * specular_k * shadow_distance_factor;
							total_light_direct =
								ColorVec::mul_scalar_add(&light_color, light_intensity_specular, &total_light_direct);
						},
						SPECULAR_TYPE_METAL =>
						{
							let specular_intensity_shadow_distance_factor = specular_intensity * shadow_distance_factor;

							let light_intensity_modulated =
								(1.0 - specular_k) * specular_intensity_shadow_distance_factor;
							total_light_albedo_modulated = ColorVec::mul_scalar_add(
								&light_color,
								light_intensity_modulated,
								&total_light_albedo_modulated,
							);

							let light_intensity_direct = specular_k * specular_intensity_shadow_distance_factor;
							total_light_direct =
								ColorVec::mul_scalar_add(&light_color, light_intensity_direct, &total_light_direct);
						},
						_ =>
						{
							panic!("Wrong specular type!")
						},
					}
				} // For dynamic lights.
			} // If use dynmic lights.

			let mut result_color = ColorVec::mul(
				&ColorVec::from_color32(texel_value.diffuse),
				&total_light_albedo_modulated,
			);
			if SPECULAR_TYPE != SPECULAR_TYPE_NONE
			{
				result_color = ColorVec::mul_scalar_add(&total_light_direct, 255.0, &result_color);
			}

			*dst_texel = result_color.into();
			src_u += 1;
			if src_u == (texture.size[0] as i32)
			{
				src_u = 0;
			}

			dst_u += 1;
		}
	}
}

pub fn mix_surface_with_texture<ColorT: AbstractColor>(
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::TextureLite,
	blending_mode: BlendingMode,
	light: [f32; 3],
	surface_data: &mut [ColorT],
)
{
	match blending_mode
	{
		BlendingMode::None => mix_surface_with_texture_impl::<ColorT, BLENDING_MODE_NONE>(
			surface_size,
			surface_tc_min,
			texture,
			light,
			surface_data,
		),
		BlendingMode::Average => mix_surface_with_texture_impl::<ColorT, BLENDING_MODE_AVERAGE>(
			surface_size,
			surface_tc_min,
			texture,
			light,
			surface_data,
		),
		BlendingMode::Additive => mix_surface_with_texture_impl::<ColorT, BLENDING_MODE_ADDITIVE>(
			surface_size,
			surface_tc_min,
			texture,
			light,
			surface_data,
		),
		BlendingMode::AlphaTest => mix_surface_with_texture_impl::<ColorT, BLENDING_MODE_ALPHA_TEST>(
			surface_size,
			surface_tc_min,
			texture,
			light,
			surface_data,
		),
		BlendingMode::AlphaBlend => mix_surface_with_texture_impl::<ColorT, BLENDING_MODE_ALPHA_BLEND>(
			surface_size,
			surface_tc_min,
			texture,
			light,
			surface_data,
		),
	}
}

fn mix_surface_with_texture_impl<ColorT: AbstractColor, const BLENDING_MODE: usize>(
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::TextureLite,
	light: [f32; 3],
	surface_data: &mut [ColorT],
)
{
	const LIGHT_SHIFT: i32 = 8;
	let light_scale = (1 << LIGHT_SHIFT) as f32;
	let light_vec =
		ColorVecI::from_color_f32x3(&[light[0] * light_scale, light[1] * light_scale, light[2] * light_scale]);

	for dst_v in 0 .. surface_size[1]
	{
		let dst_line_start = (dst_v * surface_size[0]) as usize;
		let dst_line = &mut surface_data[dst_line_start .. dst_line_start + (surface_size[0] as usize)];

		let src_v = (surface_tc_min[1] + (dst_v as i32)).rem_euclid(texture.size[1] as i32);
		let src_line_start = ((src_v as u32) * texture.size[0]) as usize;
		let src_line = &texture.pixels[src_line_start .. src_line_start + (texture.size[0] as usize)];
		let mut src_u = surface_tc_min[0].rem_euclid(texture.size[0] as i32);

		for dst_texel in dst_line.iter_mut()
		{
			let texel_value = unsafe { debug_only_checked_fetch(src_line, src_u as usize) };
			let texel_value_modulated = ColorVecI::shift_right::<LIGHT_SHIFT>(&ColorVecI::mul(
				&ColorVecI::from_color32(texel_value),
				&light_vec,
			));

			if BLENDING_MODE == BLENDING_MODE_NONE
			{
				*dst_texel = texel_value_modulated.into();
			}
			else if BLENDING_MODE == BLENDING_MODE_AVERAGE
			{
				*dst_texel =
					ColorVecI::shift_right::<1>(&ColorVecI::add(&texel_value_modulated, &(*dst_texel).into())).into();
			}
			else if BLENDING_MODE == BLENDING_MODE_ADDITIVE
			{
				*dst_texel = ColorVecI::add(&texel_value_modulated, &(*dst_texel).into()).into();
			}
			else if BLENDING_MODE == BLENDING_MODE_ALPHA_TEST
			{
				if texel_value.test_alpha()
				{
					*dst_texel = texel_value_modulated.into();
				}
			}
			else if BLENDING_MODE == BLENDING_MODE_ALPHA_BLEND
			{
				let alpha = texel_value.get_alpha();
				*dst_texel = ColorVecI::shift_right::<8>(&ColorVecI::add(
					&ColorVecI::mul_scalar(&texel_value_modulated, alpha),
					&ColorVecI::mul_scalar(&(*dst_texel).into(), 255 - alpha),
				))
				.into();
			}

			src_u += 1;
			if src_u == (texture.size[0] as i32)
			{
				src_u = 0;
			}
		}
	}
}

trait LightmapElementOps
{
	type LightmapElement: Copy;

	fn mix(a: &Self::LightmapElement, b: &Self::LightmapElement, ratio: f32) -> Self::LightmapElement;

	fn get_constant_component(el: &Self::LightmapElement) -> [f32; 3];

	fn get_directional_component(el: &Self::LightmapElement) -> Option<LightmapDirectionalComponent>;
}

struct LightmapDirectionalComponent
{
	vector_scaled: Vec3f,
	deviation: f32,
	color: [f32; 3],
}

struct LightmapElementOpsSimple;
impl LightmapElementOps for LightmapElementOpsSimple
{
	type LightmapElement = [f32; 3];

	fn mix(a: &Self::LightmapElement, b: &Self::LightmapElement, ratio: f32) -> Self::LightmapElement
	{
		let one_minus_ratio = 1.0 - ratio;
		[
			f32_mul_add(a[0], ratio, b[0] * one_minus_ratio),
			f32_mul_add(a[1], ratio, b[1] * one_minus_ratio),
			f32_mul_add(a[2], ratio, b[2] * one_minus_ratio),
		]
	}

	fn get_constant_component(el: &Self::LightmapElement) -> [f32; 3]
	{
		*el
	}

	fn get_directional_component(_el: &Self::LightmapElement) -> Option<LightmapDirectionalComponent>
	{
		None
	}
}

struct LightmapElementOpsDirectional;
impl LightmapElementOps for LightmapElementOpsDirectional
{
	type LightmapElement = bsp_map_compact::DirectionalLightmapElement;

	fn mix(a: &Self::LightmapElement, b: &Self::LightmapElement, ratio: f32) -> Self::LightmapElement
	{
		let one_minus_ratio = 1.0 - ratio;
		Self::LightmapElement {
			ambient_light: [
				f32_mul_add(a.ambient_light[0], ratio, b.ambient_light[0] * one_minus_ratio),
				f32_mul_add(a.ambient_light[1], ratio, b.ambient_light[1] * one_minus_ratio),
				f32_mul_add(a.ambient_light[2], ratio, b.ambient_light[2] * one_minus_ratio),
			],
			light_direction_vector_scaled: Vec3f::new(
				f32_mul_add(
					a.light_direction_vector_scaled.x,
					ratio,
					b.light_direction_vector_scaled.x * one_minus_ratio,
				),
				f32_mul_add(
					a.light_direction_vector_scaled.y,
					ratio,
					b.light_direction_vector_scaled.y * one_minus_ratio,
				),
				f32_mul_add(
					a.light_direction_vector_scaled.z,
					ratio,
					b.light_direction_vector_scaled.z * one_minus_ratio,
				),
			),
			directional_light_deviation: f32_mul_add(
				a.directional_light_deviation,
				ratio,
				b.directional_light_deviation * one_minus_ratio,
			),
			directional_light_color: [
				f32_mul_add(
					a.directional_light_color[0],
					ratio,
					b.directional_light_color[0] * one_minus_ratio,
				),
				f32_mul_add(
					a.directional_light_color[1],
					ratio,
					b.directional_light_color[1] * one_minus_ratio,
				),
				f32_mul_add(
					a.directional_light_color[2],
					ratio,
					b.directional_light_color[2] * one_minus_ratio,
				),
			],
		}
	}

	fn get_constant_component(el: &Self::LightmapElement) -> [f32; 3]
	{
		el.ambient_light
	}

	fn get_directional_component(el: &Self::LightmapElement) -> Option<LightmapDirectionalComponent>
	{
		Some(LightmapDirectionalComponent {
			vector_scaled: el.light_direction_vector_scaled,
			deviation: el.directional_light_deviation,
			color: el.directional_light_color,
		})
	}
}

fn get_specular_intensity(vec_to_camera_reflected_light_angle_cos: f32, inv_roughness: f32) -> f32
{
	if false
	{
		// Old formula. Specular is bounded.
		let x = ((vec_to_camera_reflected_light_angle_cos - 1.0) * inv_roughness).max(-2.0);

		// With susch params f(-2) = 0, f(0) = 0.75, integral(-2, 0) = 0.5.
		// Integral of this function (multiplied by 2 * Pi) over sphere must be identical to integral for diffuse light over hemisphere (Lambertian law).
		let a = 0.1875;
		let b = 0.75;
		let c = 0.75;
		f32_mul_add(x, f32_mul_add(x, a, b), c) * inv_roughness
	}
	else
	{
		// New formula. Just a little bit slower, specular is unbounded.
		let x = (vec_to_camera_reflected_light_angle_cos - 1.0) * inv_roughness;

		// Integral of this function is equal to 1 / 2 in range (-inf, 0] and 3 / 8 in range [-2; 0].
		const SQRT_7: f32 = 2.645751311;
		const A: f32 = 2.0 * std::f32::consts::PI / SQRT_7;
		const B: f32 = std::f32::consts::PI * SQRT_7 / 2.0;

		inv_fast(f32_mul_add(x * x, B, A)) * inv_roughness
	}
}

fn get_fresnel_factor_base(vec_to_camera_normal_angle_cos: f32) -> f32
{
	// Schlick's approximation of Fresnel factor.
	// See https://en.wikipedia.org/wiki/Schlick%27s_approximation.
	let one_minus_angle_cos = (1.0 - vec_to_camera_normal_angle_cos).max(0.0);
	let one_minus_angle_cos2 = one_minus_angle_cos * one_minus_angle_cos;
	one_minus_angle_cos2 * one_minus_angle_cos2 * one_minus_angle_cos
}

fn get_specular_k_dielectric(fresnel_factor_base: f32, roughness: f32) -> f32
{
	let fresnel_factor = f32_mul_add(
		fresnel_factor_base,
		1.0 - DIELECTRIC_ZERO_REFLECTIVITY,
		DIELECTRIC_ZERO_REFLECTIVITY,
	);

	// For glossy surface we can just use Fresnel factor for diffuse/specular mixing.
	// But for rough srufaces we can't. Normally we should use some sort of integral of Schlick's approximation.
	// But it's too expensive. So, just make mix of Fresnel factor depending on view angle with constant factor for absolutely rough surface.
	f32_mul_add(
		fresnel_factor,
		1.0 - roughness,
		DIELECTRIC_AVERAGE_REFLECTIVITY * roughness,
	)
}

fn get_specular_k_metal(fresnel_factor_base: f32, roughness: f32) -> f32
{
	f32_mul_add(
		fresnel_factor_base,
		1.0 - roughness,
		METAL_AVERAGE_SCHLICK_FACTOR * roughness,
	)
}

const DIELECTRIC_ZERO_REFLECTIVITY: f32 = 0.04;
const DIELECTRIC_AVERAGE_REFLECTIVITY: f32 = DIELECTRIC_ZERO_REFLECTIVITY * 3.0;
const METAL_AVERAGE_SCHLICK_FACTOR: f32 = 0.5;
