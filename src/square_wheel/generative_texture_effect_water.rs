use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, material::*};

pub struct GenerativeTextureEffectWater
{
	water_effect: WaterEffect,
	wave_field_a: Vec<WaveFieldElement>,
	wave_field_b: Vec<WaveFieldElement>,
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
			wave_field_a: vec![0.0; size],
			wave_field_b: vec![0.0; size],
			frame: 0,
		}
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
		_texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		_current_time_s: f32,
	)
	{
		// TODO - use fixed frequency.
		self.frame += 1;

		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];

		// TODO - setup update frequency.
		let fixed_time = (self.frame as f32) / 60.0;

		let (dst_field, src_field) = if self.frame % 2 == 0
		{
			(&mut self.wave_field_b, &mut self.wave_field_a)
		}
		else
		{
			(&mut self.wave_field_a, &mut self.wave_field_b)
		};

		// Add test emitter.
		let spot_value = (fixed_time * 24.0).sin() * 2.0;
		let spot_coord = (size[0] / 2 + size[1] / 2 * size[0]) as usize;
		src_field[spot_coord] = spot_value;

		// Perfrom wave field calculation.
		update_wave_field(size, dst_field, src_field);

		// Allocate texture.
		out_texture_data.texture[0].size = size;
		out_texture_data.texture[0]
			.pixels
			.resize((size[0] * size[1]) as usize, TextureElement::default());

		// Generate texture based on wave field.
		// TODO - perform more complex texture generation, based on wave field.
		for (dst_texel, wave_value) in out_texture_data.texture[0].pixels.iter_mut().zip(dst_field.iter())
		{
			let v = unsafe { (wave_value * 255.0).max(0.0).min(255.0).to_int_unchecked::<u8>() };
			dst_texel.diffuse = Color32::from_rgb(v, v, v);
		}

		// TODO - generate mips. Now just fill with stub.
		for mip in 1 .. NUM_MIPS
		{
			if out_texture_data.texture[mip].pixels.is_empty()
			{
				out_texture_data.texture[mip] =
					super::textures::make_texture(crate::common::image::make_stub(), None, 0.0, None, false);
			}
		}
	}
}

// TODO - try to use less memory (16 bit or even 8 bit).
type WaveFieldElement = f32;

fn update_wave_field(size: [u32; 2], dst: &mut [WaveFieldElement], src: &[WaveFieldElement])
{
	// TODO - move into config.
	let attenuation = 0.992;

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
