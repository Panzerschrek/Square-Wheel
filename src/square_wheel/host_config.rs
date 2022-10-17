use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HostConfig
{
	#[serde(default = "max_fps_default")]
	pub max_fps: f32,

	#[serde(default)]
	pub num_threads: u32,

	#[serde(default)]
	pub show_debug_stats: bool,

	#[serde(default = "default_true")]
	pub parallel_swap_buffers: bool,

	#[serde(default)]
	pub fullscreen_mode: u32,

	#[serde(default)]
	pub frame_scale: u32,

	#[serde(default)]
	pub frame_resize_interpolate: bool,
}

impl HostConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.lock().unwrap()["host"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.lock().unwrap()["host"] = json;
		}
	}
}

fn max_fps_default() -> f32
{
	120.0
}

fn default_true() -> bool
{
	true
}
