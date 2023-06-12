use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, fixed_math::*, material_water::*, math_types::*};

pub struct GenerativeTextureEffectWater
{
	water_effect: WaterEffect,
	wave_field: Vec<WaveFieldElement>,
	wave_field_old: Vec<WaveFieldElement>,
	frame: u32,
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
			water_effect,
			wave_field: vec![0.0; size],
			wave_field_old: vec![0.0; size],
			frame: 0,
		}
	}

	fn step_update_wave_field(&mut self)
	{
		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];
		let size_mask = [size[0] - 1, size[1] - 1];
		let v_shift = self.water_effect.resolution_log2[1];

		// TODO - setup update frequency.
		let fixed_time = (self.frame as f32) / 60.0;

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
					let field_value =
						(fixed_time * frequency * std::f32::consts::TAU + phase).sin() * amplitude + offset;
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
					let field_value =
						(fixed_time * frequency * std::f32::consts::TAU + phase).sin() * amplitude + offset;

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
					let sin_value = (fixed_time * frequency * std::f32::consts::TAU + phase).sin();
					if sin_value >= 0.9
					{
						let spot_coord =
							((center[0] & size_mask[0]) + ((center[1] & size_mask[1]) << v_shift)) as usize;
						self.wave_field[spot_coord] = -*amplitude; // Minus because this is a droplet.
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
		0 // TODO
	}

	fn update(
		&mut self,
		out_texture_data: &mut GenerativeTextureData,
		texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		_current_time_s: f32,
	)
	{
		// TODO - use fixed frequency.
		self.frame += 1;
		self.step_update_wave_field();

		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];

		// Generate texture with normals based on wave field.
		// TODO - support other kinds of textures.

		let last_mip_texel = &texture_data.texture[MAX_MIP].pixels[0];
		make_texture_with_normals_of_wave_field(
			size,
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
