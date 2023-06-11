use super::map_materials_processor_structs::*;
use crate::common::material::*;

pub struct GenerativeTextureEffectWater
{
	water_effect: WaterEffect,
}

impl GenerativeTextureEffectWater
{
	pub fn new(water_effect: WaterEffect) -> Self
	{
		Self { water_effect }
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
		_out_texture_data: &mut GenerativeTextureData,
		_texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		_current_time_s: f32,
	)
	{
		// TODO
	}
}
