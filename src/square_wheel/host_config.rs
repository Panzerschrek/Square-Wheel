use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HostConfig
{
	#[serde(default = "max_fps_default")]
	pub max_fps: f32,

	// Use "f32" because of problems with "JSON" serialization.
	#[serde(default)]
	pub num_threads: f32,

	#[serde(default)]
	pub show_debug_stats: bool,

	#[serde(default)]
	pub fullscreen_mode: f32,

	#[serde(default)]
	pub maps_path: String,
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
