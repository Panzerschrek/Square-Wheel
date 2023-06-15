use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, material_fire::*};

pub struct GenerativeTextureEffectFire
{
	// Corrected config value.
	update_frequency: f32,
	fire_effect: FireEffect,
	heat_map: Vec<HeatMapElemement>,
	palette: Palette,
}

impl GenerativeTextureEffectFire
{
	pub fn new(fire_effect: FireEffect) -> Self
	{
		if fire_effect.resolution_log2[0] < 2 || fire_effect.resolution_log2[1] < 2
		{
			panic!("Fire texture must have size at least 4x4!");
		}

		let area = 1 << (fire_effect.resolution_log2[0] + fire_effect.resolution_log2[1]);
		if area >= (1 << 22)
		{
			panic!("Fire texture is too big!");
		}

		Self {
			update_frequency: fire_effect.update_frequency.max(1.0).min(200.0),
			fire_effect,
			heat_map: vec![0; area],
			palette: [Color32::black(); 256],
		}
	}
}

impl GenerativeTextureEffect for GenerativeTextureEffectFire
{
	fn get_estimated_texel_count(&self, _texture_data: &MapTextureData, _all_textures_data: &[MapTextureData]) -> u32
	{
		// TODO
		0
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
		let size = [
			1 << self.fire_effect.resolution_log2[0],
			1 << self.fire_effect.resolution_log2[1],
		];

		self.heat_map[(size[0] / 2 + (size[1] - 1) * size[0]) as usize] = 255;

		update_heat_map(size, &mut self.heat_map);

		if out_texture_data.emissive_texture[0].pixels.is_empty()
		{
			// First update - generate palette.
			// TODO - use emissive image for it.
			for i in 0 .. 256
			{
				self.palette[i] = Color32::from_rgb(i as u8, i as u8, i as u8);
			}
		}

		generate_emissive_texture_based_on_heat_map(
			size,
			&self.heat_map,
			&self.palette,
			&mut out_texture_data.emissive_texture[0],
		);

		for mip in 1 .. NUM_MIPS
		{
			if out_texture_data.texture[mip].pixels.is_empty()
			{
				out_texture_data.emissive_texture[mip] = crate::common::image::make_stub();
			}
		}
	}
}

type HeatMapElemement = u8;

fn update_heat_map(size: [u32; 2], heat_map: &mut [HeatMapElemement])
{
	debug_assert!(size[0] >= 4);
	debug_assert!(size[1] >= 4);
	debug_assert!(heat_map.len() == (size[0] * size[1]) as usize);

	// TODO - make it configurable.
	let attenuation = 240;

	let mut update_func = |offset, offset_y_plus_x_minus, offset_y_plus, offset_y_plus_x_plus, offset_y_plus_plus| unsafe {
		let sum = (debug_only_checked_fetch(heat_map, offset_y_plus_x_minus as usize) as u32) +
			(debug_only_checked_fetch(heat_map, offset_y_plus as usize) as u32) +
			(debug_only_checked_fetch(heat_map, offset_y_plus_x_plus as usize) as u32) +
			(debug_only_checked_fetch(heat_map, offset_y_plus_plus as usize) as u32);
		debug_only_checked_write(heat_map, offset as usize, ((sum * attenuation) >> 10) as u8);
	};

	for y in 0 .. size[1] - 2
	{
		let line_start = y * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - left border.
		update_func(
			line_start,
			line_end_minus_one + size[0],
			line_start + size[0],
			line_start + 1 + size[0],
			line_start + size[0] * 2,
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1 + size[0], x + size[0], x + 1 + size[0], x + size[0] * 2);
		}

		// Special case - right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1 + size[0],
			line_end_minus_one + size[0],
			line_start + size[0],
			line_end_minus_one + size[0] * 2,
		);
	}

	{
		// Special case - line before last.
		// Clamp y + 2 to y + 1.
		let line_start = (size[1] - 2) * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - left border.
		update_func(
			line_start,
			line_end_minus_one + size[0],
			line_start + size[0],
			line_start + 1 + size[0],
			line_start + size[0],
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1 + size[0], x + size[0], x + 1 + size[0], x + size[0]);
		}

		// Special case - right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1 + size[0],
			line_end_minus_one + size[0],
			line_start + size[0],
			line_end_minus_one + size[0],
		);
	}

	// TODO - handle last line.
}

type Palette = [Color32; 256];

fn generate_emissive_texture_based_on_heat_map(
	size: [u32; 2],
	heat_map: &[HeatMapElemement],
	palette: &Palette,
	texture: &mut TextureLite,
)
{
	texture.size = size;
	texture.pixels.resize((size[0] * size[1]) as usize, Color32::black());

	for (dst_texel, &heat_value) in texture.pixels.iter_mut().zip(heat_map)
	{
		// TODO - use unchecked palette fetch.
		*dst_texel = palette[heat_value as usize]
	}
}
