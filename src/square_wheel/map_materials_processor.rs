use super::textures::*;
use common::{bsp_map_compact, material::*};

pub struct MapMaterialsProcessor
{
	materials: Vec<Material>,
	textures: Vec<TextureWithMips>,
}

impl MapMaterialsProcessor
{
	pub fn new(map: &bsp_map_compact::BSPMap, materials_path: &str, textures_path: &str) -> Self
	{
		// TODO - cache materials globally.

		let all_materials = load_materials(&std::path::PathBuf::from(materials_path));

		let mut materials = Vec::with_capacity(map.textures.len());
		for texture_name in &map.textures
		{
			let material_name_string = bsp_map_compact::get_texture_string(texture_name);
			let material = if let Some(material) = all_materials.get(material_name_string)
			{
				material.clone()
			}
			else
			{
				println!("Failed to find material \"{}\"", material_name_string);
				Material::default()
			};
			materials.push(material);
		}

		let textures = load_textures(&materials, &std::path::PathBuf::from(textures_path));

		Self { materials, textures }
	}

	pub fn update(&mut self, current_time_s: f32) {}

	pub fn get_material(&self, material_index: u32) -> &Material
	{
		&self.materials[material_index as usize]
	}

	pub fn get_texture(&self, material_index: u32) -> &TextureWithMips
	{
		&self.textures[material_index as usize]
	}
}
