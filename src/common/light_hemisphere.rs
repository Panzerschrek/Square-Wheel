use super::{color::*, image, math_types::*};

pub struct LightHemisphere
{
	pixels: [[f32; 3]; TEXTURE_AREA],
}

const TEXTURE_SIZE: u32 = 64;
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

	pub fn add_sized_light(&mut self, direction: &Vec3f, color: &[f32; 3], size: f32)
	{
		// Add sized light using Gaussian-blur.
		// Use spherial gaussian function for this.
		// Deviation is based on provided size.

		let direction_normalized = direction / direction.magnitude();

		let coord_projected = project_normalized_vector(&direction_normalized);
		let coord_in_texture = coord_projected * (HALF_TEXTURE_SIZE_F / 2.0_f32.sqrt()) +
			Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F);
		let coord = [
			clamp_to_texture_border(coord_in_texture.x),
			clamp_to_texture_border(coord_in_texture.y),
		];

		let deviation = size / 20.0;

		let box_half_size_f = deviation * 12.0 * TEXTURE_SIZE_F;
		if box_half_size_f < 0.25
		{
			// Sharp gaussian. Avoid useless integration, just assign light power to center pixel.
			let dst = &mut self.pixels[(coord[0] + coord[1] * TEXTURE_SIZE) as usize];
			for i in 0 .. 3
			{
				dst[i] += color[i];
			}
			return;
		}

		let box_half_size = (box_half_size_f + 1.0) as i32;

		let x_start = (coord[0] as i32 - box_half_size).max(0);
		let x_end = (coord[0] as i32 + box_half_size).min(TEXTURE_SIZE as i32 - 1);

		let y_start = (coord[1] as i32 - box_half_size).max(0);
		let y_end = (coord[1] as i32 + box_half_size).min(TEXTURE_SIZE as i32 - 1);

		let gaussian_scale = 1.0 / (deviation * deviation * (TEXTURE_SIZE_F * TEXTURE_SIZE_F));

		let power_func = |pos| {
			let projection_point =
				(pos - Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F)) * (2.0_f32.sqrt() / HALF_TEXTURE_SIZE_F);
			let vec = unproject_normalized_coord(&projection_point);
			let angle_cos = vec.dot(direction_normalized);
			gaussian_scale * ((angle_cos - 1.0) / deviation).exp()
		};

		if box_half_size >= 6
		{
			// Large scale - no supersampling.
			for y in y_start ..= y_end
			{
				for x in x_start ..= x_end
				{
					let power = power_func(Vec2f::new(x as f32 + 0.5, y as f32 + 0.5));
					let dst = &mut self.pixels[(x + y * (TEXTURE_SIZE as i32)) as usize];
					for i in 0 .. 3
					{
						dst[i] += power * color[i];
					}
				}
			}
		}
		else if box_half_size >= 3
		{
			// Middle scale - perform 2x2 supersampling.
			for y in y_start ..= y_end
			{
				for x in x_start ..= x_end
				{
					let base_pos = Vec2f::new(x as f32, y as f32);
					let mut power = power_func(base_pos + Vec2f::new(0.25, 0.25)) +
						power_func(base_pos + Vec2f::new(0.25, 0.75)) +
						power_func(base_pos + Vec2f::new(0.75, 0.25)) +
						power_func(base_pos + Vec2f::new(0.75, 0.75));
					power *= 0.25;
					let dst = &mut self.pixels[(x + y * (TEXTURE_SIZE as i32)) as usize];
					for i in 0 .. 3
					{
						dst[i] += power * color[i];
					}
				}
			}
		}
		else
		{
			// Small scale - perform 4x4 supersampling.
			for y in y_start ..= y_end
			{
				for x in x_start ..= x_end
				{
					let base_pos = Vec2f::new(x as f32, y as f32);
					let mut power = 0.0;
					for dx in 0 .. 4
					{
						for dy in 0 .. 4
						{
							power += power_func(
								base_pos + Vec2f::new(0.125 + (dx as f32) * 0.25, 0.125 + (dy as f32) * 0.25),
							);
						}
					}
					power *= 1.0 / 16.0;
					let dst = &mut self.pixels[(x + y * (TEXTURE_SIZE as i32)) as usize];
					for i in 0 .. 3
					{
						dst[i] += power * color[i];
					}
				}
			}
		}
	}

	pub fn debug_save(&self, file_path: &std::path::Path)
	{
		let mut img = image::Image {
			size: [TEXTURE_SIZE, TEXTURE_SIZE],
			pixels: vec![Color32::black(); TEXTURE_AREA],
		};
		for (dst, src) in img.pixels.iter_mut().zip(self.pixels.iter())
		{
			let scale = 255.0 * 64.0;
			let r = (src[0] * scale).max(0.0).min(255.0) as u8;
			let g = (src[1] * scale).max(0.0).min(255.0) as u8;
			let b = (src[2] * scale).max(0.0).min(255.0) as u8;
			*dst = Color32::from_rgba(r, g, b, 255);
		}
		image::save(&img, file_path);
	}
}

fn project_vec_to_texture(v: &Vec3f) -> [u32; 2]
{
	let v_normalized = v / v.magnitude();

	let coord_projected = project_normalized_vector(&v_normalized);
	let coord_in_texture =
		coord_projected * (HALF_TEXTURE_SIZE_F / 2.0_f32.sqrt()) + Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F);
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
