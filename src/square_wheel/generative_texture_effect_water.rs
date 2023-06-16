use super::{fast_math::*, generative_texture_effects_common::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, fixed_math::*, image::*, material_water::*, math_types::*};

pub struct GenerativeTextureEffectWater
{
	// Corrected config value.
	update_frequency: f32,
	water_effect: WaterEffect,
	wave_field: Vec<WaveFieldElement>,
	wave_field_old: Vec<WaveFieldElement>,
	// Used for setting color of result texture.
	color_image: Image,
	// Use random for some wave sources.
	rand_engine: RandEngine,
	update_step: u32,
	prev_update_time_s: f32,
}

impl GenerativeTextureEffectWater
{
	pub fn new(water_effect: WaterEffect) -> Self
	{
		if water_effect.resolution_log2[0] < 2 || water_effect.resolution_log2[1] < 2
		{
			panic!("Water texture must have size at least 4x4!");
		}

		let area = 1 << (water_effect.resolution_log2[0] + water_effect.resolution_log2[1]);
		if area >= (1 << 22)
		{
			panic!("Water texture is too big!");
		}

		let resolution_log2 = water_effect.resolution_log2;

		let mut result = Self {
			update_frequency: water_effect.update_frequency.max(1.0).min(200.0),
			water_effect,
			wave_field: vec![0.0; area],
			wave_field_old: vec![0.0; area],
			color_image: Image::default(),
			rand_engine: create_rand_engine(),
			update_step: 0,
			prev_update_time_s: 0.0,
		};

		// Perform several steps of wave field update in order to reach some sort of dynamic equilibrium.
		let diagonal_len = (((1 << (resolution_log2[0] * 2)) + (1 << (resolution_log2[1] * 2))) as f32).sqrt();
		let num_steps = (diagonal_len as i32).min(512);
		for _i in 0 .. num_steps
		{
			result.step_update_wave_field();
		}

		result
	}

	fn step_update_wave_field(&mut self)
	{
		self.update_step += 1;

		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];
		let size_mask = [size[0] - 1, size[1] - 1];
		let v_shift = self.water_effect.resolution_log2[0];

		let time_s = (self.update_step as f32) / self.update_frequency;

		// Add wave sources.

		let wave_field = &mut self.wave_field;
		let wave_field_old = &mut self.wave_field_old;
		let mut add_point_value = |x, y, value: f32| {
			let address = ((x & size_mask[0]) + ((y & size_mask[1]) << v_shift)) as usize;
			// Adding (not setting) two values in antiphaze produces good result.
			// With such approach wave center doesn't look so pointy.
			wave_field[address] += value;
			wave_field_old[address] -= value;
		};

		for wave_source in &self.water_effect.wave_sources
		{
			match wave_source
			{
				WaveSource::WavySpot {
					center,
					frequency,
					phase,
					amplitude,
					offset,
				} =>
				{
					let field_value =
						(time_s * frequency * std::f32::consts::TAU + phase).sin() * amplitude * frequency + offset;
					add_point_value(center[0], center[1], field_value);
				},
				WaveSource::WavyLine {
					points,
					frequency,
					phase,
					amplitude,
					offset,
				} =>
				{
					let field_value =
						(time_s * frequency * std::f32::consts::TAU + phase).sin() * amplitude * frequency + offset;

					// Perform simple line reasterization (with integer coords of points).
					let mut points = *points;
					let abs_dx = ((points[0][0] as i32) - (points[1][0] as i32)).abs();
					let abs_dy = ((points[0][1] as i32) - (points[1][1] as i32)).abs();
					if abs_dx >= abs_dy
					{
						if points[0][0] > points[1][0]
						{
							points.swap(0, 1);
						}
						let y_step = int_to_fixed16((points[1][1] as i32) - (points[0][1] as i32)) / abs_dx;
						let mut y_fract = int_to_fixed16(points[0][1] as i32);
						for x_offset in 0 ..= abs_dx as i32
						{
							let x = (points[0][0] as i32) + x_offset;
							let y = fixed16_round_to_int(y_fract);
							y_fract += y_step;
							add_point_value(x as u32, y as u32, field_value);
						}
					}
					else
					{
						if points[0][1] > points[1][1]
						{
							points.swap(0, 1);
						}
						let x_step = int_to_fixed16((points[1][0] as i32) - (points[0][0] as i32)) / abs_dy;
						let mut x_fract = int_to_fixed16(points[0][0] as i32);
						for y_offset in 1 ..= abs_dy as i32
						{
							let y = (points[0][1] as i32) + y_offset;
							let x = fixed16_round_to_int(x_fract);
							x_fract += x_step;
							add_point_value(x as u32, y as u32, field_value);
						}
					}
				},
				WaveSource::PeriodicDroplet {
					center,
					frequency,
					phase,
					amplitude,
				} =>
				{
					let relative_frequency = frequency / self.update_frequency;
					let begin = (self.update_step as f32 * relative_frequency + phase) as i32;
					let end = ((self.update_step + 1) as f32 * relative_frequency + phase) as i32;
					for _i in begin .. end
					{
						add_point_value(center[0], center[1], *amplitude);
					}
				},
				WaveSource::Rain {
					center,
					frequency,
					radius,
					amplitude,
				} =>
				{
					if self.rand_engine.gen_range(0.0 ..= 1.0) <= frequency / self.update_frequency
					{
						let (x, y) = if *radius > 0.0
						{
							let dist: f32 = self.rand_engine.gen_range(0.0 ..= *radius);
							let angle: f32 = self.rand_engine.gen_range(0.0 ..= std::f32::consts::TAU);
							let (dx, dy) = (angle.cos() * dist, angle.sin() * dist);

							(
								((center[0] as i32) + (dx as i32)) as u32,
								((center[1] as i32) + (dy as i32)) as u32,
							)
						}
						else
						{
							(self.rand_engine.next_u32(), self.rand_engine.next_u32())
						};

						add_point_value(x, y, *amplitude);
					}
				},
			}
		}

		let attenuation = 1.0 - 1.0 / self.water_effect.fluidity.max(10.0).min(1000000.0);
		update_wave_field(size, attenuation, &mut self.wave_field_old, &self.wave_field);

		// Old field is now new field.
		// Swapping two vectors is cheap.
		std::mem::swap(&mut self.wave_field, &mut self.wave_field_old);
	}
}

impl GenerativeTextureEffect for GenerativeTextureEffectWater
{
	fn get_estimated_texel_count(&self, _texture_data: &MapTextureData, _all_textures_data: &[MapTextureData]) -> u32
	{
		let base_size = 1 << (self.water_effect.resolution_log2[0] + self.water_effect.resolution_log2[1]);
		// Count result texture and wave field buffers.
		base_size * 2
	}

	fn update(
		&mut self,
		out_texture_data: &mut GenerativeTextureData,
		texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		current_time_s: f32,
	)
	{
		// Wave field update works in fixed step.
		// Perform 0 or more steps, but avoid performing too much steps (large field size, debug build).
		let num_update_steps = ((current_time_s * self.update_frequency) as i32 -
			(self.prev_update_time_s * self.update_frequency) as i32)
			.max(0)
			.min(10);

		self.prev_update_time_s = current_time_s;

		if num_update_steps == 0
		{
			return;
		}

		for _i in 0 .. num_update_steps
		{
			self.step_update_wave_field();
		}

		// Generate texture (mip0).
		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];

		if self.water_effect.color_texture_apply_mode != ColorTextureApplyMode::SingleColor &&
			self.color_image.pixels.is_empty()
		{
			// If we apply source texture - extract image itself from it.
			// It is cache-frendly to work with 32-bit image rather than with texture containing both color and normal data.
			self.color_image = extract_color_image_from_texture(&texture_data.texture[0]);
			if self.color_image.size != size
			{
				self.color_image = resize_image(&self.color_image, size);
			}
		}

		let last_mip_texel_color = texture_data.texture[MAX_MIP].pixels[0].diffuse;
		make_wavy_texture(
			self.water_effect.color_texture_apply_mode,
			size,
			&self.wave_field,
			&mut out_texture_data.texture[0],
			last_mip_texel_color,
			texture_data.material.roughness.max(MIN_VALID_ROUGHNESS).min(1.0),
			&self.color_image.pixels,
		);

		out_texture_data.texture[0].is_metal = texture_data.material.is_metal;

		// Generate mips.
		// TODO - maybe reduce frequency of mips update?
		for i in 1 .. NUM_MIPS
		{
			let (s0, s1) = out_texture_data.texture.split_at_mut(i);
			build_texture_mip(&mut s1[0], &s0[i - 1]);
		}
	}
}

// TODO - try to use less memory (16 bit or even 8 bit).
type WaveFieldElement = f32;

// This function performs one step of wave field simulation.
fn update_wave_field(size: [u32; 2], attenuation: f32, dst: &mut [WaveFieldElement], src: &[WaveFieldElement])
{
	debug_assert!(size[0] >= 4);
	debug_assert!(size[1] >= 4);
	debug_assert!(dst.len() == src.len());
	debug_assert!(dst.len() == (size[0] * size[1]) as usize);

	let mut update_func = |offset, offset_x_minus_one, offset_x_plus_one, offset_y_minus_one, offset_y_plus_one| unsafe {
		let sum = debug_only_checked_fetch(src, offset_x_minus_one as usize) +
			debug_only_checked_fetch(src, offset_x_plus_one as usize) +
			debug_only_checked_fetch(src, offset_y_minus_one as usize) +
			debug_only_checked_fetch(src, offset_y_plus_one as usize);
		let val = debug_only_checked_fetch(dst, offset as usize);
		debug_only_checked_write(dst, offset as usize, (sum * 0.5 - val) * attenuation);
	};

	// Special case - upper border.
	{
		let y_minus_one_offset = (size[1] - 1) * size[0];

		let line_start = 0;
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - wrap around left border.
		update_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start + y_minus_one_offset,
			line_start + size[0],
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1, x + 1, x + y_minus_one_offset, x + size[0]);
		}

		// Special case - wrap around right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one + y_minus_one_offset,
			line_end_minus_one + size[0],
		);
	}

	for y in 1 .. size[1] - 1
	{
		let line_start = y * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - wrap around left border.
		update_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start - size[0],
			line_start + size[0],
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1, x + 1, x - size[0], x + size[0]);
		}

		// Special case - wrap around right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one - size[0],
			line_end_minus_one + size[0],
		);
	}

	// Special case - lower border.
	{
		let y_plus_one_offset = (size[1] - 1) * size[0];

		let line_start = (size[1] - 1) * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - wrap around left border.
		update_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start - size[0],
			line_start - y_plus_one_offset,
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1, x + 1, x - size[0], x - y_plus_one_offset);
		}

		// Special case - wrap around right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one - size[0],
			line_end_minus_one - y_plus_one_offset,
		);
	}
}

// Create texture with normal map calculated based on wave field, (possible) input color texture and (possible) with deformation of color texture.
fn make_wavy_texture(
	color_texture_apply_mode: ColorTextureApplyMode,
	size: [u32; 2],
	wave_field: &[WaveFieldElement],
	out_texture: &mut Texture,
	base_color: Color32,
	roughness: f32,
	color_image_pixels: &[Color32],
)
{
	out_texture.has_non_one_roughness = roughness < 1.0;
	out_texture.has_normal_map = true;
	out_texture.size = size;

	if out_texture.pixels.is_empty()
	{
		out_texture
			.pixels
			.resize((size[0] * size[1]) as usize, TextureElement::default());

		// If deformation is not required - fill color once and later just preserve it.
		if color_texture_apply_mode == ColorTextureApplyMode::SingleColor
		{
			for texel in &mut out_texture.pixels
			{
				texel.diffuse = base_color;
			}
		}
		if color_texture_apply_mode == ColorTextureApplyMode::SourceTexture
		{
			debug_assert!(color_image_pixels.len() == out_texture.pixels.len());
			for (texel, src_texel) in out_texture.pixels.iter_mut().zip(color_image_pixels.iter())
			{
				texel.diffuse = *src_texel;
			}
		}
	}

	match color_texture_apply_mode
	{
		ColorTextureApplyMode::SingleColor | ColorTextureApplyMode::SourceTexture =>
		{
			make_wavy_texture_impl::<WAVY_TEXTURE_COLOR_MODE_NONE>(
				size,
				wave_field,
				out_texture,
				roughness,
				color_image_pixels,
			)
		},
		ColorTextureApplyMode::SourceTextureNormalDeformed =>
		{
			make_wavy_texture_impl::<WAVY_TEXTURE_COLOR_MODE_SOURCE_TEXTURE_NORMAL_DEFORMED>(
				size,
				wave_field,
				out_texture,
				roughness,
				color_image_pixels,
			)
		},
		ColorTextureApplyMode::SourceTextureNormalDeformedX =>
		{
			make_wavy_texture_impl::<WAVY_TEXTURE_COLOR_MODE_SOURCE_TEXTURE_NORMAL_DEFORMED_X>(
				size,
				wave_field,
				out_texture,
				roughness,
				color_image_pixels,
			)
		},
	}
}

const WAVY_TEXTURE_COLOR_MODE_NONE: u32 = 0;
const WAVY_TEXTURE_COLOR_MODE_SOURCE_TEXTURE_NORMAL_DEFORMED: u32 = 1;
const WAVY_TEXTURE_COLOR_MODE_SOURCE_TEXTURE_NORMAL_DEFORMED_X: u32 = 2;

fn make_wavy_texture_impl<const COLOR_MODE: u32>(
	size: [u32; 2],
	wave_field: &[WaveFieldElement],
	out_texture: &mut Texture,
	roughness: f32,
	color_image_pixels: &[Color32],
)
{
	debug_assert!(size[0] >= 4);
	debug_assert!(size[1] >= 4);
	debug_assert!((size[0] & (size[0] - 1)) == 0);
	debug_assert!((size[1] & (size[1] - 1)) == 0);

	let size_mask = [size[0] - 1, size[1] - 1];

	let tc_deform_scale = 16.0; // TODO - read from config.

	let mut gen_func = |offset, offset_x_minus_one, offset_x_plus_one, offset_y_minus_one, offset_y_plus_one, x, y| unsafe {
		let val_x_minus = debug_only_checked_fetch(wave_field, offset_x_minus_one as usize);
		let val_x_plus = debug_only_checked_fetch(wave_field, offset_x_plus_one as usize);
		let val_y_minus = debug_only_checked_fetch(wave_field, offset_y_minus_one as usize);
		let val_y_plus = debug_only_checked_fetch(wave_field, offset_y_plus_one as usize);

		let dx = val_x_plus - val_x_minus;
		let dy = val_y_plus - val_y_minus;
		let normal = Vec3f::new(dx, dy, 1.0);
		// TODO - try to use fast inverse square root.
		let normal_normalized = normal.normalize();

		let out_texel = debug_only_checked_access_mut(&mut out_texture.pixels, offset as usize);
		out_texel.packed_normal_roughness = PackedNormalRoughness::pack(&normal_normalized, roughness);

		match COLOR_MODE
		{
			WAVY_TEXTURE_COLOR_MODE_NONE =>
			{
				// Preserve original color of dst texture.
				// Such approach allows us to avoid reading/writing color texture each time when only normal map is regenerated.
			},
			WAVY_TEXTURE_COLOR_MODE_SOURCE_TEXTURE_NORMAL_DEFORMED =>
			{
				let du = (normal_normalized.x * tc_deform_scale) as i32;
				let dv = (normal_normalized.y * tc_deform_scale) as i32;
				let u = (((x as i32) + du) as u32) & size_mask[0];
				let v = (((y as i32) + dv) as u32) & size_mask[1];
				out_texel.diffuse = debug_only_checked_fetch(color_image_pixels, (u + v * size[0]) as usize)
			},
			WAVY_TEXTURE_COLOR_MODE_SOURCE_TEXTURE_NORMAL_DEFORMED_X =>
			{
				// This is more cache friendly way to deform source texture.
				let du = (normal_normalized.x * tc_deform_scale) as i32;
				let u = (((x as i32) + du) as u32) & size_mask[0];
				out_texel.diffuse = debug_only_checked_fetch(color_image_pixels, (u + y * size[0]) as usize)
			},
			_ =>
			{},
		};
	};

	// Special case - upper border.
	{
		let y = 0;
		let y_minus_one_offset = (size[1] - 1) * size[0];

		let line_start = 0;
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - wrap around left border.
		gen_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start + y_minus_one_offset,
			line_start + size[0],
			0,
			y,
		);

		for offset in line_start_plus_one .. line_end_minus_one
		{
			let x = offset - line_start;
			gen_func(
				offset,
				offset - 1,
				offset + 1,
				offset + y_minus_one_offset,
				offset + size[0],
				x,
				y,
			);
		}

		// Special case - wrap around right border.
		gen_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one + y_minus_one_offset,
			line_end_minus_one + size[0],
			size[0] - 1,
			y,
		);
	}

	for y in 1 .. size[1] - 1
	{
		let line_start = y * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - wrap around left border.
		gen_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start - size[0],
			line_start + size[0],
			0,
			y,
		);

		for offset in line_start_plus_one .. line_end_minus_one
		{
			let x = offset - line_start;
			gen_func(offset, offset - 1, offset + 1, offset - size[0], offset + size[0], x, y);
		}

		// Special case - wrap around right border.
		gen_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one - size[0],
			line_end_minus_one + size[0],
			size[0] - 1,
			y,
		);
	}

	// Special case - lower border.
	{
		let y = size[1] - 1;
		let y_plus_one_offset = y * size[0];

		let line_start = (size[1] - 1) * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - wrap around left border.
		gen_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start - size[0],
			line_start - y_plus_one_offset,
			0,
			y,
		);

		for offset in line_start_plus_one .. line_end_minus_one
		{
			let x = offset - line_start;
			gen_func(
				offset,
				offset - 1,
				offset + 1,
				offset - size[0],
				offset - y_plus_one_offset,
				x,
				y,
			);
		}

		// Special case - wrap around right border.
		gen_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one - size[0],
			line_end_minus_one - y_plus_one_offset,
			size[0] - 1,
			y,
		);
	}
}

fn extract_color_image_from_texture(texture: &Texture) -> Image
{
	Image {
		size: texture.size,
		pixels: texture.pixels.iter().map(|t| t.diffuse).collect(),
	}
}
