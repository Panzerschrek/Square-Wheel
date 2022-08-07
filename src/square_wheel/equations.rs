use crate::common::math_types::*;

// Screen-space depth equation.
#[derive(Copy, Clone, Default)]
pub struct DepthEquation
{
	pub d_inv_z_dx: f32,
	pub d_inv_z_dy: f32,
	pub k: f32,
}

// Screen-space textures coordinates equation (divided by depth).
#[derive(Copy, Clone, Default)]
pub struct TexCoordEquation
{
	pub d_tc_dx: [f32; 2],
	pub d_tc_dy: [f32; 2],
	pub k: [f32; 2],
}

impl DepthEquation
{
	pub fn from_transformed_plane_equation(plane_transformed: &Vec4f) -> Self
	{
		let plane_transformed_w = -plane_transformed.w;
		Self {
			d_inv_z_dx: plane_transformed.x / plane_transformed_w,
			d_inv_z_dy: plane_transformed.y / plane_transformed_w,
			k: plane_transformed.z / plane_transformed_w,
		}
	}

	pub fn sample_point(&self, point: &Vec2f) -> f32
	{
		self.d_inv_z_dx * point.x + self.d_inv_z_dy * point.y + self.k
	}
}

impl std::ops::Mul<f32> for DepthEquation
{
	type Output = Self;

	fn mul(self, scalar: f32) -> Self
	{
		Self {
			d_inv_z_dx: self.d_inv_z_dx * scalar,
			d_inv_z_dy: self.d_inv_z_dy * scalar,
			k: self.k * scalar,
		}
	}
}

impl TexCoordEquation
{
	pub fn from_depth_equation_and_transformed_tex_coord_equations(
		depth_equation: &DepthEquation,
		tex_coord_equations: &[Vec4f; 2],
	) -> Self
	{
		Self {
			d_tc_dx: [
				tex_coord_equations[0].x + tex_coord_equations[0].w * depth_equation.d_inv_z_dx,
				tex_coord_equations[1].x + tex_coord_equations[1].w * depth_equation.d_inv_z_dx,
			],
			d_tc_dy: [
				tex_coord_equations[0].y + tex_coord_equations[0].w * depth_equation.d_inv_z_dy,
				tex_coord_equations[1].y + tex_coord_equations[1].w * depth_equation.d_inv_z_dy,
			],
			k: [
				tex_coord_equations[0].z + tex_coord_equations[0].w * depth_equation.k,
				tex_coord_equations[1].z + tex_coord_equations[1].w * depth_equation.k,
			],
		}
	}
}

impl std::ops::Mul<f32> for TexCoordEquation
{
	type Output = Self;

	fn mul(self, scalar: f32) -> Self
	{
		Self {
			d_tc_dx: [self.d_tc_dx[0] * scalar, self.d_tc_dx[1] * scalar],
			d_tc_dy: [self.d_tc_dy[0] * scalar, self.d_tc_dy[1] * scalar],
			k: [self.k[0] * scalar, self.k[1] * scalar],
		}
	}
}
