use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct RendererConfig
{
	#[serde(default)]
	pub clear_background: bool,

	#[serde(default)]
	pub show_stats: bool,

	#[serde(default)]
	pub debug_draw_depth: bool,

	#[serde(default)]
	pub invert_polygons_order: bool,

	// use alternaive (slow, recursive) search algorithm.
	#[serde(default)]
	pub recursive_visible_leafs_marking: bool,

	#[serde(default)]
	pub textures_mip_bias: f32,

	#[serde(default)]
	pub dynamic_mip_bias: bool,
}

impl RendererConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.borrow()["renderer"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.borrow_mut()["renderer"] = json;
		}
	}
}
