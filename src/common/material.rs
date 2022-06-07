use serde::{Deserialize, Serialize};

pub type MaterialsMap = std::collections::HashMap<String, Material>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Material
{
	/// Diffuse texture path.
	pub diffuse: Option<String>,

	/// Normal map texture path.
	pub normal_map: Option<String>,

	/// 0 - no specular at all, material looks like diffuse.
	/// 0.1 - some specular is visible.
	/// 0.25 - specular is noticeable on most surfaces.
	/// 0.5 - slightly rough mirror.
	/// 1.0 - almost like a mirror.
	#[serde(default)]
	pub glossiness: f32,

	/// If non-empty - glossiness from this texture will be used instead of glossiness param.
	pub glossiness_map: Option<String>,

	/// If false - totally exclude from BSP build. Completely removes all polygons with such materials.
	#[serde(default = "default_true")]
	pub bsp: bool,

	/// If false - do not draw polygons with such material.
	#[serde(default = "default_true")]
	pub draw: bool,

	/// Polygons with this material blocks view.
	/// This flag is used during BSP leaf portals calculation.
	#[serde(default = "default_true")]
	pub blocks_view: bool,

	/// If true - polygon is affected by light and has lightmap.
	#[serde(default = "default_true")]
	pub light: bool,
}

impl Default for Material
{
	fn default() -> Self
	{
		Self {
			diffuse: None,
			normal_map: None,
			glossiness: 0.0,
			glossiness_map: None,
			bsp: true,
			draw: true,
			blocks_view: true,
			light: true,
		}
	}
}

pub fn load_materials(dir_path: &std::path::Path) -> MaterialsMap
{
	let mut result = MaterialsMap::new();
	load_materials_from_dir(dir_path, &mut result);
	result
}

fn load_materials_from_dir(dir_path: &std::path::Path, out_materials: &mut MaterialsMap)
{
	let dir_iterator = if let Ok(it) = std::fs::read_dir(dir_path)
	{
		it
	}
	else
	{
		println!("Failed to iterate over directory {:?}", dir_path);
		return;
	};

	// TODO - log entry errors?
	for entry_opt in dir_iterator
	{
		let entry = if let Ok(e) = entry_opt
		{
			e
		}
		else
		{
			break;
		};

		let file_type = if let Ok(t) = entry.file_type()
		{
			t
		}
		else
		{
			continue;
		};
		if file_type.is_dir()
		{
			load_materials_from_dir(&entry.path(), out_materials);
		}
		else if file_type.is_file()
		{
			load_materials_from_file(&entry.path(), out_materials);
		}
	}
}

fn load_materials_from_file(file_path: &std::path::Path, out_materials: &mut MaterialsMap)
{
	let file_contents = if let Ok(c) = std::fs::read_to_string(file_path)
	{
		c
	}
	else
	{
		println!("Failed to read material file {:?} ", file_path);
		return;
	};

	let materials_set_json = if let Ok(j) = serde_json::from_str::<serde_json::Value>(&file_contents)
	{
		j
	}
	else
	{
		println!("Failed to parse material json file {:?} ", file_path);
		return;
	};

	let root_object = if let Some(o) = materials_set_json.as_object()
	{
		o
	}
	else
	{
		println!("Unexpected json root type, expected object");
		return;
	};

	for (material_name, material_json) in root_object
	{
		if let Ok(material_parsed) = serde_json::from_value::<Material>(material_json.clone())
		{
			out_materials.insert(material_name.to_string(), material_parsed);
		}
		else
		{
			println!("Failed to parse material {}", material_name);
		}
	}
}

fn default_true() -> bool
{
	true
}
