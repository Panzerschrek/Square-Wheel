use super::{material_fire::*, material_function::*, material_water::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type MaterialsMap = HashMap<String, Material>;

// TODO - put rarely used fields into box.
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

	/// If true - map compiler will create duplicated polygon with inverted normal.
	/// Duplicated polygon will be completely independent, have separate lightmap.
	/// This affects only map compiler. Lightmaper or the engine ignore it.
	#[serde(default)]
	pub twosided: bool,

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

	/// Pixels/s.
	#[serde(default)]
	pub scroll_speed: [f32; 2],

	/// If non-empty - additional texture will be composed atop of surface, multiplied by specified light.
	/// Note that if this material uses layered animation and at least some of the layers include emissive layer,
	/// emissive layer of this materials must be present (with dummy emissive texture).
	/// This is required in order to set proper light value.
	#[serde(default)]
	pub emissive_layer: Option<EmissiveLayer>,

	/// If some - this is frame-animated material and displayed texture will be switched in specified time interval.
	/// Create loop of references in order to create looped animation.
	/// Frame animation presense doesn't affect map compiler and lightmaper - only selected material properties will be used.
	/// Only main image will be used as albedo for lightmapper.
	#[serde(default)]
	pub framed_animation: Option<AnimationFrame>,

	/// None for regular textures.
	#[serde(default)]
	pub special_effect: SpecialMaterialEffect,

	// Other fields of material.
	// May be used to store game-specific material properties.
	// Keep it last here.
	#[serde(flatten)]
	pub extra: HashMap<String, serde_json::Value>,
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

// Same constants, as above, but these constants may used in template params.
pub const BLENDING_MODE_NONE: usize = 0;
pub const BLENDING_MODE_AVERAGE: usize = 1;
pub const BLENDING_MODE_ADDITIVE: usize = 2;
pub const BLENDING_MODE_ALPHA_TEST: usize = 3;
pub const BLENDING_MODE_ALPHA_BLEND: usize = 4;

impl Default for BlendingMode
{
	fn default() -> Self
	{
		BlendingMode::None
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmissiveLayer
{
	/// Path to emissive image.
	pub image: String,
	/// Value, by which texture value is multiplied in order to build surface.
	pub light: [f32; 3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimationFrame
{
	/// Duration of this frame, in seconds.
	#[serde(default = "default_animation_frame_duration")]
	pub duration: f32,
	/// Name of next material. Must be valid material name.
	pub next_material_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SpecialMaterialEffect
{
	// Just display regular texture.
	None,

	/// Use texture turbulence effect.
	/// Note that texture size must be power of two.
	Turb(TurbParams),

	/// This is layered-animated material.
	/// Material texture will be generated each frame, based on provided layers and params.
	/// Material texture needs to be regenerated each frame and this may take significant amount of frame time.
	/// So, avoid using too many textures in map or too large textures with layered animations - prefer framed animations and/or scrolling.
	/// Layered animation presense doesn't affect map compiler and lightmaper - like framed animation.
	LayeredAnimation(LayeredAnimation),

	/// Perform water simulation (via wave field) and create texture based on it.
	/// Normal map is generated based on the wave field state.
	/// Normal map of material itself is not used.
	/// Roughness map is also not used, only roughness material property itself.
	/// Color image may be used and potentially deformed by the wave field.
	/// Color image will be resized to size of wave filed (if sizes doen't equal).
	Water(WaterEffect),

	/// Perform heat map simulation and generate emissive texture based on it.
	/// Regular texture (diffuse, normal map, roughness, etc.) is not affected.
	Fire(FireEffect),

	/// Draw skybox instead of regular texture.
	Skybox(SkyboxParams),
}

impl Default for SpecialMaterialEffect
{
	fn default() -> Self
	{
		Self::None
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayeredAnimation
{
	/// Size of result image is determined by size of first layer.
	/// So, it is possible to create large texture (with large first layer) with additional layers as smaller tileable textures.
	/// First layer should have None blending in order to avoid mixing with garbage color data.
	pub layers: Vec<LayeredAnimationLayer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayeredAnimationLayer
{
	/// Material, that will be used for this layer.
	/// blending mode of this material is used for blending of result animated texture.
	/// It's important to know, that blending doesn't work for normal and roughness data.
	/// Normal/roughness is written only with blending mode = none or alpha test. Modulation is not used.
	/// Note that only static material texture is used, layered animation of that material can't be used, because this can cause infinite recursion.
	pub material_name: String,

	/// If true - fetch not layer material itself, but its current framed animation frame.
	#[serde(default)]
	pub follow_framed_animation: bool,

	/// How to perform shift of texture, depending on time.
	#[serde(default)]
	pub tex_coord_shift: [SingleArgumentFunction; 2],

	/// If some - modulate texture color using this time-dependent function.
	/// Value is clamped to 0.
	/// Values greater than one are possible, but result texture will be clamped to range [0, 1], because textures have only 8 bits per channel.
	#[serde(default)]
	pub modulate: Option<SingleArgumentFunction>,

	/// Same as abowe, but per-component modulation function.
	/// If both modulation params are none - no modulation will be used (is equivalent to multiplication by 1).
	#[serde(default)]
	pub modulate_color: Option<[SingleArgumentFunction; 3]>,
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
			twosided: false,
			draw: true,
			blocks_view: true,
			shadow: true,
			light: true,
			decals: true,
			emissive_light: [0.0, 0.0, 0.0],
			blending_mode: BlendingMode::None,
			scroll_speed: [0.0, 0.0],
			emissive_layer: None,
			framed_animation: None,
			special_effect: SpecialMaterialEffect::None,
			extra: HashMap::new(),
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

fn default_animation_frame_duration() -> f32
{
	0.5
}
