use super::{light::*, shadow_map::*, textures};
use common::{bsp_map_compact, color::*, lightmaps_builder, math_types::*, plane::*};

pub type LightWithShadowMap<'a, 'b> = (&'a PointLight, &'b CubeShadowMap);

pub fn build_surface(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_scale_log2: u32,
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	// Perform call in each branch instead of assigning function to function pointer and calling it later because LLVM compiler can't inline call via pointer.
	// Proper inlining is very important here - it can reduce call overhead and merge identical code.
	if lightmap_scale_log2 == 0
	{
		build_surface_impl_1_static_params::<0>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_1_static_params::<1>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_1_static_params::<2>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_1_static_params::<3>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_1_static_params::<4>(
			plane,
			tex_coord_equation,
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

fn build_surface_impl_1_static_params<const LIGHTAP_SCALE_LOG2: u32>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if lightmap_data.is_empty()
	{
		build_surface_impl_2_static_params::<LIGHTAP_SCALE_LOG2, false>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_2_static_params::<LIGHTAP_SCALE_LOG2, true>(
			plane,
			tex_coord_equation,
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

fn build_surface_impl_2_static_params<const LIGHTAP_SCALE_LOG2: u32, const USE_LIGHTMAP: bool>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if dynamic_lights.is_empty()
	{
		build_surface_impl_3_static_params::<LIGHTAP_SCALE_LOG2, USE_LIGHTMAP, false>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_3_static_params::<LIGHTAP_SCALE_LOG2, USE_LIGHTMAP, true>(
			plane,
			tex_coord_equation,
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

fn build_surface_impl_3_static_params<
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_LIGHTMAP: bool,
	const USE_DYNAMIC_LIGHTS: bool,
>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if texture.has_normal_map
	{
		build_surface_impl_4_static_params::<LIGHTAP_SCALE_LOG2, USE_LIGHTMAP, USE_DYNAMIC_LIGHTS, true>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_4_static_params::<LIGHTAP_SCALE_LOG2, USE_LIGHTMAP, USE_DYNAMIC_LIGHTS, false>(
			plane,
			tex_coord_equation,
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
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_LIGHTMAP: bool,
	const USE_DYNAMIC_LIGHTS: bool,
	const USE_NORMAL_MAP: bool,
>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if texture.has_non_zero_glossiness
	{
		build_surface_impl_5_static_params::<LIGHTAP_SCALE_LOG2, USE_LIGHTMAP, USE_DYNAMIC_LIGHTS, USE_NORMAL_MAP, true>(
			plane,
			tex_coord_equation,
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
		build_surface_impl_5_static_params::<LIGHTAP_SCALE_LOG2, USE_LIGHTMAP, USE_DYNAMIC_LIGHTS, USE_NORMAL_MAP, false>(
			plane,
			tex_coord_equation,
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

// Specify various settings as template params in order to get most efficient code for current combination of params.
// Use chained dispatch in order to convert dynamic params into static.
fn build_surface_impl_5_static_params<
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_LIGHTMAP: bool,
	const USE_DYNAMIC_LIGHTS: bool,
	const USE_NORMAL_MAP: bool,
	const USE_SPECULAR: bool,
>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	// Calculate inverse matrix for tex_coord equation and plane equation in order to calculate world position for UV.
	// TODO - project tc equation to surface plane?
	let tex_coord_basis = Mat4f::from_cols(
		tex_coord_equation[0].vec.extend(tex_coord_equation[0].dist),
		tex_coord_equation[1].vec.extend(tex_coord_equation[1].dist),
		plane.vec.extend(-plane.dist),
		Vec4f::new(0.0, 0.0, 0.0, 1.0),
	);
	let tex_coord_basis_inverted_opt = tex_coord_basis.transpose().invert();
	if tex_coord_basis_inverted_opt.is_none()
	{
		return;
	}
	let tex_coord_basis_inverted = tex_coord_basis_inverted_opt.unwrap();

	let u_vec = tex_coord_basis_inverted.x.truncate();
	let v_vec = tex_coord_basis_inverted.y.truncate();
	let start_pos = tex_coord_basis_inverted.w.truncate() +
		u_vec * ((surface_tc_min[0]) as f32 + 0.5) +
		v_vec * ((surface_tc_min[1]) as f32 + 0.5);

	let plane_normal_normalized = plane.vec * inv_sqrt_fast(plane.vec.magnitude2());

	// Use texture basis vectors as basis for normal transformation.
	// This may be inaccurate if texture is non-uniformly stretched or shifted, but it is still fine for most cases.
	let u_vec_normalized = u_vec * inv_sqrt_fast(u_vec.magnitude2().max(MIN_POSITIVE_VALUE));
	let v_vec_normalized = v_vec * inv_sqrt_fast(v_vec.magnitude2().max(MIN_POSITIVE_VALUE));

	// TODO - use uninitialized memory.
	let mut line_lightmap = [[0.0, 0.0, 0.0]; (lightmaps_builder::MAX_LIGHTMAP_SIZE + 2) as usize];
	let lightmap_scale_f = (1 << LIGHTAP_SCALE_LOG2) as f32;
	let inv_lightmap_scale_f = 1.0 / lightmap_scale_f;
	let k_shift = 0.5 * inv_lightmap_scale_f;
	let lightmap_fetch_mask = (1 << LIGHTAP_SCALE_LOG2) - 1;

	for dst_v in 0 .. surface_size[1]
	{
		// Prepare interpolated lightmap values for current line.
		// TODO - skip samples outside current surface borders.
		if USE_LIGHTMAP
		{
			let lightmap_base_v = dst_v + lightmap_tc_shift[1];
			let lightmap_v = lightmap_base_v >> LIGHTAP_SCALE_LOG2;
			let lightmap_v_plus_one = lightmap_v + 1;
			debug_assert!(lightmap_v_plus_one < lightmap_size[1]);
			let k = ((lightmap_base_v & lightmap_fetch_mask) as f32) * inv_lightmap_scale_f + k_shift;
			let k_minus_one = 1.0 - k;
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
				for i in 0 .. 3
				{
					dst[i] = l0[i] * k_minus_one + l1[i] * k;
				}
			}
		}

		let dst_line_start = (dst_v * surface_size[0]) as usize;
		let dst_line = &mut out_surface_data[dst_line_start .. dst_line_start + (surface_size[0] as usize)];

		let src_v = (surface_tc_min[1] + (dst_v as i32)).rem_euclid(texture.size[1] as i32);
		let src_line_start = ((src_v as u32) * texture.size[0]) as usize;
		let src_line = &texture.pixels[src_line_start .. src_line_start + (texture.size[0] as usize)];
		let mut src_u = surface_tc_min[0].rem_euclid(texture.size[0] as i32);
		let mut dst_u = 0;
		let start_pos_v = start_pos + (dst_v as f32) * v_vec;
		for dst_texel in dst_line.iter_mut()
		{
			let mut total_light = [0.0, 0.0, 0.0];
			if USE_LIGHTMAP
			{
				let lightmap_base_u = dst_u + lightmap_tc_shift[0];
				let lightmap_u = lightmap_base_u >> LIGHTAP_SCALE_LOG2;
				let lightmap_u_plus_one = lightmap_u + 1;
				debug_assert!(lightmap_u_plus_one < lightmap_size[0]);
				let l0 = unsafe { debug_only_checked_fetch(&line_lightmap, lightmap_u as usize) };
				let l1 = unsafe { debug_only_checked_fetch(&line_lightmap, lightmap_u_plus_one as usize) };
				let k = ((lightmap_base_u & lightmap_fetch_mask) as f32) * inv_lightmap_scale_f + k_shift;
				let k_minus_one = 1.0 - k;
				for i in 0 .. 3
				{
					total_light[i] = l0[i] * k_minus_one + l1[i] * k;
				}
			}

			let texel_value = unsafe { debug_only_checked_fetch(src_line, src_u as usize) };

			if USE_DYNAMIC_LIGHTS
			{
				let pos = start_pos_v + (dst_u as f32) * u_vec;

				let normal = if USE_NORMAL_MAP
				{
					// Normal transformed to world space.
					texel_value.normal.x * u_vec_normalized +
						texel_value.normal.y * v_vec_normalized +
						texel_value.normal.z * plane_normal_normalized
				}
				else
				{
					plane_normal_normalized
				};

				for (light, shadow_cube_map) in dynamic_lights
				{
					let vec_to_light = light.pos - pos;

					let shadow_factor = cube_shadow_map_fetch(shadow_cube_map, &vec_to_light);

					let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);
					let vec_to_light_normal_dot = normal.dot(vec_to_light);
					let angle_cos = vec_to_light_normal_dot * inv_sqrt_fast(vec_to_light_len2);

					let angle_cos_zero_clamped = angle_cos.max(0.0);
					let diffuse_intensity = 1.0;

					let mut specular_intensity = 0.0;
					let mut specular_k = 0.0;
					if USE_SPECULAR
					{
						let vec_to_light_reflected = normal * (2.0 * vec_to_light_normal_dot) - vec_to_light;
						let vec_to_camera = cam_pos - pos;
						let vec_to_camera_len2 = vec_to_camera.magnitude2().max(MIN_POSITIVE_VALUE);
						let vec_to_camera_light_reflected_angle_cos = vec_to_camera.dot(vec_to_light_reflected) *
							inv_sqrt_fast(vec_to_camera_len2 * vec_to_light_len2);

						// This formula is not physically-correct but it gives good results.
						let glossiness_shifted = 2.0 * texel_value.glossiness - 1.0;
						specular_intensity = ((1.0 + 1.0 / 64.0) - glossiness_shifted * glossiness_shifted) /
							((2.0 + 1.0 / 256.0) -
								texel_value.glossiness - vec_to_camera_light_reflected_angle_cos.min(1.0));

						// Schlick's approximation of Fresnel factor.
						// See https://en.wikipedia.org/wiki/Schlick%27s_approximation.
						let fresnel_factor = {
							let n1 = 1.0;
							let n2 = 1.3;
							let r_root = (n1 - n2) / (n1 + n2);
							let r0 = r_root * r_root;
							let one_minus_angle_cos = (1.0 - angle_cos_zero_clamped).max(0.0);
							let one_minus_angle_cos2 = one_minus_angle_cos * one_minus_angle_cos;
							(1.0 - r0) - (1.0 - r0) * one_minus_angle_cos2 * one_minus_angle_cos2 * one_minus_angle_cos
						};

						// This formula is correct for dielectrics.
						// TODO - fix this. For metals almost all light is reflected.
						specular_k = texel_value.glossiness * fresnel_factor;
					}

					let light_combined = shadow_factor *
						angle_cos_zero_clamped * (diffuse_intensity * (1.0 - specular_k) +
						specular_intensity * specular_k) /
						vec_to_light_len2;

					total_light[0] += light.color[0] * light_combined;
					total_light[1] += light.color[1] * light_combined;
					total_light[2] += light.color[2] * light_combined;
				}
			}

			let components = texel_value.diffuse.unpack_to_rgb_f32();
			let components_modulated = [
				(components[0] * total_light[0]).min(Color32::MAX_RGB_F32_COMPONENTS[0]),
				(components[1] * total_light[1]).min(Color32::MAX_RGB_F32_COMPONENTS[1]),
				(components[2] * total_light[2]).min(Color32::MAX_RGB_F32_COMPONENTS[2]),
			];

			// Here we 100% sure that components overflow is not possible (because of "min").
			// NaNs are not possible here too.
			let color_packed = unsafe { Color32::from_rgb_f32_unchecked(&components_modulated) };

			*dst_texel = color_packed;
			src_u += 1;
			if src_u == (texture.size[0] as i32)
			{
				src_u = 0;
			}

			dst_u += 1;
		}
	}
}

// Returns 1 if in light, 0 if in shadow.
fn cube_shadow_map_fetch(cube_shadow_map: &CubeShadowMap, vec: &Vec3f) -> f32
{
	let vec_abs = Vec3f::new(vec.x.abs(), vec.y.abs(), vec.z.abs());
	if vec_abs.x >= vec_abs.y && vec_abs.x >= vec_abs.z
	{
		if vec.x >= 0.0
		{
			cube_shadow_map_side_fetch(cube_shadow_map, &Vec3f::new(-vec.y, vec.z, vec_abs.x), 1)
		}
		else
		{
			cube_shadow_map_side_fetch(cube_shadow_map, &Vec3f::new(vec.y, vec.z, vec_abs.x), 0)
		}
	}
	else if vec_abs.y >= vec_abs.x && vec_abs.y >= vec_abs.z
	{
		if vec.y >= 0.0
		{
			cube_shadow_map_side_fetch(cube_shadow_map, &Vec3f::new(vec.x, vec.z, vec_abs.y), 3)
		}
		else
		{
			cube_shadow_map_side_fetch(cube_shadow_map, &Vec3f::new(-vec.x, vec.z, vec_abs.y), 2)
		}
	}
	else
	{
		if vec.z >= 0.0
		{
			cube_shadow_map_side_fetch(cube_shadow_map, &Vec3f::new(-vec.x, vec.y, vec_abs.z), 5)
		}
		else
		{
			cube_shadow_map_side_fetch(cube_shadow_map, &Vec3f::new(-vec.x, -vec.y, vec_abs.z), 4)
		}
	}
}

// Returns 1 if in light, 0 if in shadow.
fn cube_shadow_map_side_fetch(cube_shadow_map: &CubeShadowMap, vec: &Vec3f, side: u32) -> f32
{
	const ONE_MINUS_EPS: f32 = 1.0 - 1.0 / 65536.0;
	let cubemap_size_f = cube_shadow_map.size as f32;

	let depth = 1.0 / vec.z.max(MIN_POSITIVE_VALUE);
	let half_depth = 0.5 * depth;
	let u_f = (vec.x * half_depth + 0.5).max(0.0).min(ONE_MINUS_EPS) * cubemap_size_f;
	let v_f = (vec.y * half_depth + 0.5).max(0.0).min(ONE_MINUS_EPS) * cubemap_size_f;
	// It is safe to use "unsafe" f32 to int conversion, since NaN and Inf is not possible here.
	let u = unsafe { u_f.to_int_unchecked::<u32>() };
	let v = unsafe { v_f.to_int_unchecked::<u32>() };
	debug_assert!(u < cube_shadow_map.size);
	debug_assert!(v < cube_shadow_map.size);
	let texel_address = (u + v * cube_shadow_map.size) as usize;
	let value = unsafe { debug_only_checked_fetch(&cube_shadow_map.sides[side as usize], texel_address) };
	return if depth >= value { 1.0 } else { 0.0 };
}

const MIN_POSITIVE_VALUE: f32 = 1.0 / ((1 << 30) as f32);

// Relative erorr <= 1.5 * 2^(-12)
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn inv_sqrt_fast(x: f32) -> f32
{
	unsafe { core::arch::x86_64::_mm_cvtss_f32(core::arch::x86_64::_mm_rsqrt_ss(core::arch::x86_64::_mm_set1_ps(x))) }
}

#[cfg(all(target_arch = "x86", target_feature = "sse"))]
fn inv_sqrt_fast(x: f32) -> f32
{
	unsafe { core::arch::x86::_mm_cvtss_f32(core::arch::x86::_mm_rsqrt_ss(core::arch::x86::_mm_set1_ps(x))) }
}

#[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "sse")))]
fn inv_sqrt_fast(x: f32) -> f32
{
	1.0 / sqrt(x)
}

unsafe fn debug_only_checked_fetch<T: Copy>(data: &[T], address: usize) -> T
{
	// operator [] checks bounds and calls panic! handler in case if index is out of bounds.
	// This check is useless here since we clamp coordnates properly.
	// So, use "get_unchecked" in release mode.
	#[cfg(debug_assertions)]
	{
		data[address]
	}
	#[cfg(not(debug_assertions))]
	{
		*data.get_unchecked(address)
	}
}
