use super::{bsp_map_compact, math_types::*};

pub struct LightCube
{
	light_cube: [[f32; 3]; 6],
	// Store sum of scaled vectors for each component.
	sum_scaled_light_dirs: [Vec3f; 3],
}

impl LightCube
{
	pub fn new() -> Self
	{
		Self {
			light_cube: [[0.0; 3]; 6],
			sum_scaled_light_dirs: [Vec3f::zero(); 3],
		}
	}

	pub fn add_light_sample(&mut self, direction: &Vec3f, light_scaled: &[f32; 3])
	{
		let direction_normalized = direction / direction.magnitude().max(MIN_VEC_LEN);
		if direction_normalized.x <= 0.0
		{
			for i in 0 .. 3
			{
				self.light_cube[0][i] += light_scaled[i] * (-direction_normalized.x);
			}
		}
		else
		{
			for i in 0 .. 3
			{
				self.light_cube[1][i] += light_scaled[i] * direction_normalized.x;
			}
		}
		if direction_normalized.y <= 0.0
		{
			for i in 0 .. 3
			{
				self.light_cube[2][i] += light_scaled[i] * (-direction_normalized.y);
			}
		}
		else
		{
			for i in 0 .. 3
			{
				self.light_cube[3][i] += light_scaled[i] * direction_normalized.y;
			}
		}
		if direction_normalized.z <= 0.0
		{
			for i in 0 .. 3
			{
				self.light_cube[4][i] += light_scaled[i] * (-direction_normalized.z);
			}
		}
		else
		{
			for i in 0 .. 3
			{
				self.light_cube[5][i] += light_scaled[i] * direction_normalized.z;
			}
		}

		for i in 0 .. 3
		{
			self.sum_scaled_light_dirs[i] += direction_normalized * light_scaled[i];
		}
	}

	pub fn add_constant_light(&mut self, light: &[f32; 3])
	{
		for side in &mut self.light_cube
		{
			*side = *light;
		}
	}

	pub fn scale(&mut self, scale: f32)
	{
		for side in &mut self.light_cube
		{
			for component in side
			{
				*component *= scale;
			}
		}
		for i in 0 .. 3
		{
			self.sum_scaled_light_dirs[i] *= scale;
		}
	}

	pub fn convert_into_light_grid_sample(&self) -> bsp_map_compact::LightGridElement
	{
		// Calculate common direction for all 3 color components.
		// TODO - maybe use different weight for different components?
		let light_direction_vector_scaled =
			0.33 * (self.sum_scaled_light_dirs[0] + self.sum_scaled_light_dirs[1] + self.sum_scaled_light_dirs[2]);

		let light_direction_vector_scaled_len2 = light_direction_vector_scaled.magnitude2().max(MIN_VEC_LEN);
		let light_color = [
			self.sum_scaled_light_dirs[0]
				.dot(light_direction_vector_scaled)
				.max(0.0) / light_direction_vector_scaled_len2,
			self.sum_scaled_light_dirs[1]
				.dot(light_direction_vector_scaled)
				.max(0.0) / light_direction_vector_scaled_len2,
			self.sum_scaled_light_dirs[2]
				.dot(light_direction_vector_scaled)
				.max(0.0) / light_direction_vector_scaled_len2,
		];

		// Subtract dominant light vector from light cube.
		let mut light_cube_corrected = self.light_cube;
		for i in 0 .. 3
		{
			if light_direction_vector_scaled.x <= 0.0
			{
				light_cube_corrected[0][i] =
					(light_cube_corrected[0][i] - light_color[i] * (-light_direction_vector_scaled.x)).max(0.0);
			}
			else
			{
				light_cube_corrected[1][i] =
					(light_cube_corrected[1][i] - light_color[i] * light_direction_vector_scaled.x).max(0.0);
			}
			if light_direction_vector_scaled.y <= 0.0
			{
				light_cube_corrected[2][i] =
					(light_cube_corrected[2][i] - light_color[i] * (-light_direction_vector_scaled.y)).max(0.0);
			}
			else
			{
				light_cube_corrected[3][i] =
					(light_cube_corrected[3][i] - light_color[i] * light_direction_vector_scaled.y).max(0.0);
			}
			if light_direction_vector_scaled.z <= 0.0
			{
				light_cube_corrected[4][i] =
					(light_cube_corrected[4][i] - light_color[i] * (-light_direction_vector_scaled.z)).max(0.0);
			}
			else
			{
				light_cube_corrected[5][i] =
					(light_cube_corrected[5][i] - light_color[i] * light_direction_vector_scaled.z).max(0.0);
			}
		}

		bsp_map_compact::LightGridElement {
			light_cube: light_cube_corrected,
			light_direction_vector_scaled: light_direction_vector_scaled,
			directional_light_color: light_color,
		}
	}
}

const MIN_VEC_LEN: f32 = 0.00000001;
