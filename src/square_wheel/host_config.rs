use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HostConfig
{
	#[serde(default = "max_fps_default")]
	pub max_fps: f32,
}

impl HostConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.borrow()["host"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.borrow_mut()["host"] = json;
		}
	}
}

fn max_fps_default() -> f32
{
	120.0
}
