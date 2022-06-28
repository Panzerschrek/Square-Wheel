use super::{debug_stats_printer::*, fast_math::*, performance_counter::*};
use common::{color::*, system_window};

pub struct Postprocessor
{
	hdr_buffer_size: [usize; 2],
	hdr_buffer: Vec<Color64>,
	performance_counters: PostprocessorPerformanceCounters,
}

struct PostprocessorPerformanceCounters
{
	tonemapping_duration: PerformanceCounter,
}

impl PostprocessorPerformanceCounters
{
	fn new() -> Self
	{
		let window_size = 100;
		Self {
			tonemapping_duration: PerformanceCounter::new(window_size),
		}
	}
}
type Clock = std::time::Instant;

impl Postprocessor
{
	pub fn new() -> Self
	{
		Self {
			hdr_buffer_size: [0, 0],
			hdr_buffer: Vec::new(),
			performance_counters: PostprocessorPerformanceCounters::new(),
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

	pub fn perform_postprocessing(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		exposure: f32,
		debug_stats_printer: &mut DebugStatsPrinter,
	)
	{
		let surface_size = [surface_info.width, surface_info.height];
		if self.hdr_buffer_size != surface_size
		{
			panic!(
				"Wrong buffer size, expected {:?}, got {:?}",
				self.hdr_buffer_size, surface_size
			);
		}

		let tonemapping_start_time = Clock::now();

		// Use Reinhard formula for tonemapping.

		let inv_scale = 1.0 / exposure;

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

		let tonemapping_end_time = Clock::now();
		let mtonemapping_duration_s = (tonemapping_end_time - tonemapping_start_time).as_secs_f32();
		self.performance_counters
			.tonemapping_duration
			.add_value(mtonemapping_duration_s);

		if debug_stats_printer.show_debug_stats()
		{
			debug_stats_printer.add_line(format!(
				"tonemapping time: {:04.2}ms",
				self.performance_counters.tonemapping_duration.get_average_value() * 1000.0
			));
		}
	}
}
