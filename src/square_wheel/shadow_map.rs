use crate::common::{math_types::*, matrix::*};

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

pub type ShadowMapElement = f32;

pub struct CubeShadowMap<'a>
{
	pub size: u32,
	pub sides: [&'a [ShadowMapElement]; 6],
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
			mat.x.x = 0.0;
			mat.x.y = 0.0;
			mat.x.z = 1.0;
			mat.y.x = -1.0;
			mat.y.y = 0.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::XMinus =>
		{
			mat.x.x = 0.0;
			mat.x.y = 0.0;
			mat.x.z = -1.0;
			mat.y.x = 1.0;
			mat.y.y = 0.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::YPlus =>
		{
			mat.x.x = 1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = 0.0;
			mat.y.z = 1.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::YMinus =>
		{
			mat.x.x = -1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = 0.0;
			mat.y.z = -1.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::ZPlus =>
		{
			mat.x.x = 1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = 1.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = 0.0;
			mat.z.z = 1.0;
		},
		CubeMapSide::ZMinus =>
		{
			mat.x.x = 1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = -1.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = 0.0;
			mat.z.z = -1.0;
		},
	}
	mat
}
