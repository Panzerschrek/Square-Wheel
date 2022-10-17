use crate::common::{color::*, system_window};

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
		let scaled_width = surface_info.width / scale;
		let scaled_height = surface_info.height / scale;
		for y in 0 .. scaled_height
		{
			for x in 0 .. scaled_width
			{
				pixels[x + y * surface_info.pitch] = self.buffer[x + y * scaled_width];
			}
		}
	}
}
