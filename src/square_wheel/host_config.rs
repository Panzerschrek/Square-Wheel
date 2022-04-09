use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HostConfig
{
	#[serde(default = "map_fps_default")]
	pub max_fps: f32,
}

impl HostConfig
{
	pub fn from_app_config(app_config: &serde_json::Value) -> Self
	{
		serde_json::from_value(app_config["host"].clone()).unwrap_or_default()
	}
}

fn map_fps_default() -> f32
{
	120.0
}
