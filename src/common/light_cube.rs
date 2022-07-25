use super::{bsp_map_compact, math_types::*};

pub struct LightCube
{
	light_cube: [[f32; 3]; 6],
}

impl LightCube
{
	pub fn new() -> Self
	{
		Self {
			light_cube: [[0.0; 3]; 6],
		}
	}

	pub fn add_light_sample(&mut self, direction: &Vec3f, light_scaled: &[f32; 3])
	{
		let direction_normalized = direction / direction.magnitude().max(0.0000001);
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
	}

	pub fn convert_into_light_grid_sample(&self) -> bsp_map_compact::LightGridElement
	{
		bsp_map_compact::LightGridElement {
			light_cube: self.light_cube,
			// TODO - calculate directional component.
			light_direction_vector_scaled: Vec3f::zero(),
			directional_light_color: [0.0; 3],
		}
	}
}
