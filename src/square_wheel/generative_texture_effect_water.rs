use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, fixed_math::*, material_water::*, math_types::*};
use rand::{Rng, RngCore, SeedableRng};

pub struct GenerativeTextureEffectWater
{
	// Corrected config value.
	update_frequency: f32,
	water_effect: WaterEffect,
	wave_field: Vec<WaveFieldElement>,
	wave_field_old: Vec<WaveFieldElement>,
	rand_engine: RandEngine,
	update_step: u32,
	prev_update_time_s: f32,
}

impl GenerativeTextureEffectWater
{
	pub fn new(water_effect: WaterEffect) -> Self
	{
		// TODO - use half-size?
		let size = 1 << (water_effect.resolution_log2[0] + water_effect.resolution_log2[1]);
		if size == 0
		{
			panic!("Invalid size");
		}

		Self {
			update_frequency: water_effect.update_frequency.max(1.0).min(200.0),
			water_effect,
			wave_field: vec![0.0; size],
			wave_field_old: vec![0.0; size],
			// Initialize random engine generator with good, but deterministic value.
			rand_engine: RandEngine::seed_from_u64(0b1001100000111010100101010101010111000111010110100101111001010101),
			update_step: 0,
			prev_update_time_s: 0.0,
		}
	}

	fn step_update_wave_field(&mut self)
	{
		self.update_step += 1;

		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];
		let size_mask = [size[0] - 1, size[1] - 1];
		let v_shift = self.water_effect.resolution_log2[1];

		let time_s = (self.update_step as f32) / self.update_frequency;

		// Add wave sources.
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
					let field_value = (time_s * frequency * std::f32::consts::TAU + phase).sin() * amplitude + offset;
					let spot_coord = ((center[0] & size_mask[0]) + ((center[1] & size_mask[1]) << v_shift)) as usize;
					self.wave_field[spot_coord] = field_value;
				},
				WaveSource::WavyLine {
					points,
					frequency,
					phase,
					amplitude,
					offset,
				} =>
				{
					let field_value = (time_s * frequency * std::f32::consts::TAU + phase).sin() * amplitude + offset;

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
							self.wave_field
								[((x as u32 & size_mask[0]) + ((y as u32 & size_mask[1]) << v_shift)) as usize] = field_value;
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
							self.wave_field
								[((x as u32 & size_mask[0]) + ((y as u32 & size_mask[1]) << v_shift)) as usize] = field_value;
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
					// TODO - perform wave field modification exactly once per period.
					let sin_value = (time_s * frequency * std::f32::consts::TAU + phase).sin();
					if sin_value >= 0.9
					{
						let spot_coord =
							((center[0] & size_mask[0]) + ((center[1] & size_mask[1]) << v_shift)) as usize;
						let field_value = -*amplitude; // Minus because this is a droplet.
						self.wave_field[spot_coord] = field_value;
						self.wave_field_old[spot_coord] = -field_value;
					}
				},
				WaveSource::Rain {
					center,
					radius,
					amplitude,
				} =>
				{
					// TODO - setup frequency.
					if self.rand_engine.next_u32() % 16 == 0
					{
						let (x, y) = if *radius > 0.0
						{
							let dist: f32 = self.rand_engine.gen_range(0.0 ..= *radius);
							let angle: f32 = self.rand_engine.gen_range(0.0 ..= std::f32::consts::TAU);
							let (dx, dy) = (angle.cos() * dist, angle.sin() * dist);

							(
								(((center[0] as i32) + (dx as i32)) as u32) & size_mask[0],
								(((center[1] as i32) + (dy as i32)) as u32) & size_mask[1],
							)
						}
						else
						{
							(
								self.rand_engine.next_u32() & size_mask[0],
								self.rand_engine.next_u32() & size_mask[1],
							)
						};

						let spot_coord = (x + (y << v_shift)) as usize;
						let field_value = -*amplitude; // Minus because this is a droplet.

						// TODO - such random droplets looks ugly. Maybe modify wave field more smoothly?
						self.wave_field[spot_coord] = field_value;
						self.wave_field_old[spot_coord] = -field_value;
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

type RandEngine = rand::rngs::SmallRng;

impl GenerativeTextureEffect for GenerativeTextureEffectWater
{
	fn get_estimated_texel_count(&self, _texture_data: &MapTextureData, _all_textures_data: &[MapTextureData]) -> u32
	{
		0 // TODO
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

		// Generate texture with normals based on wave field.
		// TODO - support other kinds of textures.
		let last_mip_texel = &texture_data.texture[MAX_MIP].pixels[0];
		make_texture_with_normals_of_wave_field(
			[
				1 << self.water_effect.resolution_log2[0],
				1 << self.water_effect.resolution_log2[1],
			],
			&self.wave_field,
			&mut out_texture_data.texture[0],
			last_mip_texel.diffuse,
			last_mip_texel.packed_normal_roughness.unpack_roughness(),
		);

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

fn update_wave_field(size: [u32; 2], attenuation: f32, dst: &mut [WaveFieldElement], src: &[WaveFieldElement])
{
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

fn make_texture_with_normals_of_wave_field(
	size: [u32; 2],
	wave_field: &[WaveFieldElement],
	out_texture: &mut Texture,
	base_color: Color32,
	base_roughness: f32,
)
{
	out_texture.has_non_one_roughness = base_roughness < 1.0;
	out_texture.has_normal_map = true;
	out_texture.size = size;
	out_texture
		.pixels
		.resize((size[0] * size[1]) as usize, TextureElement::default());

	let mut gen_func = |offset, offset_x_minus_one, offset_x_plus_one, offset_y_minus_one, offset_y_plus_one| unsafe {
		let val_x_minus = debug_only_checked_fetch(wave_field, offset_x_minus_one as usize);
		let val_x_plus = debug_only_checked_fetch(wave_field, offset_x_plus_one as usize);
		let val_y_minus = debug_only_checked_fetch(wave_field, offset_y_minus_one as usize);
		let val_y_plus = debug_only_checked_fetch(wave_field, offset_y_plus_one as usize);

		let dx = val_x_plus - val_x_minus;
		let dy = val_y_plus - val_y_minus;
		let normal = Vec3f::new(dx, dy, 1.0);

		// TODO - perform fast normalization.
		let result_texel = TextureElement {
			diffuse: base_color,
			packed_normal_roughness: PackedNormalRoughness::pack(&normal.normalize(), base_roughness),
		};
		debug_only_checked_write(&mut out_texture.pixels, offset as usize, result_texel);
	};

	// Special case - upper border.
	{
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
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			gen_func(x, x - 1, x + 1, x + y_minus_one_offset, x + size[0]);
		}

		// Special case - wrap around right border.
		gen_func(
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
		gen_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start - size[0],
			line_start + size[0],
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			gen_func(x, x - 1, x + 1, x - size[0], x + size[0]);
		}

		// Special case - wrap around right border.
		gen_func(
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
		gen_func(
			line_start,
			line_end_minus_one,
			line_start_plus_one,
			line_start - size[0],
			line_start - y_plus_one_offset,
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			gen_func(x, x - 1, x + 1, x - size[0], x - y_plus_one_offset);
		}

		// Special case - wrap around right border.
		gen_func(
			line_end_minus_one,
			line_end_minus_one - 1,
			line_start,
			line_end_minus_one - size[0],
			line_end_minus_one - y_plus_one_offset,
		);
	}
}
