use super::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ResourcesManagerConfig
{
	#[serde(default)]
	pub maps_path: String,

	#[serde(default)]
	pub materials_path: String,

	#[serde(default)]
	pub models_path: String,

	#[serde(default)]
	pub textures_path: String,
}

impl ResourcesManagerConfig
{
	pub fn from_app_config(app_config: &config::ConfigSharedPtr) -> Self
	{
		serde_json::from_value(app_config.lock().unwrap()["resources"].clone()).unwrap_or_default()
	}

	pub fn update_app_config(&self, app_config: &config::ConfigSharedPtr)
	{
		if let Ok(json) = serde_json::to_value(self)
		{
			app_config.lock().unwrap()["resources"] = json;
		}
	}
}
