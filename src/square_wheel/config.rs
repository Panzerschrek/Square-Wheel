use std::{
	fs,
	path::Path,
	sync::{Arc, Mutex},
};

pub type ConfigSharedPtr = Arc<Mutex<serde_json::Value>>;

pub fn load(file_path: &Path) -> Option<serde_json::Value>
{
	if let Ok(file_contents) = fs::read_to_string(file_path)
	{
		return serde_json::from_str(&file_contents).ok();
	}

	None
}

pub fn save(config: &serde_json::Value, file_path: &Path)
{
	if let Ok(s) = serde_json::to_string_pretty(config)
	{
		let _ignore = fs::write(file_path, &s);
	}
}

pub fn make_shared(config: serde_json::Value) -> ConfigSharedPtr
{
	Arc::new(Mutex::new(config))
}
