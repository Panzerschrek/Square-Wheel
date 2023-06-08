use super::config;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub struct MapMaterialsProcessorConfig
{
	// Update period in frames. If one - update all animated textures each frame.
	#[serde(default = "animated_textures_update_period_default")]
	pub animated_textures_update_period: u32,

	// If non-zero - calculate update frequency based on this limit - update no more, than N texels of animated textures.
	// Count all layers of animated textures, count emissive layers as half-texels, do not count mips.
	#[serde(default)]
	pub animated_textures_update_texels_limit: u32,
}

impl MapMaterialsProcessorConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.lock().unwrap()["materials_processor"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.lock().unwrap()["materials_processor"] = json;
		}
	}
}
fn animated_textures_update_period_default() -> u32
{
	1
}
