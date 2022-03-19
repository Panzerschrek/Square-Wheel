use super::{fixed_math::*, system_window};

pub struct DebugRenderer<'a>
{
	color_buffer: &'a mut [u8],
	width: i32,
	height: i32,
	row_size: i32,
}

impl<'a> DebugRenderer<'a>
{
	pub fn new(color_buffer: &'a mut [u8], surface_info: &system_window::SurfaceInfo) -> Self
	{
		DebugRenderer {
			color_buffer,
			width: surface_info.width as i32,
			height: surface_info.height as i32,
			row_size: (surface_info.pitch / 4) as i32,
		}
	}

	pub fn draw_line(&mut self, mut v0: PointProjected, mut v1: PointProjected)
	{
		// TODO - provide input color.
		// TODO - work with 32-bit values, not bytes.
		// TODO - optimize this. Discard lines totally outside viewport.

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
			let x_int_start = fixed16_round_to_int(v0.x).max(0);
			let x_int_end = fixed16_round_to_int(v1.x).min(self.width);
			let mut y = v0.y + fixed16_mul(int_to_fixed16(x_int_start) + FIXED16_HALF - v0.x, dy_dx);
			y += FIXED16_HALF; // Add extra half to replace expensive "round" with cheap "floor" in loop.
			for x_int in x_int_start .. x_int_end
			{
				let y_int = fixed16_floor_to_int(y);
				if y_int >= 0 && y_int < self.height
				{
					let pix_address = (4 * (x_int + y_int * self.row_size)) as usize;
					self.color_buffer[pix_address] = 255;
					self.color_buffer[pix_address + 1] = 255;
					self.color_buffer[pix_address + 2] = 255;
					self.color_buffer[pix_address + 3] = 255;
				}
				y += dy_dx;
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
			let y_int_start = fixed16_round_to_int(v0.y).max(0);
			let y_int_end = fixed16_round_to_int(v1.y).min(self.height);
			let mut x = v0.x + fixed16_mul(int_to_fixed16(y_int_start) + FIXED16_HALF - v0.y, dx_dy);
			x += FIXED16_HALF; // Add extra half to replace expensive "round" with cheap "floor" in loop.
			for y_int in y_int_start .. y_int_end
			{
				let x_int = fixed16_floor_to_int(x);
				if x_int >= 0 && x_int < self.width
				{
					let pix_address = (4 * (x_int + y_int * self.row_size)) as usize;
					self.color_buffer[pix_address] = 255;
					self.color_buffer[pix_address + 1] = 255;
					self.color_buffer[pix_address + 2] = 255;
					self.color_buffer[pix_address + 3] = 255;
				}
				x += dx_dy;
			}
		}
	}
}

pub struct PointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
}
