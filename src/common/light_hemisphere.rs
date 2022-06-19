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

		let pixel = &mut self.pixels[get_pixel_address(coord[0], coord[1])];
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

		// TODO - check correctness of integration.

		let direction_normalized = direction / direction.magnitude();

		let coord_projected = project_normalized_vector(&direction_normalized);
		let coord_in_texture = coord_projected * (HALF_TEXTURE_SIZE_F / 2.0_f32.sqrt()) +
			Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F);

		let deviation = size * 0.5;

		let box_half_size = deviation * (3.0 * HALF_TEXTURE_SIZE_F);
		if box_half_size < 0.25
		{
			let coord = [
				clamp_to_texture_border(coord_in_texture.x),
				clamp_to_texture_border(coord_in_texture.y),
			];
			// Sharp gaussian. Avoid useless integration, just assign light power to center pixel.
			let dst = &mut self.pixels[get_pixel_address(coord[0], coord[1])];
			for i in 0 .. 3
			{
				dst[i] += color[i];
			}
			return;
		}

		let deviation2 = deviation * deviation;

		let x_start = clamp_to_texture_border(coord_in_texture[0] - box_half_size);
		let x_end = clamp_to_texture_border(coord_in_texture[0] + box_half_size);

		let y_start = clamp_to_texture_border(coord_in_texture[1] - box_half_size);
		let y_end = clamp_to_texture_border(coord_in_texture[1] + box_half_size);

		let gaussian_scale = 4.0 / (deviation2 * (TEXTURE_SIZE_F * TEXTURE_SIZE_F * std::f32::consts::PI));
		let inv_deviation2 = 1.0 / deviation2;

		let power_func = |pos| {
			let projection_point =
				(pos - Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F)) * (2.0_f32.sqrt() / HALF_TEXTURE_SIZE_F);
			let vec = unproject_normalized_coord(&projection_point);
			let angle_cos = vec.dot(direction_normalized);
			gaussian_scale * ((angle_cos - 1.0) * inv_deviation2).exp()
		};

		if box_half_size >= 5.0
		{
			// Large scale - no supersampling.
			for y in y_start ..= y_end
			{
				for x in x_start ..= x_end
				{
					let power = power_func(Vec2f::new(x as f32 + 0.5, y as f32 + 0.5));
					let dst = &mut self.pixels[get_pixel_address(x, y)];
					for i in 0 .. 3
					{
						dst[i] += power * color[i];
					}
				}
			}
		}
		else if box_half_size >= 2.5
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
					let dst = &mut self.pixels[get_pixel_address(x, y)];
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
					let dst = &mut self.pixels[get_pixel_address(x, y)];
					for i in 0 .. 3
					{
						dst[i] += power * color[i];
					}
				}
			}
		}
	}

	pub fn extract_ambient_light(&mut self) -> [f32; 3]
	{
		// Collect pixels inside projection circle.
		let mut arr = [[0.0, 0.0, 0.0]; TEXTURE_AREA]; // TODO - use uninitialized memory.
		let mut arr_elements = 0;
		for y in 0 .. TEXTURE_SIZE
		{
			let two_dy = (2 * y + 1) as i32 - (TEXTURE_SIZE as i32);
			let two_dy2 = two_dy * two_dy;
			for x in 0 .. TEXTURE_SIZE
			{
				let two_dx = (2 * x + 1) as i32 - (TEXTURE_SIZE as i32);
				let two_dx2 = two_dx * two_dx;

				let two_len2 = two_dx2 + two_dy2;
				// TODO - include also texels touching projection circle.
				if two_len2 <= (TEXTURE_SIZE * TEXTURE_SIZE) as i32
				{
					arr[arr_elements] = self.pixels[get_pixel_address(x, y)];
					arr_elements += 1;
				}
			}
		}

		// Find median light value.
		let mut median_value = [0.0, 0.0, 0.0];
		let projection_pixels = &mut arr[.. arr_elements];
		for i in 0 .. 3
		{
			// Sort pixels using current component.
			// Crap! Rust-faggots, do not allow me to sort floats normally!
			// projection_pixels.sort_unstable_by_key(|x| x[i]);
			projection_pixels.sort_by(|a, b| a[i].partial_cmp(&b[i]).unwrap());

			median_value[i] = projection_pixels[arr_elements / 2][i];
		}

		// Subtract median light value. Accumulate sutracted value, using cosine law.
		let mut ambient_light = [0.0, 0.0, 0.0];
		for y in 0 .. TEXTURE_SIZE
		{
			for x in 0 .. TEXTURE_SIZE
			{
				let light = &mut self.pixels[get_pixel_address(x, y)];
				let normal_cos = unproject_texture_coord(x, y).z.max(0.0);

				for i in 0 .. 3
				{
					ambient_light[i] += median_value[i].min(light[i]) * normal_cos;
					light[i] = (light[i] - median_value[i]).max(0.0)
				}
			}
		}

		ambient_light
	}

	pub fn calculate_light_direction(&self) -> DirectionalLightParams
	{
		// Caclulate brightness for each pixel,
		// scale direction vector (normalized) by this brightness,
		// caclulate sum of scaled vectors.
		// Sum vector is a direction of light scaled by brightness.
		// Ratio between this vector length and sum of brightness values is half cosine of deviation vecror.

		// Also calculate light sum and calculate color based on this sum.

		let mut light_sum = [0.0, 0.0, 0.0];
		let mut scaled_vecs_sum = Vec3f::new(0., 0.0, 0.0);
		let mut brightness_sum = 0.0;
		for y in 0 .. TEXTURE_SIZE
		{
			for x in 0 .. TEXTURE_SIZE
			{
				let light = &self.pixels[get_pixel_address(x, y)];
				for i in 0 .. 3
				{
					light_sum[i] += light[i];
				}
				let brightness = get_light_brightness(light);
				let vec = unproject_texture_coord(x, y);
				scaled_vecs_sum += vec * brightness;
				brightness_sum += brightness;
			}
		}

		const MIN_LEN: f32 = 1.0 / (1024.0 * 1024.0);

		brightness_sum = brightness_sum.max(MIN_LEN);

		let mut vec_len = scaled_vecs_sum.magnitude();
		if vec_len <= 0.0
		{
			scaled_vecs_sum = Vec3f::new(0.0, 0.0, MIN_LEN);
			vec_len = MIN_LEN;
		}
		let half_deviation_cos = vec_len / brightness_sum;
		let deviation = (1.0 - half_deviation_cos).max(0.0).min(1.0); // TODO - select proper formula.

		let light_sum_brightness = get_light_brightness(&light_sum).max(MIN_LEN);
		let color = [
			light_sum[0] / light_sum_brightness,
			light_sum[1] / light_sum_brightness,
			light_sum[2] / light_sum_brightness,
		];

		DirectionalLightParams {
			direction_vector_scaled: scaled_vecs_sum,
			deviation,
			color,
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
			let scale = 255.0 * 8.0 * (TEXTURE_AREA as f32);
			let r = (src[0] * scale).max(0.0).min(255.0) as u8;
			let g = (src[1] * scale).max(0.0).min(255.0) as u8;
			let b = (src[2] * scale).max(0.0).min(255.0) as u8;
			*dst = Color32::from_rgba(r, g, b, 255);
		}
		image::save(&img, file_path);
	}
}

pub struct DirectionalLightParams
{
	pub direction_vector_scaled: Vec3f,
	pub deviation: f32,
	pub color: [f32; 3],
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

fn unproject_texture_coord(x: u32, y: u32) -> Vec3f
{
	let projection_point = (Vec2f::new(x as f32 + 0.5, y as f32 + 0.5) -
		Vec2f::new(HALF_TEXTURE_SIZE_F, HALF_TEXTURE_SIZE_F)) *
		(2.0_f32.sqrt() / HALF_TEXTURE_SIZE_F);
	unproject_normalized_coord(&projection_point)
}

// Unproject projection with size +-sqrt(2) for hemisphere and +-2 for sphere.
// Produces normalized vector.
fn unproject_normalized_coord(coord: &Vec2f) -> Vec3f
{
	let coord_square_len = coord.magnitude2();
	let xy_scale = ((1.0 - coord_square_len * 0.25).max(0.0)).sqrt();
	Vec3f::new(xy_scale * coord.x, xy_scale * coord.y, 1.0 - coord_square_len * 0.5)
}

fn get_pixel_address(x: u32, y: u32) -> usize
{
	(x + y * TEXTURE_SIZE) as usize
}

fn get_light_brightness(light: &[f32; 3]) -> f32
{
	// YCbCr-like RGB to grayscale conversion.
	light[0] * 0.299 + light[1] * 0.587 + light[2] * 0.114
}
