use super::{resources_manager::*, textures::*};
use crate::common::material::*;

pub type TextureShift = [i32; 2];

pub struct MapTextureData
{
	pub material: Material,
	pub texture: SharedResourcePtr<TextureWithMips>,
	// Non-empty if emissive texture exists.
	pub emissive_texture: Option<SharedResourcePtr<TextureLiteWithMips>>,
	// Used only for sparse texture update.
	pub animated_texture_order: u32,
	// Invalid index if has no framed animation.
	pub next_frame_texture_index: TextureIndex,
}

#[derive(Default)]
pub struct MapTextureDataMutable
{
	pub generative_texture_data: GenerativeTextureData,
	pub generative_effect: OptDynGenerativeTextureEffect,
	pub shift: TextureShift,
}

// Data created/modified by generative effect.
#[derive(Default)]
pub struct GenerativeTextureData
{
	// Non-empty for textures with animations.
	pub texture_modified: TextureWithMips,
	// Exists only for emissive textures with animations.
	pub emissive_texture_modified: TextureLiteWithMips,
}

pub type OptDynGenerativeTextureEffect = Option<Box<dyn GenerativeTextureEffect + Send + Sync>>;

pub trait GenerativeTextureEffect
{
	fn update(
		&mut self,
		texture_data_mutable: &mut GenerativeTextureData,
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
