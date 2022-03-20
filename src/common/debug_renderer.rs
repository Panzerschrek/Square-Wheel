use super::{color::*, fixed_math::*, system_window};

pub struct DebugRenderer<'a>
{
	color_buffer: &'a mut [Color32],
	width: i32,
	height: i32,
	row_size: i32,
	depth_buffer: Vec<f32>,
}

impl<'a> DebugRenderer<'a>
{
	pub fn new(color_buffer: &'a mut [Color32], surface_info: &system_window::SurfaceInfo) -> Self
	{
		DebugRenderer {
			color_buffer,
			width: surface_info.width as i32,
			height: surface_info.height as i32,
			row_size: (surface_info.pitch) as i32,
			depth_buffer: vec![1.0; surface_info.width * surface_info.pitch],
		}
	}

	pub fn get_width(&self) -> i32
	{
		self.width
	}

	pub fn get_height(&self) -> i32
	{
		self.height
	}

	pub fn draw_line(&mut self, mut v0: PointProjected, mut v1: PointProjected, color: Color32)
	{
		// TODO - optimize this. Discard lines totally outside viewport.
		// TODO - process depth using fixed values, instead of floating point.

		if (v1.x - v0.x).abs() >= (v1.y - v0.y).abs()
		{
			if v0.x > v1.x
			{
				std::mem::swap(&mut v0, &mut v1);
			}
			if v0.x == v1.x
			{
				return;
			}

			let dy_dx = fixed16_div(v1.y - v0.y, v1.x - v0.x);
			let dz_dx = (v1.z - v0.z) / fixed16_to_f32(v1.x - v0.x);
			let x_int_start = fixed16_round_to_int(v0.x).max(0);
			let x_int_end = fixed16_round_to_int(v1.x).min(self.width);
			let x_delta = int_to_fixed16(x_int_start) + FIXED16_HALF - v0.x;
			let mut y = v0.y + fixed16_mul(x_delta, dy_dx) + FIXED16_HALF; // Add extra half to replace expensive "round" with cheap "floor" in loop.
			let mut z = v0.z + fixed16_to_f32(x_delta) * dz_dx;
			for x_int in x_int_start .. x_int_end
			{
				let y_int = fixed16_floor_to_int(y);
				if y_int >= 0 && y_int < self.height
				{
					let pix_address = (x_int + y_int * self.row_size) as usize;
					if z <= self.depth_buffer[pix_address]
					{
						self.color_buffer[pix_address] = color;
						self.depth_buffer[pix_address] = z;
					}
				}
				y += dy_dx;
				z += dz_dx;
			}
		}
		else
		{
			if v0.y > v1.y
			{
				std::mem::swap(&mut v0, &mut v1);
			}
			if v0.y == v1.y
			{
				return;
			}

			let dx_dy = fixed16_div(v1.x - v0.x, v1.y - v0.y);
			let dz_dy = (v1.z - v0.z) / fixed16_to_f32(v1.y - v0.y);
			let y_int_start = fixed16_round_to_int(v0.y).max(0);
			let y_int_end = fixed16_round_to_int(v1.y).min(self.height);
			let y_delta = int_to_fixed16(y_int_start) + FIXED16_HALF - v0.y;
			let mut x = v0.x + fixed16_mul(y_delta, dx_dy) + FIXED16_HALF; // Add extra half to replace expensive "round" with cheap "floor" in loop.
			let mut z = v0.z + fixed16_to_f32(y_delta) * dz_dy;
			for y_int in y_int_start .. y_int_end
			{
				let x_int = fixed16_floor_to_int(x);
				if x_int >= 0 && x_int < self.width
				{
					let pix_address = (x_int + y_int * self.row_size) as usize;
					if z <= self.depth_buffer[pix_address]
					{
						self.color_buffer[pix_address] = color;
						self.depth_buffer[pix_address] = z;
					}
				}
				x += dx_dy;
				z += dz_dy;
			}
		}
	}
}

pub struct PointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
	pub z: f32,
}
