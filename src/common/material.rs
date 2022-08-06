use serde::{Deserialize, Serialize};

pub type MaterialsMap = std::collections::HashMap<String, Material>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Material
{
	/// Diffuse texture path.
	pub diffuse: Option<String>,

	/// Normal map texture path.
	pub normal_map: Option<String>,

	/// 1.0 - no specular at all.
	/// 0.5 - specular is noticeable.
	/// 0.25 - strong specular.
	/// 0.001 - almost like a mirror.
	#[serde(default = "default_one")]
	pub roughness: f32,

	/// If non-empty - roughness from this texture will be used instead of roughness param.
	pub roughness_map: Option<String>,

	/// For glossy materials use metal-style specular, instead of dielectric-style specular.
	#[serde(default)]
	pub is_metal: bool,

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

	/// Polygons with this material cast shadows.
	/// This flag is used during light calculations.
	#[serde(default = "default_true")]
	pub shadow: bool,

	/// If true - polygon is affected by light and has lightmap.
	#[serde(default = "default_true")]
	pub light: bool,

	/// If true - polygon will be affected by decals.
	#[serde(default = "default_true")]
	pub decals: bool,

	/// RGB power of light for emissive materials.
	/// Used during lightmaps preparation.
	#[serde(default)]
	pub emissive_light: [f32; 3],

	/// If some - polygons with such materials are semitransparent.
	#[serde(default)]
	pub blending_mode: BlendingMode,

	/// If some - use texture turbulence effect.
	#[serde(default)]
	pub turb: Option<TurbParams>,

	/// If some - this is a skybox.
	#[serde(default)]
	pub skybox: Option<SkyboxParams>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum BlendingMode
{
	None,
	Average,
	Additive,
	AlphaTest,
	AlphaBlend,
}

impl Default for BlendingMode
{
	fn default() -> Self
	{
		BlendingMode::None
	}
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct TurbParams
{
	/// In pixels.
	pub amplitude: f32,

	/// In pixels.
	pub wave_length: f32,

	/// In seconds.
	pub frequency: f32,

	/// Pixels/s.
	#[serde(default)]
	pub scroll_speed: [f32; 2],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SkyboxParams
{
	/// Side images in order -X, +X, -Y, +Y, -Z, +Z. If empty - side is not drawn.
	pub side_images: [String; 6],

	// Side images are modulated by this value.
	#[serde(default = "default_one")]
	pub brightness: f32,
}

impl Default for Material
{
	fn default() -> Self
	{
		Self {
			diffuse: None,
			normal_map: None,
			roughness: 1.0,
			roughness_map: None,
			is_metal: false,
			bsp: true,
			draw: true,
			blocks_view: true,
			shadow: true,
			light: true,
			decals: true,
			emissive_light: [0.0, 0.0, 0.0],
			blending_mode: BlendingMode::None,
			turb: None,
			skybox: None,
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

fn default_one() -> f32
{
	1.0
}
