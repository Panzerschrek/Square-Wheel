use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct RendererConfig
{
	#[serde(default)]
	pub clear_background: bool,

	#[serde(default)]
	pub show_stats: bool,

	#[serde(default)]
	pub invert_polygons_order: bool,

	// use alternaive (slow, recursive) search algorithm.
	#[serde(default)]
	pub recursive_visible_leafs_marking: bool,

	#[serde(default)]
	pub textures_mip_bias: f32,
}

impl RendererConfig
{
	pub fn from_app_config(app_config: &serde_json::Value) -> Self
	{
		serde_json::from_value(app_config["renderer"].clone()).unwrap_or_default()
	}
}
