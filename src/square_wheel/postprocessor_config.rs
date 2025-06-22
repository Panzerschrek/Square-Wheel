use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct PostprocessorConfig
{
	#[serde(default)]
	pub hdr_rendering: bool,

	#[serde(default)]
	pub use_multithreading: bool,

	#[serde(default = "default_one")]
	pub exposure_update_speed: f32,

	#[serde(default = "default_one")]
	pub base_brightness: f32,

	#[serde(default = "default_zero_level_brightness")]
	pub zero_level_brightness: f32,

	#[serde(default = "default_brightness_scale_power")]
	pub brightness_scale_power: f32,

	#[serde(default = "default_min_exposure")]
	pub min_exposure: f32,

	#[serde(default = "default_max_exposure")]
	pub max_exposure: f32,

	#[serde(default = "default_one")]
	pub bloom_sigma: f32,

	#[serde(default = "default_bloom_buffer_scale_log2")]
	pub bloom_buffer_scale_log2: u32,

	#[serde(default = "default_bloom_scale")]
	pub bloom_scale: f32,

	#[serde(default)]
	pub linear_bloom_filter: bool,
}

impl PostprocessorConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.read().unwrap()["postprocessor"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.write().unwrap()["postprocessor"] = json;
		}
	}
}

fn default_one() -> f32
{
	1.0
}

fn default_bloom_buffer_scale_log2() -> u32
{
	1
}

fn default_zero_level_brightness() -> f32
{
	1.0
}

fn default_brightness_scale_power() -> f32
{
	1.0 / 4.0
}

fn default_min_exposure() -> f32
{
	1.0 / 65536.0
}

fn default_max_exposure() -> f32
{
	128.0
}

fn default_bloom_scale() -> f32
{
	0.25
}
