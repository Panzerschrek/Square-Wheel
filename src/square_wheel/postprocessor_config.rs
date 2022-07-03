use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct PostprocessorConfig
{
	#[serde(default)]
	pub hdr_rendering: bool,

	#[serde(default)]
	pub use_multithreadig: bool,

	#[serde(default = "default_one")]
	pub exposure: f32,

	#[serde(default = "default_one")]
	pub bloom_sigma: f32,

	#[serde(default = "default_one")]
	pub bloom_buffer_scale_log2: f32,

	#[serde(default = "default_bloom_scale")]
	pub bloom_scale: f32,

	#[serde(default)]
	pub linear_bloom_filter: bool,
}

impl PostprocessorConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.lock().unwrap()["postprocessor"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.lock().unwrap()["postprocessor"] = json;
		}
	}
}

fn default_one() -> f32
{
	1.0
}

fn default_bloom_scale() -> f32
{
	0.25
}