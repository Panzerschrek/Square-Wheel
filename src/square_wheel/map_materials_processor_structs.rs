use super::{resources_manager::*, textures::*};
use crate::common::material::*;

pub struct MapTextureData
{
	pub material: Material,
	pub texture: SharedResourcePtr<TextureWithMips>,
	// Non-empty if emissive texture exists.
	pub emissive_texture: Option<SharedResourcePtr<TextureLiteWithMips>>,
}

// Data created/modified by generative effect.
#[derive(Default)]
pub struct GenerativeTextureData
{
	// Non-empty for textures with animations.
	pub texture: TextureWithMips,
	// Nono-empty for emissive textures with animations.
	pub emissive_texture: TextureLiteWithMips,
}

pub type OptDynGenerativeTextureEffect = Option<Box<dyn GenerativeTextureEffect + Send + Sync>>;

// Interface for textures, that are generated each frame.
pub trait GenerativeTextureEffect
{
	// This is used in order to calculate update frequency and show some statistics.
	fn get_estimated_texel_count(&self, texture_data: &MapTextureData, all_textures_data: &[MapTextureData]) -> u32;

	fn update(
		&mut self,
		out_texture_data: &mut GenerativeTextureData,
		texture_data: &MapTextureData,
		all_textures_data: &[MapTextureData],
		textures_mapping_table: &[TextureMappingElement],
		current_time_s: f32,
	);
}

#[derive(Default, Copy, Clone)]
pub struct TextureMappingElement
{
	pub index: TextureIndex,
	pub frame_change_time_point: f32,
}

// Use indeces instead of strings in order to access map textures.
pub type TextureIndex = u32;
