use super::fast_math::*;
use common::{color::*, system_window};

pub struct Postprocessor
{
	hdr_buffer_size: [usize; 2],
	hdr_buffer: Vec<Color64>,
}

impl Postprocessor
{
	pub fn new() -> Self
	{
		Self {
			hdr_buffer_size: [0, 0],
			hdr_buffer: Vec::new(),
		}
	}

	pub fn get_hdr_buffer(&mut self, size: [usize; 2]) -> &mut [Color64]
	{
		let required_size = size[0] * size[1];
		if self.hdr_buffer.len() < required_size
		{
			self.hdr_buffer.resize(required_size, Color64::black());
		}
		self.hdr_buffer_size = size;

		&mut self.hdr_buffer[.. required_size]
	}

	pub fn perform_postprocessing(&self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		let surface_size = [surface_info.width, surface_info.height];
		if self.hdr_buffer_size != surface_size
		{
			panic!(
				"Wrong buffer size, expected {:?}, got {:?}",
				self.hdr_buffer_size, surface_size
			);
		}

		// Use Reinhard formula for tonemapping.

		let scale = 1.0; // TODO - make this dependent on exposure.
		let inv_scale = 1.0 / scale;

		let inv_scale_vec = ColorVec::from_color_f32x3(&[inv_scale, inv_scale, inv_scale]);

		let inv_255 = 1.0 / 255.0;
		let inv_255_vec = ColorVec::from_color_f32x3(&[inv_255, inv_255, inv_255]);

		for y in 0 .. surface_size[1]
		{
			let src_line = &self.hdr_buffer[y * self.hdr_buffer_size[0] .. (y + 1) * self.hdr_buffer_size[0]];
			let dst_line = &mut pixels[y * surface_info.pitch .. (y + 1) * surface_info.pitch];
			for (dst, &src) in dst_line.iter_mut().zip(src_line.iter())
			{
				let c = ColorVec::from_color64(src);
				let c_mapped = ColorVec::div(&c, &ColorVec::mul_add(&c, &inv_255_vec, &inv_scale_vec));
				*dst = c_mapped.into();
			}
		}
	}
}
