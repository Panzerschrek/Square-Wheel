use super::{map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, material::*};

pub struct GenerativeTextureEffectWater
{
	water_effect: WaterEffect,
	wave_field_a: Vec<WaveFieldElement>,
	wave_field_b: Vec<WaveFieldElement>,
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
		current_time_s: f32,
	)
	{
		// TODO - use fixed frequency.
		let size = [
			1 << self.water_effect.resolution_log2[0],
			1 << self.water_effect.resolution_log2[1],
		];

		// Add test emitter.
		self.wave_field_a[(size[0] / 2 + size[1] / 2 * size[0]) as usize] = (current_time_s * 8.0).sin() * 4.0;

		update_wave_field(size, &mut self.wave_field_b, &self.wave_field_a);
		update_wave_field(size, &mut self.wave_field_a, &self.wave_field_b);

		// TODO - generate texture itself

		out_texture_data.texture[0].size = size;
		out_texture_data.texture[0]
			.pixels
			.resize((size[0] * size[1]) as usize, TextureElement::default());

		// TODO - perform more complex texture generation, based on wave field.
		for (dst_texel, wave_value) in out_texture_data.texture[0]
			.pixels
			.iter_mut()
			.zip(self.wave_field_a.iter())
		{
			let v = (wave_value * 255.0).max(0.0).min(255.0) as u8;
			dst_texel.diffuse = Color32::from_rgb(v, v, v);
		}

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

type WaveFieldElement = f32;

fn update_wave_field(size: [u32; 2], dst: &mut [WaveFieldElement], src: &[WaveFieldElement])
{
	// TODO - handle corner case.
	// TODO - optimize this.

	// TODO - move into config.
	let attenuation = 0.995;

	for y in 1 .. size[1] - 1
	{
		for x in 1 .. size[0] - 1
		{
			let sum = src[((x - 1) + y * size[0]) as usize] +
				src[((x + 1) + y * size[0]) as usize] +
				src[(x + (y - 1) * size[0]) as usize] +
				src[(x + (y + 1) * size[0]) as usize];
			let val = &mut dst[(x + y * size[0]) as usize];
			*val = (sum * 0.5 - *val) * attenuation;
		}
	}
}
