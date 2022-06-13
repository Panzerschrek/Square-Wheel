use super::math_types::*;

pub struct LightHemisphere
{
	pixels: [[f32; 3]; TEXTURE_AREA],
}

const TEXTURE_SIZE: u32 = 32;
const TEXTURE_AREA: usize = (TEXTURE_SIZE * TEXTURE_SIZE) as usize;
const TEXTURE_SIZE_F: f32 = TEXTURE_SIZE as f32;
const HALF_TEXTURE_SIZE_F: f32 = TEXTURE_SIZE_F * 0.5;

impl LightHemisphere
{
	pub fn new() -> Self
	{
		Self {
			pixels: [[0.0, 0.0, 0.0]; TEXTURE_AREA],
		}
	}

	pub fn add_point_light(&mut self, direction: &Vec3f, color: &[f32; 3])
	{
		let coord = project_vec_to_texture(direction);

		let pixel = &mut self.pixels[(coord[0] + coord[1] * TEXTURE_SIZE) as usize];
		for i in 0 .. 3
		{
			pixel[i] += color[i];
		}
	}

	pub fn add_sized_light(&mut self, direction: &Vec3f, color: &[f32; 3], _size: f32)
	{
		// TODO - perform some sort of blur for sized lights.
		self.add_point_light(direction, color);
	}
}

fn project_vec_to_texture(v: &Vec3f) -> [u32; 2]
{
	let v_normalized = v / v.magnitude();

	let coord_projected = project_normalized_vector(&v_normalized);
	let coord_in_texture =
		coord_projected * (HALF_TEXTURE_SIZE_F * 2.0_f32.sqrt()) - Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F);
	[
		clamp_to_texture_border(coord_in_texture.x),
		clamp_to_texture_border(coord_in_texture.y),
	]
}

fn clamp_to_texture_border(coord: f32) -> u32
{
	coord.max(0.0).min(TEXTURE_SIZE_F - 1.0) as u32
}

// Use Lambert azimuthal equal-area projection.
// It's important to use equal-area projection in order to avoid reestimating light in some places of projection.

// Project unit vector to plane. projection size is +-sqrt(2) for hemisphere, +-2 for sphere.
fn project_normalized_vector(v: &Vec3f) -> Vec2f
{
	v.truncate() * (2.0 / (v.z + 1.0).max(0.0)).sqrt()
}

// Unproject projection with size +-sqrt(2) for hemisphere and +-2 for sphere.
// Produces normalized vector.
fn unproject_normalized_coord(coord: &Vec2f) -> Vec3f
{
	let coord_square_len = coord.magnitude2();
	let xy_scale = ((1.0 - coord_square_len * 0.25).max(0.0)).sqrt();
	Vec3f::new(xy_scale * coord.x, xy_scale * coord.y, 1.0 - coord_square_len * 0.5)
}
