use super::{config, debug_stats_printer::*, fast_math::*, performance_counter::*, postprocessor_config::*};
use common::{color::*, shared_mut_slice::*, system_window};
use rayon::prelude::*;

pub struct Postprocessor
{
	app_config: config::ConfigSharedPtr,
	config: PostprocessorConfig,

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
	pub fn new(app_config: config::ConfigSharedPtr) -> Self
	{
		let config_parsed = PostprocessorConfig::from_app_config(&app_config);
		config_parsed.update_app_config(&app_config); // Update JSON with struct fields.

		Self {
			app_config,
			config: config_parsed,
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

		&mut self.hdr_buffer[.. required_size]
	}

	pub fn perform_postprocessing(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
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

		let use_bloom = self.config.bloom_sigma > 0.0;

		let mut bloom_buffer_scale = 1;
		if use_bloom
		{
			let bloom_calculation_start_time = Clock::now();

			bloom_buffer_scale = self.perform_bloom();

			let bloom_calculation_end_time = Clock::now();
			let bloom_duration_s = (bloom_calculation_end_time - bloom_calculation_start_time).as_secs_f32();
			self.performance_counters.bloom_duration.add_value(bloom_duration_s);
		}

		let tonemapping_start_time = Clock::now();

		// Use Reinhard formula for tonemapping.

		let inv_scale = 1.0 / self.config.exposure;
		let inv_scale_vec = ColorVec::from_color_f32x3(&[inv_scale, inv_scale, inv_scale]);

		let inv_255 = 1.0 / 255.0;
		let inv_255_vec = ColorVec::from_color_f32x3(&[inv_255, inv_255, inv_255]);

		if use_bloom
		{
			self.perform_tonemapping_with_bloom(pixels, surface_info, inv_scale_vec, inv_255_vec, bloom_buffer_scale);
		}
		else
		{
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
		}

		let tonemapping_end_time = Clock::now();
		let tonemapping_duration_s = (tonemapping_end_time - tonemapping_start_time).as_secs_f32();
		self.performance_counters
			.tonemapping_duration
			.add_value(tonemapping_duration_s);

		if debug_stats_printer.show_debug_stats()
		{
			debug_stats_printer.add_line(format!(
				"bloom time: {:04.2}ms (scale {})",
				self.performance_counters.bloom_duration.get_average_value() * 1000.0,
				bloom_buffer_scale,
			));
			debug_stats_printer.add_line(format!(
				"tonemapping time: {:04.2}ms",
				self.performance_counters.tonemapping_duration.get_average_value() * 1000.0
			));
		}
	}

	fn perform_tonemapping_with_bloom(
		&self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		inv_scale_vec: ColorVec,
		inv_255_vec: ColorVec,
		bloom_buffer_scale: usize,
	)
	{
		match bloom_buffer_scale
		{
			0 => self.perform_tonemapping_with_bloom_impl::<MIN_BLOOM_BUFFER_SCALE>(
				pixels,
				surface_info,
				inv_scale_vec,
				inv_255_vec,
			),
			1 => self.perform_tonemapping_with_bloom_impl::<MIN_BLOOM_BUFFER_SCALE>(
				pixels,
				surface_info,
				inv_scale_vec,
				inv_255_vec,
			),
			2 => self.perform_tonemapping_with_bloom_impl::<2>(pixels, surface_info, inv_scale_vec, inv_255_vec),
			4 => self.perform_tonemapping_with_bloom_impl::<4>(pixels, surface_info, inv_scale_vec, inv_255_vec),
			8 => self.perform_tonemapping_with_bloom_impl::<8>(pixels, surface_info, inv_scale_vec, inv_255_vec),
			_ => self.perform_tonemapping_with_bloom_impl::<MAX_BLOOM_BUFFER_SCALE>(
				pixels,
				surface_info,
				inv_scale_vec,
				inv_255_vec,
			),
		}
	}

	fn perform_tonemapping_with_bloom_impl<const BLOOM_BUFFER_SCALE: usize>(
		&self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		inv_scale_vec: ColorVec,
		inv_255_vec: ColorVec,
	)
	{
		if !self.config.linear_bloom_filter
		{
			self.perform_tonemapping_with_bloom_nearest::<BLOOM_BUFFER_SCALE>(
				pixels,
				surface_info,
				inv_scale_vec,
				inv_255_vec,
			);
		}
		else
		{
			self.perform_tonemapping_with_bloom_linear::<BLOOM_BUFFER_SCALE>(
				pixels,
				surface_info,
				inv_scale_vec,
				inv_255_vec,
			);
		}
	}

	fn perform_tonemapping_with_bloom_nearest<const BLOOM_BUFFER_SCALE: usize>(
		&self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		inv_scale_vec: ColorVec,
		inv_255_vec: ColorVec,
	)
	{
		let columns_left = self.hdr_buffer_size[0] - self.bloom_buffer_size[0] * BLOOM_BUFFER_SCALE;
		let lines_left = self.hdr_buffer_size[1] - self.bloom_buffer_size[1] * BLOOM_BUFFER_SCALE;

		for src_y in 0 .. self.bloom_buffer_size[1]
		{
			let dst_y_base = src_y * BLOOM_BUFFER_SCALE;
			let src_line_offset = src_y * self.bloom_buffer_size[0];
			for src_x in 0 .. self.bloom_buffer_size[0]
			{
				let dst_x_base = src_x * BLOOM_BUFFER_SCALE;
				let bloom_src = debug_checked_fetch(&self.bloom_buffers[0], src_x + src_line_offset);
				let bloom_c = ColorVec::from_color64(bloom_src);
				for dy in 0 .. BLOOM_BUFFER_SCALE
				{
					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let c_vec = ColorVec::from_color64(c);
						let sum = ColorVec::add(&bloom_c, &c_vec);
						// let sum = bloom_c;
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}
			}
			// Leftover columns.
			if columns_left > 0
			{
				let dst_x_base = self.bloom_buffer_size[0] * BLOOM_BUFFER_SCALE;
				let bloom_src = debug_checked_fetch(
					&self.bloom_buffers[0],
					(self.bloom_buffer_size[0] - 1) + src_line_offset,
				);
				let bloom_c = ColorVec::from_color64(bloom_src);
				for dy in 0 .. BLOOM_BUFFER_SCALE
				{
					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. columns_left
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let c_vec = ColorVec::from_color64(c);
						let sum = ColorVec::add(&bloom_c, &c_vec);
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}
			}
		}
		// Leftover lines.
		if lines_left > 0
		{
			let src_line_offset = (self.bloom_buffer_size[1] - 1) * self.bloom_buffer_size[0];
			let dst_y_base = self.bloom_buffer_size[1] * BLOOM_BUFFER_SCALE;
			for src_x in 0 .. self.bloom_buffer_size[0]
			{
				let dst_x_base = src_x * BLOOM_BUFFER_SCALE;
				let bloom_src = debug_checked_fetch(&self.bloom_buffers[0], src_x + src_line_offset);
				let bloom_c = ColorVec::from_color64(bloom_src);
				for dy in 0 .. lines_left
				{
					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let c_vec = ColorVec::from_color64(c);
						let sum = ColorVec::add(&bloom_c, &c_vec);
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}
			}
			// Leftover corner.
			if columns_left > 0
			{
				let dst_x_base = self.bloom_buffer_size[0] * BLOOM_BUFFER_SCALE;
				let bloom_src = debug_checked_fetch(
					&self.bloom_buffers[0],
					(self.bloom_buffer_size[0] - 1) + src_line_offset,
				);
				let bloom_c = ColorVec::from_color64(bloom_src);
				for dy in 0 .. lines_left
				{
					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. columns_left
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let c_vec = ColorVec::from_color64(c);
						let sum = ColorVec::add(&bloom_c, &c_vec);
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}
			}
		}
	}

	fn perform_tonemapping_with_bloom_linear<const BLOOM_BUFFER_SCALE: usize>(
		&self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		inv_scale_vec: ColorVec,
		inv_255_vec: ColorVec,
	)
	{
		let mut mix_factors = [0.0; BLOOM_BUFFER_SCALE];
		let bloom_buffer_scale_f = BLOOM_BUFFER_SCALE as f32;
		for i in 0 .. BLOOM_BUFFER_SCALE
		{
			mix_factors[i] = (((BLOOM_BUFFER_SCALE + 1) & 1) as f32) / (2.0 * bloom_buffer_scale_f) +
				(i as f32) / bloom_buffer_scale_f;
		}

		let right_border_begin_x = (self.bloom_buffer_size[0] - 1) * BLOOM_BUFFER_SCALE + BLOOM_BUFFER_SCALE / 2;
		let right_border_size = self.hdr_buffer_size[0] - right_border_begin_x;

		let upper_border_begin_y = (self.bloom_buffer_size[1] - 1) * BLOOM_BUFFER_SCALE + BLOOM_BUFFER_SCALE / 2;
		let upper_border_size = self.hdr_buffer_size[1] - upper_border_begin_y;

		// Lower border.
		{
			let dst_y_base = 0;
			let mut bloom_src_0_0 = ColorVec::from_color64(debug_checked_fetch(&self.bloom_buffers[0], 0));
			for src_x in 0 .. self.bloom_buffer_size[0] - 1
			{
				let dst_x_base = src_x * BLOOM_BUFFER_SCALE + BLOOM_BUFFER_SCALE / 2;

				let bloom_src_1_0 = ColorVec::from_color64(debug_checked_fetch(&self.bloom_buffers[0], src_x + 1));

				for dy in 0 .. BLOOM_BUFFER_SCALE / 2
				{
					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let x_mix_factor = mix_factors[dx];
						let one_minus_x_mix_factor = 1.0 - x_mix_factor;
						let sum = ColorVec::mul_scalar_add(
							&bloom_src_0_0,
							one_minus_x_mix_factor,
							&ColorVec::mul_scalar_add(&bloom_src_1_0, x_mix_factor, &ColorVec::from_color64(c)),
						);
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}

				bloom_src_0_0 = bloom_src_1_0;
			}
		}
		// Main image.
		for src_y in 0 .. self.bloom_buffer_size[1] - 1
		{
			let dst_y_base = src_y * BLOOM_BUFFER_SCALE + BLOOM_BUFFER_SCALE / 2;
			let src_line_offset = src_y * self.bloom_buffer_size[0];

			let mut bloom_src_0_0 =
				ColorVec::from_color64(debug_checked_fetch(&self.bloom_buffers[0], src_line_offset));
			let mut bloom_src_0_1 = ColorVec::from_color64(debug_checked_fetch(
				&self.bloom_buffers[0],
				src_line_offset + self.bloom_buffer_size[0],
			));

			// Left border.
			for dy in 0 .. BLOOM_BUFFER_SCALE
			{
				let y_mix_factor = mix_factors[dy];
				let one_minus_y_mix_factor = 1.0 - y_mix_factor;
				let bloom_src = ColorVec::mul_scalar_add(
					&bloom_src_0_0,
					one_minus_y_mix_factor,
					&ColorVec::scalar_mul(&bloom_src_0_1, y_mix_factor),
				);

				let dst_y = dst_y_base + dy;
				let hdr_buffer_line_offset = dst_y * self.hdr_buffer_size[0];
				let pixels_line_offset = dst_y * surface_info.pitch;
				for dx in 0 .. BLOOM_BUFFER_SCALE / 2
				{
					let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
					let sum = ColorVec::add(&bloom_src, &ColorVec::from_color64(c));
					let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
					debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
				}
			}
			// Main image.
			for src_x in 0 .. self.bloom_buffer_size[0] - 1
			{
				let dst_x_base = src_x * BLOOM_BUFFER_SCALE + BLOOM_BUFFER_SCALE / 2;

				let bloom_src_1_0 =
					ColorVec::from_color64(debug_checked_fetch(&self.bloom_buffers[0], src_x + 1 + src_line_offset));
				let bloom_src_1_1 = ColorVec::from_color64(debug_checked_fetch(
					&self.bloom_buffers[0],
					src_x + 1 + src_line_offset + self.bloom_buffer_size[0],
				));

				for dy in 0 .. BLOOM_BUFFER_SCALE
				{
					let y_mix_factor = mix_factors[dy];
					let one_minus_y_mix_factor = 1.0 - y_mix_factor;
					let bloom_src_left = ColorVec::mul_scalar_add(
						&bloom_src_0_0,
						one_minus_y_mix_factor,
						&ColorVec::scalar_mul(&bloom_src_0_1, y_mix_factor),
					);
					let bloom_src_right = ColorVec::mul_scalar_add(
						&bloom_src_1_0,
						one_minus_y_mix_factor,
						&ColorVec::scalar_mul(&bloom_src_1_1, y_mix_factor),
					);

					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let x_mix_factor = mix_factors[dx];
						let one_minus_x_mix_factor = 1.0 - x_mix_factor;
						let sum = ColorVec::mul_scalar_add(
							&bloom_src_left,
							one_minus_x_mix_factor,
							&ColorVec::mul_scalar_add(&bloom_src_right, x_mix_factor, &ColorVec::from_color64(c)),
						);
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}

				bloom_src_0_0 = bloom_src_1_0;
				bloom_src_0_1 = bloom_src_1_1;
			}
			// Right border.
			for dy in 0 .. BLOOM_BUFFER_SCALE
			{
				let y_mix_factor = mix_factors[dy];
				let one_minus_y_mix_factor = 1.0 - y_mix_factor;
				let bloom_src = ColorVec::mul_scalar_add(
					&bloom_src_0_0,
					one_minus_y_mix_factor,
					&ColorVec::scalar_mul(&bloom_src_0_1, y_mix_factor),
				);

				let dst_y = dst_y_base + dy;
				let hdr_buffer_line_offset = right_border_begin_x + dst_y * self.hdr_buffer_size[0];
				let pixels_line_offset = right_border_begin_x + dst_y * surface_info.pitch;
				for dx in 0 .. right_border_size
				{
					let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
					let sum = ColorVec::add(&bloom_src, &ColorVec::from_color64(c));
					let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
					debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
				}
			}
		}
		// Upper border.
		{
			let dst_y_base = upper_border_begin_y;

			let src_line_offset = (self.bloom_buffer_size[1] - 1) * self.bloom_buffer_size[0];
			let mut bloom_src_0_0 =
				ColorVec::from_color64(debug_checked_fetch(&self.bloom_buffers[0], src_line_offset));
			for src_x in 0 .. self.bloom_buffer_size[0] - 1
			{
				let dst_x_base = src_x * BLOOM_BUFFER_SCALE + BLOOM_BUFFER_SCALE / 2;

				let bloom_src_1_0 =
					ColorVec::from_color64(debug_checked_fetch(&self.bloom_buffers[0], src_x + 1 + src_line_offset));

				for dy in 0 .. upper_border_size
				{
					let dst_y = dst_y_base + dy;
					let hdr_buffer_line_offset = dst_x_base + dst_y * self.hdr_buffer_size[0];
					let pixels_line_offset = dst_x_base + dst_y * surface_info.pitch;
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let c = debug_checked_fetch(&self.hdr_buffer, dx + hdr_buffer_line_offset);
						let x_mix_factor = mix_factors[dx];
						let one_minus_x_mix_factor = 1.0 - x_mix_factor;
						let sum = ColorVec::mul_scalar_add(
							&bloom_src_0_0,
							one_minus_x_mix_factor,
							&ColorVec::mul_scalar_add(&bloom_src_1_0, x_mix_factor, &ColorVec::from_color64(c)),
						);
						let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
						debug_checked_store(pixels, dx + pixels_line_offset, c_mapped.into());
					}
				}

				bloom_src_0_0 = bloom_src_1_0;
			}
		}

		// Process corners.
		let mut process_corner = |x_start, x_end, y_start, y_end, bloom_value| {
			for y in y_start .. y_end
			{
				let hdr_buffer_line_offset = y * self.hdr_buffer_size[0];
				let pixels_line_offset = y * surface_info.pitch;
				for x in x_start .. x_end
				{
					let c = debug_checked_fetch(&self.hdr_buffer, x + hdr_buffer_line_offset);
					let sum = ColorVec::add(&bloom_value, &ColorVec::from_color64(c));
					let c_mapped = ColorVec::div(&sum, &ColorVec::mul_add(&sum, &inv_255_vec, &inv_scale_vec));
					debug_checked_store(pixels, x + pixels_line_offset, c_mapped.into());
				}
			}
		};

		process_corner(
			0,
			BLOOM_BUFFER_SCALE / 2,
			0,
			BLOOM_BUFFER_SCALE / 2,
			ColorVec::from_color64(self.bloom_buffers[0][0]),
		);
		process_corner(
			right_border_begin_x,
			self.hdr_buffer_size[0],
			0,
			BLOOM_BUFFER_SCALE / 2,
			ColorVec::from_color64(self.bloom_buffers[0][self.bloom_buffer_size[0] - 1]),
		);
		process_corner(
			0,
			BLOOM_BUFFER_SCALE / 2,
			upper_border_begin_y,
			self.hdr_buffer_size[1],
			ColorVec::from_color64(self.bloom_buffers[0][(self.bloom_buffer_size[1] - 1) * self.bloom_buffer_size[0]]),
		);
		process_corner(
			right_border_begin_x,
			self.hdr_buffer_size[0],
			upper_border_begin_y,
			self.hdr_buffer_size[1],
			ColorVec::from_color64(
				self.bloom_buffers[0]
					[self.bloom_buffer_size[0] - 1 + (self.bloom_buffer_size[1] - 1) * self.bloom_buffer_size[0]],
			),
		);
	}

	fn perform_bloom(&mut self) -> usize
	{
		let bloom_buffer_scale = self
			.config
			.bloom_buffer_scale_log2
			.ceil()
			.max(MIN_BLOOM_BUFFER_SCALE_LOG2 as f32)
			.min(MAX_BLOOM_BUFFER_SCALE_LOG2 as f32)
			.exp2() as usize;

		self.bloom_buffer_size = [
			self.hdr_buffer_size[0] / bloom_buffer_scale,
			self.hdr_buffer_size[1] / bloom_buffer_scale,
		];
		let bloom_buffer_required_size = self.bloom_buffer_size[0] * self.bloom_buffer_size[1];
		for bloom_buffer in &mut self.bloom_buffers
		{
			if bloom_buffer.len() < bloom_buffer_required_size
			{
				bloom_buffer.resize(bloom_buffer_required_size, Color64::black());
			}
		}

		match bloom_buffer_scale
		{
			0 => self.downscale_hdr_buffer::<MIN_BLOOM_BUFFER_SCALE>(),
			1 => self.downscale_hdr_buffer::<MIN_BLOOM_BUFFER_SCALE>(),
			2 => self.downscale_hdr_buffer::<2>(),
			4 => self.downscale_hdr_buffer::<4>(),
			8 => self.downscale_hdr_buffer::<8>(),
			_ => self.downscale_hdr_buffer::<MAX_BLOOM_BUFFER_SCALE>(),
		}

		let bloom_sigma_corrected = self.config.bloom_sigma / (bloom_buffer_scale as f32);

		let blur_radius =
			((3.0 * bloom_sigma_corrected - 0.5).ceil().max(0.0) as usize).min(MAX_GAUSSIAN_KERNEL_RADIUS);
		let blur_kernel = compute_gaussian_kernel(bloom_sigma_corrected, blur_radius);
		// Use approach with constant blur size in order to use optimized (unrolled) code for each radius.
		match blur_radius
		{
			0 =>
			{},
			1 => self.perform_blur_impl::<1>(&blur_kernel),
			2 => self.perform_blur_impl::<2>(&blur_kernel),
			3 => self.perform_blur_impl::<3>(&blur_kernel),
			4 => self.perform_blur_impl::<4>(&blur_kernel),
			5 => self.perform_blur_impl::<5>(&blur_kernel),
			6 => self.perform_blur_impl::<6>(&blur_kernel),
			7 => self.perform_blur_impl::<7>(&blur_kernel),
			8 => self.perform_blur_impl::<8>(&blur_kernel),
			9 => self.perform_blur_impl::<9>(&blur_kernel),
			10 => self.perform_blur_impl::<10>(&blur_kernel),
			11 => self.perform_blur_impl::<11>(&blur_kernel),
			12 => self.perform_blur_impl::<12>(&blur_kernel),
			13 => self.perform_blur_impl::<13>(&blur_kernel),
			14 => self.perform_blur_impl::<14>(&blur_kernel),
			15 => self.perform_blur_impl::<15>(&blur_kernel),
			16 => self.perform_blur_impl::<16>(&blur_kernel),
			_ => self.perform_blur_impl::<MAX_GAUSSIAN_KERNEL_RADIUS>(&blur_kernel),
		}

		bloom_buffer_scale
	}

	fn downscale_hdr_buffer<const BLOOM_BUFFER_SCALE: usize>(&mut self)
	{
		const COLOR_SHIFT: i32 = 8;
		let average_scaler = (1 << COLOR_SHIFT) / ((BLOOM_BUFFER_SCALE * BLOOM_BUFFER_SCALE) as u32);

		for dst_y in 0 .. self.bloom_buffer_size[1]
		{
			let src_y_base = dst_y * BLOOM_BUFFER_SCALE;
			let dst_line_offset = dst_y * self.bloom_buffer_size[0];
			for dst_x in 0 .. self.bloom_buffer_size[0]
			{
				let src_x_base = dst_x * BLOOM_BUFFER_SCALE;
				let mut sum = ColorVecI::zero();
				for dy in 0 .. BLOOM_BUFFER_SCALE
				{
					let src_line_offset = (src_y_base + dy) * self.hdr_buffer_size[0];
					let src_offset = src_x_base + src_line_offset;
					for dx in 0 .. BLOOM_BUFFER_SCALE
					{
						let src = debug_checked_fetch(&self.hdr_buffer, dx + src_offset);
						let c = ColorVecI::from_color64(src);
						sum = ColorVecI::add(&sum, &c);
					}
				}

				let average = ColorVecI::shift_right::<COLOR_SHIFT>(&ColorVecI::mul_scalar(&sum, average_scaler));
				debug_checked_store(
					&mut self.bloom_buffers[0],
					dst_x + dst_line_offset,
					average.into_color64(),
				);
			}
		}

		// TODO - handle leftover pixels in borders.
	}

	fn perform_blur_impl<const RADIUS: usize>(&mut self, blur_kernel: &[f32; MAX_GAUSSIAN_KERNEL_SIZE])
	{
		const COLOR_SHIFT: i32 = 14;
		let mut blur_kernel_i = [0; MAX_GAUSSIAN_KERNEL_SIZE];
		for (dst, src) in blur_kernel_i.iter_mut().zip(blur_kernel.iter())
		{
			*dst = (src * ((1 << COLOR_SHIFT) as f32)) as u32;
		}

		// TODO - speed-up bluring code - process borders specially.
		let radius_i = RADIUS as i32;
		let bloom_buffer_size_minus_one = [
			self.bloom_buffer_size[0] as i32 - 1,
			self.bloom_buffer_size[1] as i32 - 1,
		];

		// Perform horizontal blur. Use buffer 0 as source and buffer 1 as destination.
		for dst_y in 0 .. self.bloom_buffer_size[1]
		{
			let line_offset = dst_y * self.bloom_buffer_size[0];
			for dst_x in 0 .. self.bloom_buffer_size[0]
			{
				// Optimization: fetch and scale center pixel, than fetch and scale pairs of side pixels.
				// We can do this since blur is symmetrical.
				let mut sum = ColorVecI::mul_scalar(
					&ColorVecI::from_color64(debug_checked_fetch(&self.bloom_buffers[0], dst_x + line_offset)),
					debug_checked_fetch(&blur_kernel_i, radius_i as usize),
				);
				for dx in 1 ..= radius_i
				{
					let src_x_0 = ((dst_x as i32) - dx).max(0);
					let src_x_1 = ((dst_x as i32) + dx).min(bloom_buffer_size_minus_one[0]);
					let src_0 = debug_checked_fetch(&self.bloom_buffers[0], (src_x_0 as usize) + line_offset);
					let src_1 = debug_checked_fetch(&self.bloom_buffers[0], (src_x_1 as usize) + line_offset);
					let two_sum = ColorVecI::add(&ColorVecI::from_color64(src_0), &ColorVecI::from_color64(src_1));
					let two_sum_scaled =
						ColorVecI::mul_scalar(&two_sum, debug_checked_fetch(&blur_kernel_i, (dx + radius_i) as usize));
					sum = ColorVecI::add(&sum, &two_sum_scaled);
				}

				let sum_shifted = ColorVecI::shift_right::<COLOR_SHIFT>(&sum);
				debug_checked_store(
					&mut self.bloom_buffers[1],
					dst_x + line_offset,
					sum_shifted.into_color64(),
				);
			}
		}

		// Reduce all weights by bloom scale factor for second blur pass.
		// Do this in order to avoid additional multiplication while applying bloom.
		for (dst, src) in blur_kernel_i.iter_mut().zip(blur_kernel.iter())
		{
			*dst = (src * self.config.bloom_scale * ((1 << COLOR_SHIFT) as f32)) as u32;
		}

		// Perform vertical blur. Use buffer 1 as source and buffer 0 as destination.
		for dst_y in 0 .. self.bloom_buffer_size[1]
		{
			let dst_line_offset = dst_y * self.bloom_buffer_size[0];
			for dst_x in 0 .. self.bloom_buffer_size[0]
			{
				let src_x = dst_x;
				let mut sum = ColorVecI::mul_scalar(
					&ColorVecI::from_color64(debug_checked_fetch(
						&self.bloom_buffers[1],
						src_x + dst_y * self.bloom_buffer_size[0],
					)),
					debug_checked_fetch(&blur_kernel_i, radius_i as usize),
				);

				for dy in 1 ..= radius_i
				{
					let src_y_0 = ((dst_y as i32) - dy).max(0);
					let src_y_1 = ((dst_y as i32) + dy).min(bloom_buffer_size_minus_one[1]);
					let src_0 = debug_checked_fetch(
						&self.bloom_buffers[1],
						src_x + (src_y_0 as usize) * self.bloom_buffer_size[0],
					);
					let src_1 = debug_checked_fetch(
						&self.bloom_buffers[1],
						src_x + (src_y_1 as usize) * self.bloom_buffer_size[0],
					);
					let two_sum = ColorVecI::add(&ColorVecI::from_color64(src_0), &ColorVecI::from_color64(src_1));
					let two_sum_scaled =
						ColorVecI::mul_scalar(&two_sum, debug_checked_fetch(&blur_kernel_i, (dy + radius_i) as usize));
					sum = ColorVecI::add(&sum, &two_sum_scaled);
				}

				let sum_shifted = ColorVecI::shift_right::<COLOR_SHIFT>(&sum);
				debug_checked_store(
					&mut self.bloom_buffers[0],
					dst_x + dst_line_offset,
					sum_shifted.into_color64(),
				);
			}
		}
	}

	pub fn synchronize_config(&mut self)
	{
		// TODO - fix wrong config params and update app config.
		self.config = PostprocessorConfig::from_app_config(&self.app_config);
	}

	pub fn use_hdr_rendering(&self) -> bool
	{
		self.config.hdr_rendering
	}
}

// Support only power of two scale to reduce specialized code size.
const MIN_BLOOM_BUFFER_SCALE_LOG2: usize = 1;
const MAX_BLOOM_BUFFER_SCALE_LOG2: usize = 3;
const MIN_BLOOM_BUFFER_SCALE: usize = 1 << MIN_BLOOM_BUFFER_SCALE_LOG2;
const MAX_BLOOM_BUFFER_SCALE: usize = 1 << MAX_BLOOM_BUFFER_SCALE_LOG2;

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

fn debug_checked_fetch<T: Copy>(data: &[T], index: usize) -> T
{
	#[cfg(debug_assertions)]
	{
		data[index]
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		*data.get_unchecked(index)
	}
}

fn debug_checked_store<T: Copy>(data: &mut [T], index: usize, value: T)
{
	#[cfg(debug_assertions)]
	{
		data[index] = value
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		*data.get_unchecked_mut(index) = value
	}
}
