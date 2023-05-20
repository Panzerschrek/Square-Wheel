use super::config;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub struct RendererConfig
{
	#[serde(default)]
	pub clear_background: bool,

	#[serde(default)]
	pub debug_draw_depth: bool,

	#[serde(default)]
	pub invert_polygons_order: bool,

	#[serde(default)]
	pub textures_mip_bias: f32,

	#[serde(default)]
	pub dynamic_mip_bias: bool,

	#[serde(default)]
	// In range [-1; 1]
	pub shadows_quality: f32,

	#[serde(default = "default_portals_depth")]
	pub portals_depth: u32,

	#[serde(default = "default_true")]
	pub use_directional_lightmaps: bool,
}

impl RendererConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.lock().unwrap()["renderer"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.lock().unwrap()["renderer"] = json;
		}
	}
}

fn default_true() -> bool
{
	true
}

fn default_portals_depth() -> u32
{
	2
}
