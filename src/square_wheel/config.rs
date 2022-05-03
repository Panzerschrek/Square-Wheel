use std::{cell::RefCell, rc::Rc};

pub type ConfigSharedPtr = Rc<RefCell<serde_json::Value>>;

pub fn load(file_path: &std::path::Path) -> Option<serde_json::Value>
{
	if let Ok(file_contents) = std::fs::read_to_string(file_path)
	{
		return serde_json::from_str(&file_contents).ok();
	}

	None
}

pub fn make_shared(config: serde_json::Value) -> ConfigSharedPtr
{
	Rc::new(RefCell::new(config))
}
