use crate::common::{color::*, system_window};

pub const MAX_FRAME_SCALE: usize = 6;

pub struct FrameUpscaler
{
	buffer: Vec<Color32>,
}

impl FrameUpscaler
{
	pub fn new() -> Self
	{
		Self { buffer: Vec::new() }
	}

	pub fn get_draw_buffer(
		&mut self,
		surface_info: &system_window::SurfaceInfo,
		scale: usize,
	) -> (&mut [Color32], system_window::SurfaceInfo)
	{
		let width = surface_info.width / scale;
		let height = surface_info.height / scale;

		let target_size = width * height;
		if self.buffer.len() < target_size
		{
			self.buffer.resize(target_size, Color32::black());
		}
		(
			&mut self.buffer,
			system_window::SurfaceInfo {
				width,
				height,
				pitch: width,
			},
		)
	}

	pub fn perform_upscale(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		scale: usize,
		interpolate: bool,
	)
	{
		match scale
		{
			1 => self.perform_upscale_impl::<1>(pixels, surface_info),
			2 =>
			{
				if interpolate
				{
					self.perform_upscale_2x_linear(pixels, surface_info)
				}
				else
				{
					self.perform_upscale_impl::<2>(pixels, surface_info)
				}
			},
			3 => self.perform_upscale_impl::<3>(pixels, surface_info),
			4 => self.perform_upscale_impl::<4>(pixels, surface_info),
			5 => self.perform_upscale_impl::<5>(pixels, surface_info),
			6 => self.perform_upscale_impl::<6>(pixels, surface_info),
			_ => panic!("Unsupported scale {}", scale),
		}
	}

	fn perform_upscale_impl<const SCALE: usize>(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
	)
	{
		let scaled_width = surface_info.width / SCALE;
		let scaled_height = surface_info.height / SCALE;
		let buffer_size = scaled_width * scaled_height;
		if self.buffer.len() < buffer_size
		{
			// Something went wrong.
			return;
		}

		// There is no reason to use multithreading here, since upscaling operation is mostly memory-bounded.

		let width_scaled_up = scaled_width * SCALE;
		let height_scaled_up = scaled_height * SCALE;

		for src_y in 0 .. scaled_height
		{
			let src_line = &self.buffer[src_y * scaled_width .. (src_y + 1) * scaled_width];
			let dst_line_start = &mut pixels[src_y * SCALE * surface_info.pitch ..];
			for (src_x, &src_pixel) in src_line.iter().enumerate()
			{
				let dst_x_start = src_x * SCALE;
				for dy in 0 .. SCALE
				{
					for dx in 0 .. SCALE
					{
						// TODO - use unchecked indexing.
						dst_line_start[dst_x_start + dx + dy * surface_info.pitch] = src_pixel;
					}
				}
			} // for src_x

			// Leftover column.
			if width_scaled_up < surface_info.width
			{
				let src_pixel = src_line[scaled_width - 1];
				for dst_x in width_scaled_up .. surface_info.width
				{
					for dy in 0 .. SCALE
					{
						// TODO - use unchecked indexing.
						dst_line_start[dst_x + dy * surface_info.pitch] = src_pixel;
					}
				}
			}
		} // for src_y

		// Leftover row.
		if height_scaled_up < surface_info.height
		{
			let dy_left = surface_info.height - height_scaled_up;
			let src_y = scaled_height - 1;
			let src_line = &self.buffer[src_y * scaled_width .. (src_y + 1) * scaled_width];
			let dst_line_start = &mut pixels[scaled_height * SCALE * surface_info.pitch ..];
			for (src_x, &src_pixel) in src_line.iter().enumerate()
			{
				let dst_x_start = src_x * SCALE;
				for dy in 0 .. dy_left
				{
					for dx in 0 .. SCALE
					{
						// TODO - use unchecked indexing.
						dst_line_start[dst_x_start + dx + dy * surface_info.pitch] = src_pixel;
					}
				}
			} // for src_x

			// Leftover corner.
			if width_scaled_up < surface_info.width
			{
				let src_pixel = src_line[scaled_width - 1];
				for dst_x in width_scaled_up .. surface_info.width
				{
					for dy in 0 .. dy_left
					{
						// TODO - use unchecked indexing.
						dst_line_start[dst_x + dy * surface_info.pitch] = src_pixel;
					}
				}
			}
		}
	}

	fn perform_upscale_2x_linear(&mut self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		let scaled_width = surface_info.width / 2;
		let scaled_height = surface_info.height / 2;
		let buffer_size = scaled_width * scaled_height;
		if self.buffer.len() < buffer_size
		{
			// Something went wrong.
			return;
		}

		for src_y in 0 .. scaled_height - 1
		{
			let src_line = &self.buffer[src_y * scaled_width ..];
			let dst_line_start = &mut pixels[src_y * 2 * surface_info.pitch ..];
			for src_x in 0 .. scaled_width - 1
			{
				// TODO - use unchecked indexing.
				let pix_00 = src_line[src_x];
				let pix_10 = src_line[src_x + 1];
				let pix_01 = src_line[scaled_width + src_x];
				let pix_11 = src_line[scaled_width + src_x + 1];

				let x_mix = Color32::get_average(pix_00, pix_10);
				let y_mix = Color32::get_average(pix_00, pix_01);
				let corner_mix = Color32::get_average(x_mix, Color32::get_average(pix_01, pix_11));
				let dst_x = src_x * 2;
				dst_line_start[dst_x] = pix_00;
				dst_line_start[dst_x + 1] = x_mix;
				dst_line_start[surface_info.pitch + dst_x] = y_mix;
				dst_line_start[surface_info.pitch + dst_x + 1] = corner_mix;
			} // for src_x

			// Leftover column.
			let last_pix_0 = src_line[scaled_width - 1];
			let last_pix_1 = src_line[scaled_width - 1 + scaled_width];
			let last_pix_mix = Color32::get_average(last_pix_0, last_pix_1);
			for dst_x in (scaled_width - 1) * 2 .. surface_info.width
			{
				dst_line_start[dst_x] = last_pix_mix;
				dst_line_start[dst_x + surface_info.pitch] = last_pix_mix;
			}
		} // for src_y

		// Leftover row.
		let src_y = scaled_height - 1;
		let y_left = surface_info.height - (scaled_height - 1) * 2;
		let src_line = &self.buffer[src_y * scaled_width ..];
		let dst_line_start = &mut pixels[src_y * 2 * surface_info.pitch ..];
		for src_x in 0 .. scaled_width - 1
		{
			let pix_0 = src_line[src_x];
			let pix_1 = src_line[src_x + 1];
			let x_mix = Color32::get_average(pix_0, pix_1);
			let dst_x = src_x * 2;
			for dy in 0 .. y_left
			{
				let dst_offset = dy * surface_info.pitch + dst_x;
				dst_line_start[dst_offset] = pix_0;
				dst_line_start[dst_offset + 1] = x_mix;
			}
		}

		// Leftover corner.
		let last_pix = src_line[scaled_width - 1];
		for dst_x in (scaled_width - 1) * 2 .. surface_info.width
		{
			for dy in 0 .. y_left
			{
				dst_line_start[dy * surface_info.pitch + dst_x] = last_pix;
			}
		}
	}
}
