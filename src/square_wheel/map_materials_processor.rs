use super::{abstract_color::*, fast_math::*, resources_manager::*, textures::*};
use crate::common::{bsp_map_compact, color::*, material::*};
use std::{borrow::Borrow, collections::HashMap};

pub struct MapMaterialsProcessor
{
	materials: Vec<Material>,
	textures: Vec<SharedResourcePtr<TextureWithMips>>,
	skybox_textures_32: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color32>>>,
	skybox_textures_64: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color64>>>,
	// Store here only animated textures.
	textures_modified: Vec<TextureWithMips>,
	temp_buffer: Vec<TextureElement>,
}

impl MapMaterialsProcessor
{
	pub fn new(resources_manager: ResourcesManagerSharedPtr, map: &bsp_map_compact::BSPMap) -> Self
	{
		let mut r = resources_manager.lock().unwrap();
		let all_materials = r.get_materials();

		let mut materials = Vec::with_capacity(map.textures.len());
		let mut textures = Vec::with_capacity(map.textures.len());
		let mut skybox_textures_32 = HashMap::new();
		let mut skybox_textures_64 = HashMap::new();
		for (texture_index, texture_name) in map.textures.iter().enumerate()
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

			let material_name = material_name_string.to_string();

			// TODO - load skyboxes lazily.
			// TODO - create stub regular texture for skyboxes.
			if material.skybox.is_some()
			{
				skybox_textures_32.insert(texture_index as u32, r.get_skybox_textures_32(&material_name));
				skybox_textures_64.insert(texture_index as u32, r.get_skybox_textures_64(&material_name));
			}

			materials.push(material);
			textures.push(r.get_material_texture(&material_name));
		}

		let textures_modified = vec![TextureWithMips::default(); textures.len()];

		Self {
			materials,
			textures,
			textures_modified,
			skybox_textures_32,
			skybox_textures_64,
			temp_buffer: Vec::new(),
		}
	}

	pub fn update(&mut self, current_time_s: f32)
	{
		// TODO - maybe perform lazy update (on demand)?

		// TODO - maybe use parallel for here?
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

					make_turb_distortion(turb, current_time_s, src_mip, dst_mip, mip_index, &mut self.temp_buffer);
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

	pub fn get_skybox_textures<ColorT: AbstractColor>(&self, material_index: u32) -> Option<&SkyboxTextures<ColorT>>
	{
		// Use an ugly hack to return proper skybox texture.
		let color_size = std::mem::size_of::<ColorT>();
		if color_size == 4
		{
			self.get_skybox_textures_32(material_index)
				.map(|x| unsafe { std::mem::transmute(x) })
		}
		else if color_size == 8
		{
			self.get_skybox_textures_64(material_index)
				.map(|x| unsafe { std::mem::transmute(x) })
		}
		else
		{
			panic!("Unsupported type!");
		}
	}

	pub fn get_skybox_textures_32(&self, material_index: u32) -> Option<&SkyboxTextures<Color32>>
	{
		self.skybox_textures_32.get(&material_index).map(|x| x.borrow())
	}

	pub fn get_skybox_textures_64(&self, material_index: u32) -> Option<&SkyboxTextures<Color64>>
	{
		self.skybox_textures_64.get(&material_index).map(|x| x.borrow())
	}
}

fn make_turb_distortion(
	turb: &TurbParams,
	current_time_s: f32,
	src: &Texture,
	dst: &mut Texture,
	mip: usize,
	temp_buffer: &mut Vec<TextureElement>,
)
{
	// TODO - speed-up this. Use unsafe f32 -> i32 conversion, use indexing without bounds check.

	let mip_scale = 1.0 / ((1 << mip) as f32);
	let amplitude_corrected = mip_scale * turb.amplitude;
	let frequency_scaled = std::f32::consts::TAU / (turb.wave_length * mip_scale);
	let time_based_shift = current_time_s * turb.frequency * std::f32::consts::TAU;
	let constant_shift = [
		turb.scroll_speed[0] * (current_time_s * mip_scale),
		turb.scroll_speed[1] * (current_time_s * mip_scale),
	];

	let size = [src.size[0] as i32, src.size[1] as i32];

	// Shift rows.
	for y in 0 .. size[1]
	{
		let shift = f32_mul_add(
			f32_mul_add(y as f32, frequency_scaled, time_based_shift).sin(),
			amplitude_corrected,
			constant_shift[0],
		)
		.round() as i32;

		let start_offset = (y * size[0]) as usize;
		let end_offset = ((y + 1) * size[0]) as usize;
		let src_line = &src.pixels[start_offset .. end_offset];
		let dst_line = &mut dst.pixels[start_offset .. end_offset];

		let mut src_x = shift.rem_euclid(size[0]);
		for dst in dst_line
		{
			*dst = src_line[src_x as usize];
			src_x += 1;
			if src_x == size[0]
			{
				src_x = 0;
			}
		}
	}

	// Shift columns.
	temp_buffer.resize(size[1] as usize, TextureElement::default());

	for x in 0 .. size[0]
	{
		for (temp_dst, y) in temp_buffer.iter_mut().zip(0 .. size[1])
		{
			*temp_dst = dst.pixels[(x + y * size[0]) as usize];
		}

		let shift = f32_mul_add(
			f32_mul_add(x as f32, frequency_scaled, time_based_shift).sin(),
			amplitude_corrected,
			constant_shift[1],
		)
		.round() as i32;

		let mut src_y = shift.rem_euclid(size[1]);
		for y in 0 .. size[1]
		{
			dst.pixels[(x + y * size[0]) as usize] = temp_buffer[src_y as usize];
			src_y += 1;
			if src_y == size[1]
			{
				src_y = 0;
			}
		}
	}
}
