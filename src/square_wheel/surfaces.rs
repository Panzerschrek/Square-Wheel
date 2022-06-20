use super::{light::*, shadow_map::*, textures};
use common::{bsp_map_compact, color::*, lightmap, math_types::*, plane::*};

pub type LightWithShadowMap<'a, 'b> = (&'a PointLight, &'b CubeShadowMap);

pub fn build_surface_simple_lightmap(
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
	build_surface_impl_1_static_params::<LightmapElementOpsSimple>(
		plane,
		tex_coord_equation,
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

pub fn build_surface_directional_lightmap(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_scale_log2: u32,
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[bsp_map_compact::DirectionalLightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	build_surface_impl_1_static_params::<LightmapElementOpsDirectional>(
		plane,
		tex_coord_equation,
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

fn build_surface_impl_1_static_params<LightmapElementOpsT: LightmapElementOps>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_scale_log2: u32,
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	// Perform call in each branch instead of assigning function to function pointer and calling it later because LLVM compiler can't inline call via pointer.
	// Proper inlining is very important here - it can reduce call overhead and merge identical code.
	if lightmap_data.is_empty()
	{
		build_surface_impl_2_static_params::<LightmapElementOpsT, NO_LIGHTMAP_SCALE>(
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
	else if lightmap_scale_log2 == 0
	{
		build_surface_impl_2_static_params::<LightmapElementOpsT, 0>(
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
		build_surface_impl_2_static_params::<LightmapElementOpsT, 1>(
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
		build_surface_impl_2_static_params::<LightmapElementOpsT, 2>(
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
		build_surface_impl_2_static_params::<LightmapElementOpsT, 3>(
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
		build_surface_impl_2_static_params::<LightmapElementOpsT, 4>(
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

fn build_surface_impl_2_static_params<LightmapElementOpsT: LightmapElementOps, const LIGHTAP_SCALE_LOG2: u32>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if dynamic_lights.is_empty()
	{
		build_surface_impl_3_static_params::<LightmapElementOpsT, LIGHTAP_SCALE_LOG2, false>(
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
		build_surface_impl_3_static_params::<LightmapElementOpsT, LIGHTAP_SCALE_LOG2, true>(
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
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_DYNAMIC_LIGHTS: bool,
>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if texture.has_normal_map
	{
		build_surface_impl_4_static_params::<LightmapElementOpsT, LIGHTAP_SCALE_LOG2, USE_DYNAMIC_LIGHTS, true>(
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
		build_surface_impl_4_static_params::<LightmapElementOpsT, LIGHTAP_SCALE_LOG2, USE_DYNAMIC_LIGHTS, false>(
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
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
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
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
	dynamic_lights: &[LightWithShadowMap],
	cam_pos: &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	if texture.has_non_zero_glossiness
	{
		if texture.is_metal
		{
			build_surface_impl_5_static_params::<
				LightmapElementOpsT,
				LIGHTAP_SCALE_LOG2,
				USE_DYNAMIC_LIGHTS,
				USE_NORMAL_MAP,
				SPECULAR_TYPE_METAL,
			>(
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
			build_surface_impl_5_static_params::<
				LightmapElementOpsT,
				LIGHTAP_SCALE_LOG2,
				USE_DYNAMIC_LIGHTS,
				USE_NORMAL_MAP,
				SPECULAR_TYPE_DIELECTRIC,
			>(
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
	else
	{
		build_surface_impl_5_static_params::<
			LightmapElementOpsT,
			LIGHTAP_SCALE_LOG2,
			USE_DYNAMIC_LIGHTS,
			USE_NORMAL_MAP,
			SPECULAR_TYPE_NONE,
		>(
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

pub const SPECULAR_TYPE_NONE: u32 = 0;
pub const SPECULAR_TYPE_DIELECTRIC: u32 = 1;
pub const SPECULAR_TYPE_METAL: u32 = 2;

pub const NO_LIGHTMAP_SCALE: u32 = 31;

// Specify various settings as template params in order to get most efficient code for current combination of params.
// Use chained dispatch in order to convert dynamic params into static.
fn build_surface_impl_5_static_params<
	LightmapElementOpsT: LightmapElementOps,
	const LIGHTAP_SCALE_LOG2: u32,
	const USE_DYNAMIC_LIGHTS: bool,
	const USE_NORMAL_MAP: bool,
	const SPECULAR_TYPE: u32,
>(
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &textures::Texture,
	lightmap_size: [u32; 2],
	lightmap_tc_shift: [u32; 2],
	lightmap_data: &[LightmapElementOpsT::LightmapElement],
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
			let k = ((lightmap_base_v & lightmap_fetch_mask) as f32) * inv_lightmap_scale_f + k_shift;
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
				*dst = LightmapElementOpsT::mix(&l1, &l0, k);
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
			let texel_value = unsafe { debug_only_checked_fetch(src_line, src_u as usize) };

			let mut total_light_albedo_modulated = [0.0, 0.0, 0.0];
			let mut total_light_direct = [0.0, 0.0, 0.0];
			if LIGHTAP_SCALE_LOG2 != NO_LIGHTMAP_SCALE
			{
				let lightmap_base_u = dst_u + lightmap_tc_shift[0];
				let lightmap_u = lightmap_base_u >> LIGHTAP_SCALE_LOG2;
				let lightmap_u_plus_one = lightmap_u + 1;
				debug_assert!(lightmap_u_plus_one < lightmap_size[0]);
				let l0 = unsafe { debug_only_checked_fetch(&line_lightmap, lightmap_u as usize) };
				let l1 = unsafe { debug_only_checked_fetch(&line_lightmap, lightmap_u_plus_one as usize) };
				let k = ((lightmap_base_u & lightmap_fetch_mask) as f32) * inv_lightmap_scale_f + k_shift;
				let l_mixed = LightmapElementOpsT::mix(&l1, &l0, k);

				total_light_albedo_modulated = LightmapElementOpsT::get_constant_component(&l_mixed);
				if let Some(directional_component) = LightmapElementOpsT::get_directional_component(&l_mixed)
				{
					let dot = if USE_NORMAL_MAP
					{
						directional_component.vector_scaled.dot(texel_value.normal).max(0.0)
					}
					else
					{
						directional_component.vector_scaled.z
					};
					// TODO - use deviation.
					for i in 0 .. 3
					{
						total_light_albedo_modulated[i] += directional_component.color[i] * dot;
					}
				}
			}

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

				let vec_to_camera_reflected;
				let vec_to_camera_len2;
				let specular_k;
				if SPECULAR_TYPE != SPECULAR_TYPE_NONE
				{
					// Calculate reflected view angle and fresnel factor based on it.
					// Use these data later for calculation o specular light for all dynamic lights.
					let vec_to_camera = cam_pos - pos;
					let vec_to_camera_normal_dot = vec_to_camera.dot(normal);
					vec_to_camera_reflected = normal * (2.0 * vec_to_camera_normal_dot) - vec_to_camera;
					vec_to_camera_len2 = vec_to_camera_reflected.magnitude2().max(MIN_POSITIVE_VALUE);

					let vec_to_camera_normal_angle_cos =
						(vec_to_camera_normal_dot * inv_sqrt_fast(vec_to_camera_len2)).max(0.0);

					// Schlick's approximation of Fresnel factor.
					// See https://en.wikipedia.org/wiki/Schlick%27s_approximation.

					// For glossy surface we can just use Fresnel factor for diffuse/specular mixing.
					// But for rough srufaces we can't. Normally we should use some sort of integral of Schlick's approximation.
					// But it's too expensive. So, just make mix of Fresnel factor depending on view angle with constant factor for absolutely rough surface.
					// TODO - us non-linear glossiness here?

					let one_minus_angle_cos = (1.0 - vec_to_camera_normal_angle_cos).max(0.0);
					let one_minus_angle_cos2 = one_minus_angle_cos * one_minus_angle_cos;
					let fresnel_factor_base = one_minus_angle_cos2 * one_minus_angle_cos2 * one_minus_angle_cos;
					if SPECULAR_TYPE == SPECULAR_TYPE_DIELECTRIC
					{
						let fresnel_factor =
							DIELECTRIC_ZERO_REFLECTIVITY + (1.0 - DIELECTRIC_ZERO_REFLECTIVITY) * fresnel_factor_base;

						specular_k = fresnel_factor * texel_value.glossiness +
							DIELECTRIC_AVERAGE_REFLECTIVITY * (1.0 - texel_value.glossiness);
					}
					else if SPECULAR_TYPE == SPECULAR_TYPE_METAL
					{
						specular_k = fresnel_factor_base * texel_value.glossiness +
							METAL_AVERAGE_SCHLICK_FACTOR * (1.0 - texel_value.glossiness);
					}
					else
					{
						specular_k = 0.0;
					}
				}
				else
				{
					vec_to_camera_reflected = Vec3f::zero();
					vec_to_camera_len2 = MIN_POSITIVE_VALUE;
					specular_k = 0.0;
				}

				for (light, shadow_cube_map) in dynamic_lights
				{
					let vec_to_light = light.pos - pos;

					let shadow_factor = cube_shadow_map_fetch(shadow_cube_map, &vec_to_light);
					let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);
					let shadow_distance_factor = shadow_factor / vec_to_light_len2;

					let diffuse_intensity = if SPECULAR_TYPE == SPECULAR_TYPE_METAL
					{
						// No diffuse light for metalls.
						0.0
					}
					else
					{
						(normal.dot(vec_to_light) * inv_sqrt_fast(vec_to_light_len2)).max(0.0)
					};

					let specular_intensity = if SPECULAR_TYPE == SPECULAR_TYPE_NONE
					{
						// No specular for surfaces without specular.
						0.0
					}
					else
					{
						let vec_to_camera_reflected_light_angle_cos = vec_to_camera_reflected.dot(vec_to_light) *
							inv_sqrt_fast(vec_to_camera_len2 * vec_to_light_len2);

						// This formula is not physically-correct but it gives good results.
						let glossiness_scaled = 64.0 * texel_value.glossiness;
						let x = ((vec_to_camera_reflected_light_angle_cos - 1.0) * glossiness_scaled).max(-2.0);
						(x * (x * 0.25 + 1.0) + 1.0) * glossiness_scaled
					};

					match SPECULAR_TYPE
					{
						SPECULAR_TYPE_NONE =>
						{
							total_light_albedo_modulated[0] += light.color[0] * shadow_distance_factor;
							total_light_albedo_modulated[1] += light.color[1] * shadow_distance_factor;
							total_light_albedo_modulated[2] += light.color[2] * shadow_distance_factor;
						},
						SPECULAR_TYPE_DIELECTRIC =>
						{
							let light_intensity_diffuse =
								diffuse_intensity * (1.0 - specular_k) * shadow_distance_factor;
							total_light_albedo_modulated[0] += light.color[0] * light_intensity_diffuse;
							total_light_albedo_modulated[1] += light.color[1] * light_intensity_diffuse;
							total_light_albedo_modulated[2] += light.color[2] * light_intensity_diffuse;

							let light_intensity_specular = specular_intensity * specular_k * shadow_distance_factor;
							total_light_direct[0] += light.color[0] * light_intensity_specular;
							total_light_direct[1] += light.color[1] * light_intensity_specular;
							total_light_direct[2] += light.color[2] * light_intensity_specular;
						},
						SPECULAR_TYPE_METAL =>
						{
							let specular_intensity_shadow_distance_factor = specular_intensity * shadow_distance_factor;

							let light_intensity_modulated =
								(1.0 - specular_k) * specular_intensity_shadow_distance_factor;
							total_light_albedo_modulated[0] += light.color[0] * light_intensity_modulated;
							total_light_albedo_modulated[1] += light.color[1] * light_intensity_modulated;
							total_light_albedo_modulated[2] += light.color[2] * light_intensity_modulated;

							let light_intensity_direct = specular_k * specular_intensity_shadow_distance_factor;
							total_light_direct[0] += light.color[0] * light_intensity_direct;
							total_light_direct[1] += light.color[1] * light_intensity_direct;
							total_light_direct[2] += light.color[2] * light_intensity_direct;
						},
						_ =>
						{
							panic!("Wrong specular type!")
						},
					}
				} // For dynamic lights.
			} // If use dynmic lights.

			let color_components = texel_value.diffuse.unpack_to_rgb_f32();

			let mut result_color_components = [0.0, 0.0, 0.0];
			for i in 0 .. 3
			{
				let mut c = color_components[i] * total_light_albedo_modulated[i];
				if SPECULAR_TYPE != SPECULAR_TYPE_NONE
				{
					c += total_light_direct[i] * Color32::MAX_RGB_F32_COMPONENTS[i];
				}
				result_color_components[i] = c.min(Color32::MAX_RGB_F32_COMPONENTS[i]);
			}

			// Here we 100% sure that components overflow is not possible (because of "min").
			// NaNs are not possible here too.
			let color_packed = unsafe { Color32::from_rgb_f32_unchecked(&result_color_components) };

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
			a[0] * ratio + b[0] * one_minus_ratio,
			a[1] * ratio + b[1] * one_minus_ratio,
			a[2] * ratio + b[2] * one_minus_ratio,
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
				a.ambient_light[0] * ratio + b.ambient_light[0] * one_minus_ratio,
				a.ambient_light[1] * ratio + b.ambient_light[1] * one_minus_ratio,
				a.ambient_light[2] * ratio + b.ambient_light[2] * one_minus_ratio,
			],
			light_direction_vector_scaled: a.light_direction_vector_scaled * ratio +
				b.light_direction_vector_scaled * one_minus_ratio,
			directional_light_deviation: a.directional_light_deviation * ratio +
				b.directional_light_deviation * one_minus_ratio,
			directional_light_color: [
				a.directional_light_color[0] * ratio + b.directional_light_color[0] * one_minus_ratio,
				a.directional_light_color[1] * ratio + b.directional_light_color[1] * one_minus_ratio,
				a.directional_light_color[2] * ratio + b.directional_light_color[2] * one_minus_ratio,
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

const DIELECTRIC_ZERO_REFLECTIVITY: f32 = 0.04;
const DIELECTRIC_AVERAGE_REFLECTIVITY: f32 = DIELECTRIC_ZERO_REFLECTIVITY * 3.0;
const METAL_AVERAGE_SCHLICK_FACTOR: f32 = 0.5;

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
