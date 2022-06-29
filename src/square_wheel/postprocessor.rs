use super::{debug_stats_printer::*, fast_math::*, performance_counter::*};
use common::{color::*, shared_mut_slice::*, system_window};
use rayon::prelude::*;

pub struct Postprocessor
{
	hdr_buffer_size: [usize; 2],
	hdr_buffer: Vec<Color64>,
	performance_counters: PostprocessorPerformanceCounters,
	bloom_buffer_size: [usize; 2],
	bloom_buffers: [Vec<Color64>; 2],
}

struct PostprocessorPerformanceCounters
{
	tonemapping_duration: PerformanceCounter,
	bloom_duration: PerformanceCounter,
}

impl PostprocessorPerformanceCounters
{
	fn new() -> Self
	{
		let window_size = 100;
		Self {
			tonemapping_duration: PerformanceCounter::new(window_size),
			bloom_duration: PerformanceCounter::new(window_size),
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
			bloom_buffer_size: [0, 0],
			bloom_buffers: [Vec::new(), Vec::new()],
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

		self.bloom_buffer_size = [size[0] / BLOOM_BUFFER_SCALE, size[1] / BLOOM_BUFFER_SCALE];
		let bloom_buffer_required_size = self.bloom_buffer_size[0] * self.bloom_buffer_size[1];
		for bloom_buffer in &mut self.bloom_buffers
		{
			if bloom_buffer.len() < bloom_buffer_required_size
			{
				bloom_buffer.resize(bloom_buffer_required_size, Color64::black());
			}
		}

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

		let bloom_calculation_start_time = Clock::now();

		self.perform_bloom();

		let bloom_calculation_end_time = Clock::now();
		let bloom_duration_s = (bloom_calculation_end_time - bloom_calculation_start_time).as_secs_f32();
		self.performance_counters.bloom_duration.add_value(bloom_duration_s);

		let tonemapping_start_time = Clock::now();

		// Use Reinhard formula for tonemapping.

		let inv_scale = 1.0 / exposure;

		let inv_scale_vec = ColorVec::from_color_f32x3(&[inv_scale, inv_scale, inv_scale]);

		let inv_255 = 1.0 / 255.0;
		let inv_255_vec = ColorVec::from_color_f32x3(&[inv_255, inv_255, inv_255]);

		// It is safe to share destination buffer since each thead writes into its own regon.
		let pixels_shared = SharedMutSlice::new(pixels);

		let convert_line = |y| {
			let pixels_unshared = unsafe { pixels_shared.get() };
			let src_line = &self.hdr_buffer[y * self.hdr_buffer_size[0] .. (y + 1) * self.hdr_buffer_size[0]];
			let dst_line = &mut pixels_unshared[y * surface_info.pitch .. (y + 1) * surface_info.pitch];
			for (dst, &src) in dst_line.iter_mut().zip(src_line.iter())
			{
				let c = ColorVec::from_color64(src);
				let c_mapped = ColorVec::div(&c, &ColorVec::mul_add(&c, &inv_255_vec, &inv_scale_vec));
				*dst = c_mapped.into();
			}
		};

		let num_threads = rayon::current_num_threads();
		if num_threads == 1
		{
			for y in 0 .. surface_size[1]
			{
				convert_line(y);
			}

			// TEMP code. Fix it!!!
			let bloom_scale = 0.125;
			for src_y in 0 .. self.bloom_buffer_size[1]
			{
				for src_x in 0 .. self.bloom_buffer_size[0]
				{
					// TODO - use unchecked indexing operator.
					let bloom_src = self.bloom_buffers[0][src_x + src_y * self.bloom_buffer_size[0]];
					let bloom_c = ColorVec::from_color64(bloom_src);
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						for dy in 0 .. BLOOM_BUFFER_SCALE
						{
							// TODO - use unchecked indexing operator.
							let dst_x = src_x * BLOOM_BUFFER_SCALE + dx;
							let dst_y = src_y * BLOOM_BUFFER_SCALE + dy;
							let c = self.hdr_buffer[dst_x + dst_y * self.hdr_buffer_size[0]];
							let c_vec = ColorVec::from_color64(c);
							let sum = ColorVec::mul_scalar_add(&bloom_c, bloom_scale, &c_vec);
							let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
							pixels[dst_x + dst_y * surface_info.pitch] = c_mapped.into();
						}
					}
				}
			}
		}
		else
		{
			let mut ranges = [[0, 0]; 64];
			for i in 0 .. num_threads
			{
				ranges[i] = [
					surface_size[1] * i / num_threads,
					surface_size[1] * (i + 1) / num_threads,
				];
			}

			ranges[.. num_threads].par_iter().for_each(|range| {
				for y in range[0] .. range[1]
				{
					convert_line(y);
				}
			});
		}

		let tonemapping_end_time = Clock::now();
		let tonemapping_duration_s = (tonemapping_end_time - tonemapping_start_time).as_secs_f32();
		self.performance_counters
			.tonemapping_duration
			.add_value(tonemapping_duration_s);

		if debug_stats_printer.show_debug_stats()
		{
			debug_stats_printer.add_line(format!(
				"bloom time: {:04.2}ms",
				self.performance_counters.bloom_duration.get_average_value() * 1000.0
			));
			debug_stats_printer.add_line(format!(
				"tonemapping time: {:04.2}ms",
				self.performance_counters.tonemapping_duration.get_average_value() * 1000.0
			));
		}
	}

	fn perform_bloom(&mut self)
	{
		// First step - downsample HDR buffer into bloom buffer #0.
		let average_scaler = 1.0 / ((BLOOM_BUFFER_SCALE * BLOOM_BUFFER_SCALE) as f32);
		let average_scaler_vec = ColorVec::from_color_f32x3(&[average_scaler, average_scaler, average_scaler]);

		for dst_y in 0 .. self.bloom_buffer_size[1]
		{
			for dst_x in 0 .. self.bloom_buffer_size[0]
			{
				// TODO - use integer vector computations.
				let mut sum = ColorVec::zero();
				for dy in 0 .. BLOOM_BUFFER_SCALE
				{
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let src_x = dst_x * BLOOM_BUFFER_SCALE + dx;
						let src_y = dst_y * BLOOM_BUFFER_SCALE + dy;

						// TODO - use unchecked indexing operator.
						let src = self.hdr_buffer[src_x + src_y * self.hdr_buffer_size[0]];
						let c = ColorVec::from_color64(src);
						sum = ColorVec::add(&sum, &c);
					}
				}

				let average = ColorVec::mul(&sum, &average_scaler_vec);
				// TODO - use unchecked indexing operator.
				self.bloom_buffers[0][dst_x + dst_y * self.bloom_buffer_size[0]] = average.into_color64();
			}
		}

		// TODO - handle leftover pixels in borders.

		let sigma: f32 = 2.0;
		let blur_radius = ((3.0 * sigma - 0.5).ceil().max(0.0) as usize).max(MAX_GAUSSIAN_KERNEL_RADIUS);

		let blur_kernel = compute_gaussian_kernel(sigma, blur_radius);

		// TODO - speed-up bluring code. Use unchecked indexing, process borders specially.

		// Perform horizontal blur. Use buffer 0 as source and buffer 1 as destination.
		for dst_y in 0 .. self.bloom_buffer_size[1]
		{
			for dst_x in 0 .. self.bloom_buffer_size[0]
			{
				// TODO - use integer vector computations.
				let mut sum = ColorVec::zero();
				for dx in -(blur_radius as i32) ..= (blur_radius as i32)
				{
					let src_x = (dx + (dst_x as i32)).max(0).min(self.bloom_buffer_size[0] as i32 - 1);
					let src_y = dst_y;
					let src = self.bloom_buffers[0][(src_x as usize) + src_y * self.bloom_buffer_size[0]];
					let src_vec = ColorVec::from_color64(src);
					sum = ColorVec::mul_scalar_add(&src_vec, blur_kernel[(dx + (blur_radius as i32)) as usize], &sum);
				}

				// TODO - use unchecked indexing operator.
				self.bloom_buffers[1][dst_x + dst_y * self.bloom_buffer_size[0]] = sum.into_color64();
			}
		}

		// Perform vertical blur. Use buffer 1 as source and buffer 0 as destination.
		for dst_y in 0 .. self.bloom_buffer_size[1]
		{
			for dst_x in 0 .. self.bloom_buffer_size[0]
			{
				// TODO - use integer vector computations.
				let mut sum = ColorVec::zero();
				for dy in -(blur_radius as i32) ..= (blur_radius as i32)
				{
					let src_x = dst_x;
					let src_y = (dy + (dst_y as i32)).max(0).min(self.bloom_buffer_size[1] as i32 - 1);
					let src = self.bloom_buffers[1][src_x + (src_y as usize) * self.bloom_buffer_size[0]];
					let src_vec = ColorVec::from_color64(src);
					sum = ColorVec::mul_scalar_add(&src_vec, blur_kernel[(dy + (blur_radius as i32)) as usize], &sum);
				}

				// TODO - use unchecked indexing operator.
				self.bloom_buffers[0][dst_x + dst_y * self.bloom_buffer_size[0]] = sum.into_color64();
			}
		}
	}
}

const BLOOM_BUFFER_SCALE: usize = 4;

const MAX_GAUSSIAN_KERNEL_RADIUS: usize = 16;
const MAX_GAUSSIAN_KERNEL_SIZE: usize = 1 + 2 * MAX_GAUSSIAN_KERNEL_RADIUS;

fn compute_gaussian_kernel(sigma: f32, radius: usize) -> [f32; MAX_GAUSSIAN_KERNEL_SIZE]
{
	let mut result = [0.0; MAX_GAUSSIAN_KERNEL_SIZE];

	for x in -(radius as i32) ..= (radius as i32)
	{
		const SAMPLES: [f32; 4] = [-0.375, -0.125, 0.125, 0.375];
		let mut val = 0.0;
		for sample in SAMPLES
		{
			let coord = (x as f32) + sample;
			val += (-0.5 * (coord / sigma) * (coord / sigma)).exp() / (sigma * std::f32::consts::TAU.sqrt());
		}
		let average = val / (SAMPLES.len() as f32);

		result[(x + (radius as i32)) as usize] = average
	}

	result
}
