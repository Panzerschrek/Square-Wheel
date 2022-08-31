use super::fast_math::*;
use crate::common::{math_types::*, matrix::*};

pub struct DynamicLightWithShadow<'a>
{
	pub position: Vec3f,
	pub radius: f32,
	pub inv_square_radius: f32,
	pub color: [f32; 3],
	pub shadow_map: ShadowMap<'a>,
}

#[repr(u32)]
#[derive(PartialEq, PartialOrd)]
pub enum CubeMapSide
{
	XPlus,
	XMinus,
	YPlus,
	YMinus,
	ZPlus,
	ZMinus,
}

pub fn int_to_cubemap_side(i: u32) -> Option<CubeMapSide>
{
	match i
	{
		0 => Some(CubeMapSide::XPlus),
		1 => Some(CubeMapSide::XMinus),
		2 => Some(CubeMapSide::YPlus),
		3 => Some(CubeMapSide::YMinus),
		4 => Some(CubeMapSide::ZPlus),
		5 => Some(CubeMapSide::ZMinus),
		_ => None,
	}
}

pub enum ShadowMap<'a>
{
	None,
	Cube(CubeShadowMap<'a>),
	Projector(ProjectorShadowMap<'a>),
}

pub type ShadowMapElement = f32;

pub struct CubeShadowMap<'a>
{
	pub size: u32,
	pub sides: [&'a [ShadowMapElement]; 6],
}

pub struct ProjectorShadowMap<'a>
{
	pub size: u32,
	pub data: &'a [ShadowMapElement],
	pub basis_x: Vec3f,
	pub basis_y: Vec3f,
	pub basis_z: Vec3f,
}

pub fn calculate_projector_shadow_map_matrices(
	position: Vec3f,
	rotation: QuaternionF,
	shadow_map_size: f32,
) -> CameraMatrices
{
	build_view_matrix_with_full_rotation(
		position,
		rotation,
		std::f32::consts::PI * 0.5,
		shadow_map_size,
		shadow_map_size,
	)
}

pub fn calculate_cube_shadow_map_side_matrices(
	position: Vec3f,
	shadow_map_size: f32,
	side: CubeMapSide,
) -> CameraMatrices
{
	complete_view_matrix(
		position,
		&get_cube_map_side_matrix(side),
		std::f32::consts::PI * 0.5,
		shadow_map_size,
		shadow_map_size,
	)
}

fn get_cube_map_side_matrix(side: CubeMapSide) -> Mat4f
{
	let mut mat = Mat4f::identity();
	match side
	{
		CubeMapSide::XPlus =>
		{
			mat.x = Vec4f::unit_z();
			mat.y = -Vec4f::unit_x();
			mat.z = -Vec4f::unit_y();
		},
		CubeMapSide::XMinus =>
		{
			mat.x = -Vec4f::unit_z();
			mat.y = Vec4f::unit_x();
			mat.z = -Vec4f::unit_y();
		},
		CubeMapSide::YPlus =>
		{
			mat.x = Vec4f::unit_x();
			mat.y = Vec4f::unit_z();
			mat.z = -Vec4f::unit_y();
		},
		CubeMapSide::YMinus =>
		{
			mat.x = -Vec4f::unit_x();
			mat.y = -Vec4f::unit_z();
			mat.z = -Vec4f::unit_y();
		},
		CubeMapSide::ZPlus =>
		{
			mat.x = Vec4f::unit_x();
			mat.y = Vec4f::unit_y();
			mat.z = Vec4f::unit_z();
		},
		CubeMapSide::ZMinus =>
		{
			mat.x = Vec4f::unit_x();
			mat.y = -Vec4f::unit_y();
			mat.z = -Vec4f::unit_z();
		},
	}
	mat
}

pub fn get_light_shadow_factor(light: &DynamicLightWithShadow, vec: &Vec3f) -> f32
{
	match &light.shadow_map
	{
		ShadowMap::None => 1.0,
		ShadowMap::Cube(cube_shadow_map) => cube_shadow_map_fetch(cube_shadow_map, vec),
		ShadowMap::Projector(projector_shadow_map) => projector_shadow_map_fetch(projector_shadow_map, vec),
	}
}

// Returns 1 if in light, 0 if in shadow.
pub fn cube_shadow_map_fetch(cube_shadow_map: &CubeShadowMap, vec: &Vec3f) -> f32
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

	let depth = inv_fast(vec.z.max(MIN_POSITIVE_VALUE));
	let half_depth = 0.5 * depth;
	let u_f = f32_mul_add(vec.x, half_depth, 0.5).max(0.0).min(ONE_MINUS_EPS) * cubemap_size_f;
	let v_f = f32_mul_add(vec.y, half_depth, 0.5).max(0.0).min(ONE_MINUS_EPS) * cubemap_size_f;
	// It is safe to use "unsafe" f32 to int conversion, since NaN and Inf is not possible here.
	let u = unsafe { u_f.to_int_unchecked::<u32>() };
	let v = unsafe { v_f.to_int_unchecked::<u32>() };
	debug_assert!(u < cube_shadow_map.size);
	debug_assert!(v < cube_shadow_map.size);
	let texel_address = (u + v * cube_shadow_map.size) as usize;
	let value = unsafe { debug_only_checked_fetch(&cube_shadow_map.sides[side as usize], texel_address) };
	if depth >= value
	{
		1.0
	}
	else
	{
		0.0
	}
}

fn projector_shadow_map_fetch(projector_shadow_map: &ProjectorShadowMap, vec: &Vec3f) -> f32
{
	let z = projector_shadow_map.basis_z.dot(*vec);
	if z <= 0.0
	{
		return 0.0;
	}

	let depth = inv_fast(z);
	let half_depth = 0.5 * depth;
	let size_f = projector_shadow_map.size as f32;
	let u_f = f32_mul_add(projector_shadow_map.basis_x.dot(*vec), half_depth, 0.5) * size_f;
	let v_f = f32_mul_add(projector_shadow_map.basis_y.dot(*vec), half_depth, 0.5) * size_f;
	if u_f < 0.0 || v_f < 0.0 || u_f >= size_f || v_f >= size_f
	{
		return 0.0;
	}

	// It is safe to use "unsafe" f32 to int conversion, since NaN and Inf is not possible here.
	let u = unsafe { u_f.to_int_unchecked::<u32>() };
	let v = unsafe { v_f.to_int_unchecked::<u32>() };
	debug_assert!(u < projector_shadow_map.size);
	debug_assert!(v < projector_shadow_map.size);
	let texel_address = (u + v * projector_shadow_map.size) as usize;
	let value = unsafe { debug_only_checked_fetch(projector_shadow_map.data, texel_address) };
	if depth >= value
	{
		1.0
	}
	else
	{
		0.0
	}
}

pub fn get_light_cube_light(light_cube: &[[f32; 3]; 6], normal_normalized: &Vec3f) -> [f32; 3]
{
	let mut total_light = [0.0, 0.0, 0.0];
	if normal_normalized.x <= 0.0
	{
		for i in 0 .. 3
		{
			total_light[i] += light_cube[0][i] * (-normal_normalized.x);
		}
	}
	else
	{
		for i in 0 .. 3
		{
			total_light[i] += light_cube[1][i] * normal_normalized.x;
		}
	}
	if normal_normalized.y <= 0.0
	{
		for i in 0 .. 3
		{
			total_light[i] += light_cube[2][i] * (-normal_normalized.y);
		}
	}
	else
	{
		for i in 0 .. 3
		{
			total_light[i] += light_cube[3][i] * normal_normalized.y;
		}
	}
	if normal_normalized.z <= 0.0
	{
		for i in 0 .. 3
		{
			total_light[i] += light_cube[4][i] * (-normal_normalized.z);
		}
	}
	else
	{
		for i in 0 .. 3
		{
			total_light[i] += light_cube[5][i] * normal_normalized.z;
		}
	}

	total_light
}
