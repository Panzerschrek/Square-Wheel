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

	pub fn perform_upscale(&mut self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo, scale: usize)
	{
		match scale
		{
			1 => self.perform_upscale_impl::<1>(pixels, surface_info),
			2 => self.perform_upscale_impl::<2>(pixels, surface_info),
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
						dst_line_start[dst_x + dy * surface_info.pitch] = src_pixel;
					}
				}
			}
		}
	}
}
