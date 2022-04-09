pub fn load(file_path: &std::path::Path) -> Option<serde_json::Value>
{
	if let Ok(file_contents) = std::fs::read_to_string(file_path)
	{
		return serde_json::from_str(&file_contents).ok();
	}

	None
}
