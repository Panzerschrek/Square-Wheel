use super::textures::*;
use common::{bsp_map_compact, material::*};

pub struct MapMaterialsProcessor
{
	materials: Vec<Material>,
	textures: Vec<TextureWithMips>,
	// Store here only animated textures.
	textures_modified: Vec<TextureWithMips>,
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

		let textures_modified = vec![TextureWithMips::default(); textures.len()];

		Self {
			materials,
			textures,
			textures_modified,
		}
	}

	pub fn update(&mut self, current_time_s: f32)
	{
		for (material, (src_texture, dst_texture)) in self
			.materials
			.iter()
			.zip(self.textures.iter().zip(self.textures_modified.iter_mut()))
		{
			if let Some(turb) = &material.turb
			{
				for mip_index in 0 .. NUM_MIPS
				{
					let src_mip = &src_texture[mip_index];
					let dst_mip = &mut dst_texture[mip_index];
					if dst_mip.pixels.is_empty()
					{
						*dst_mip = src_mip.clone();
					}

					make_turb_distortion(turb, current_time_s, src_mip, dst_mip);
				}
			}
		}
	}

	pub fn get_material(&self, material_index: u32) -> &Material
	{
		&self.materials[material_index as usize]
	}

	pub fn get_texture(&self, material_index: u32) -> &TextureWithMips
	{
		let animated_texture = &self.textures_modified[material_index as usize];
		if !animated_texture[0].pixels.is_empty()
		{
			// Return texture animated for current frame.
			return animated_texture;
		}

		// Return source texture.
		&self.textures[material_index as usize]
	}
}

fn make_turb_distortion(turb: &TurbParams, current_time_s: f32, src: &Texture, dst: &mut Texture)
{
	// TODO - perform actual turb effect.
	for y in 0 .. src.size[1]
	{
		let shift = y;
		for x in 0 .. src.size[0]
		{
			dst.pixels[((x + shift).rem_euclid(src.size[0]) + y * src.size[0]) as usize] =
				src.pixels[(x + y * src.size[0]) as usize];
		}
	}
}
