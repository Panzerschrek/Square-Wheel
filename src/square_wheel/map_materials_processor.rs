use super::{abstract_color::*, fast_math::*, resources_manager::*, textures::*};
use crate::common::{bsp_map_compact, color::*, material::*};
use std::{borrow::Borrow, collections::HashMap};

pub struct MapMaterialsProcessor
{
	textures: Vec<MapTextureData>,
	skybox_textures_32: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color32>>>,
	skybox_textures_64: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color64>>>,
	temp_buffer: Vec<TextureElement>,
}

pub type TextureShift = [i32; 2];

impl MapMaterialsProcessor
{
	pub fn new(resources_manager: ResourcesManagerSharedPtr, map: &bsp_map_compact::BSPMap) -> Self
	{
		let mut r = resources_manager.lock().unwrap();
		let all_materials = r.get_materials();

		let mut map_textures_loaded = r.get_map_material_textures(map);
		let mut skybox_textures_32 = HashMap::new();
		let mut skybox_textures_64 = HashMap::new();

		let mut textures = Vec::with_capacity(map.textures.len());
		for (texture_index, (texture, material_name)) in map_textures_loaded
			.drain(..)
			.zip(map.textures.iter().map(bsp_map_compact::get_texture_string))
			.enumerate()
		{
			let material = if let Some(material) = all_materials.get(material_name)
			{
				material.clone()
			}
			else
			{
				println!("Failed to find material \"{}\"", material_name);
				Material::default()
			};

			let emissive_texture = material.emissive_layer.as_ref().map(|l| r.get_texture_lite(&l.image));

			// TODO - load skyboxes lazily.
			// TODO - create stub regular texture for skyboxes.
			if material.skybox.is_some()
			{
				skybox_textures_32.insert(texture_index as u32, r.get_skybox_textures_32(material_name));
				skybox_textures_64.insert(texture_index as u32, r.get_skybox_textures_64(material_name));
			}

			textures.push(MapTextureData {
				material,
				texture,
				texture_modified: TextureWithMips::default(),
				emissive_texture,
				shift: [0, 0],
			});
		}

		Self {
			textures,
			skybox_textures_32,
			skybox_textures_64,
			temp_buffer: Vec::new(),
		}
	}

	pub fn update(&mut self, current_time_s: f32)
	{
		// Update shifts.
		for texture_data in &mut self.textures
		{
			for i in 0 .. 2
			{
				if texture_data.material.scroll_speed[i] != 0.0
				{
					texture_data.shift[i] = ((texture_data.material.scroll_speed[i] * current_time_s) as i32)
						.rem_euclid(texture_data.texture[0].size[i] as i32);
				}
			}
		}

		// TODO - maybe perform lazy update (on demand)?

		// TODO - maybe use parallel for here?
		for texture_data in &mut self.textures
		{
			if let Some(turb) = &texture_data.material.turb
			{
				for mip_index in 0 .. NUM_MIPS
				{
					let src_mip = &texture_data.texture[mip_index];
					let dst_mip = &mut texture_data.texture_modified[mip_index];
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
		&self.textures[material_index as usize].material
	}

	pub fn get_texture(&self, material_index: u32) -> &TextureWithMips
	{
		let texture_data = &self.textures[material_index as usize];
		if !texture_data.texture_modified[0].pixels.is_empty()
		{
			// Return texture animated for current frame.
			&texture_data.texture_modified
		}
		else
		{
			// Return source texture.
			&texture_data.texture
		}
	}

	pub fn get_texture_shift(&self, material_index: u32) -> TextureShift
	{
		self.textures[material_index as usize].shift
	}

	pub fn get_emissive_texture(&self, material_index: u32) -> Option<(&TextureLiteWithMips, [f32; 3])>
	{
		let texture_data = &self.textures[material_index as usize];
		if let (Some(emissive_layer), Some(emissive_texture)) =
			(&texture_data.material.emissive_layer, &texture_data.emissive_texture)
		{
			Some((emissive_texture, emissive_layer.light))
		}
		else
		{
			None
		}
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

struct MapTextureData
{
	material: Material,
	texture: SharedResourcePtr<TextureWithMips>,
	// Non-empty for textures with animations.
	texture_modified: TextureWithMips,
	// Non-empty if emissive texture exists.
	emissive_texture: Option<SharedResourcePtr<TextureLiteWithMips>>,
	shift: TextureShift,
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

	let size = [src.size[0] as i32, src.size[1] as i32];

	// Shift rows.
	for y in 0 .. size[1]
	{
		let shift =
			(f32_mul_add(y as f32, frequency_scaled, time_based_shift).sin() * amplitude_corrected).round() as i32;

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

		let shift =
			(f32_mul_add(x as f32, frequency_scaled, time_based_shift).sin() * amplitude_corrected).round() as i32;

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
