use super::{abstract_color::*, fast_math::*, resources_manager::*, textures::*};
use crate::common::{bsp_map_compact, color::*, material::*};
use rayon::prelude::*;
use std::{borrow::Borrow, collections::HashMap};

pub struct MapMaterialsProcessor
{
	// First map textures, than additional textures.
	// use two vectors in order to have immutable access to one, while modifying another.
	textures: Vec<MapTextureData>,
	textures_mutable: Vec<MapTextureDataMutable>,
	textures_mapping_table: Vec<TextureMappingElement>,
	skybox_textures_32: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color32>>>,
	skybox_textures_64: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color64>>>,
}

pub type TextureShift = [i32; 2];

impl MapMaterialsProcessor
{
	pub fn new(resources_manager: ResourcesManagerSharedPtr, map: &bsp_map_compact::BSPMap) -> Self
	{
		let mut r = resources_manager.lock().unwrap();
		let all_materials = r.get_materials();

		let mut skybox_textures_32 = HashMap::new();
		let mut skybox_textures_64 = HashMap::new();
		let mut textures = Vec::with_capacity(map.textures.len());
		let mut material_name_to_texture_index = HashMap::<String, u32>::new();

		let invalid_texture_index = !0;

		for (texture_index, (texture, material_name)) in r
			.get_map_material_textures(map)
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

			// TODO - load emissive textures in parallel.
			let emissive_texture = material.emissive_layer.as_ref().map(|l| r.get_texture_lite(&l.image));

			// TODO - load skyboxes lazily.
			// TODO - create stub regular texture for skyboxes.
			if material.skybox.is_some()
			{
				skybox_textures_32.insert(texture_index as u32, r.get_skybox_textures_32(material_name));
				skybox_textures_64.insert(texture_index as u32, r.get_skybox_textures_64(material_name));
			}

			material_name_to_texture_index.insert(material_name.to_string(), textures.len() as u32);
			textures.push(MapTextureData {
				material,
				texture,
				emissive_texture,
				next_frame_texture_index: invalid_texture_index,
				layered_animation_layers_texture_index: Vec::new(),
			});
		}

		// Load additional materials for animation frames.
		// TODO - try to load textures in parallel.
		// Can't use "for" loop here, because range is calculated once, but we need to iterate over all textures, including newly loaded.
		let mut i = 0;
		while i < textures.len()
		{
			let material = &textures[i].material;
			if let Some(framed_animation) = &material.framed_animation
			{
				if let Some(already_loaded_texture_index) =
					material_name_to_texture_index.get(&framed_animation.next_material_name)
				{
					textures[i].next_frame_texture_index = *already_loaded_texture_index;
				}
				else
				{
					if let Some(material) = all_materials.get(&framed_animation.next_material_name).as_deref()
					{
						let texture = r.get_material_texture(&framed_animation.next_material_name);
						let emissive_texture = material.emissive_layer.as_ref().map(|l| r.get_texture_lite(&l.image));

						let texture_index = textures.len() as u32;
						material_name_to_texture_index
							.insert(framed_animation.next_material_name.clone(), texture_index);
						textures.push(MapTextureData {
							material: material.clone(),
							texture,
							emissive_texture,
							next_frame_texture_index: invalid_texture_index,
							layered_animation_layers_texture_index: Vec::new(),
						});

						textures[i].next_frame_texture_index = texture_index;
					}
					else
					{
						println!("Can't find material {}", framed_animation.next_material_name);
					}
				}
			}

			i += 1;
		}

		// Load additional materials for layered animations.
		// TODO - try to load textures in parallel.
		// Can't use "for" loop here, because range is calculated once, but we need to iterate over all textures, including newly loaded.
		let mut i = 0;
		while i < textures.len()
		{
			if let Some(layered_animation) = textures[i].material.layered_animation.clone()
			{
				for layered_animation_layer in &layered_animation.layers
				{
					if let Some(already_loaded_texture_index) =
						material_name_to_texture_index.get(&layered_animation_layer.material_name)
					{
						textures[i]
							.layered_animation_layers_texture_index
							.push(*already_loaded_texture_index);
					}
					else
					{
						if let Some(material) = all_materials.get(&layered_animation_layer.material_name).as_deref()
						{
							let texture = r.get_material_texture(&layered_animation_layer.material_name);
							let emissive_texture =
								material.emissive_layer.as_ref().map(|l| r.get_texture_lite(&l.image));

							let texture_index = textures.len() as u32;
							material_name_to_texture_index
								.insert(layered_animation_layer.material_name.clone(), texture_index);
							textures.push(MapTextureData {
								material: material.clone(),
								texture,
								emissive_texture,
								next_frame_texture_index: invalid_texture_index,
								layered_animation_layers_texture_index: Vec::new(),
							});

							textures[i].layered_animation_layers_texture_index.push(texture_index);
						}
						else
						{
							println!("Can't find material {}", layered_animation_layer.material_name);
							textures[i].layered_animation_layers_texture_index.push(0);
						}
					}
				}
				debug_assert!(
					textures[i].layered_animation_layers_texture_index.len() == layered_animation.layers.len()
				);
			}

			i += 1;
		}

		let textures_mutable = vec![MapTextureDataMutable::default(); textures.len()];

		// Prepare mapping table. Initially all textures are mapped to themselves.
		let mut textures_mapping_table = vec![TextureMappingElement::default(); textures.len()];
		for (index, table_element) in textures_mapping_table.iter_mut().enumerate()
		{
			*table_element = TextureMappingElement {
				index: index as u32,
				frame_change_time_point: 0.0,
			};
		}

		Self {
			textures,
			textures_mutable,
			textures_mapping_table,
			skybox_textures_32,
			skybox_textures_64,
		}
	}

	pub fn update(&mut self, current_time_s: f32)
	{
		// Update framed animations.
		// Assume time never goes backwards.
		for mapping_element in &mut self.textures_mapping_table
		{
			if self.textures[mapping_element.index as usize].next_frame_texture_index < self.textures.len() as u32
			{
				// Valid next frame index - this is animated texture.
				if current_time_s >= mapping_element.frame_change_time_point
				{
					// Reached frame switch time point.
					// Assume, that materials update frame rate is higher than animation frequency.

					// Set new index.
					let current_index = self.textures[mapping_element.index as usize].next_frame_texture_index;
					mapping_element.index = current_index;
					// Use duration of current frame for calculation of next frame change time point.
					let duration = if let Some(framed_animation) =
						&self.textures[current_index as usize].material.framed_animation
					{
						framed_animation.duration
					}
					else
					{
						0.5 // WTF?
					};

					mapping_element.frame_change_time_point += duration;
				}
			}
		}

		// Update shifts.
		for (texture_data, texture_data_mutable) in self.textures.iter().zip(self.textures_mutable.iter_mut())
		{
			for i in 0 .. 2
			{
				if texture_data.material.scroll_speed[i] != 0.0
				{
					texture_data_mutable.shift[i] = ((texture_data.material.scroll_speed[i] * current_time_s) as i32)
						.rem_euclid(texture_data.texture[0].size[i] as i32);
				}
			}
		}

		// Perform animation in parallel (if has enough threads).
		// TODO - maybe perform lazy update (on demand)?
		if rayon::current_num_threads() == 1
		{
			self.textures
				.iter_mut()
				.zip(&mut self.textures_mutable)
				.for_each(|(t, tm)| animate_texture(t, tm, current_time_s));
		}
		else
		{
			self.textures
				.par_iter_mut()
				.zip_eq(&mut self.textures_mutable)
				.for_each(|(t, tm)| animate_texture(t, tm, current_time_s));
		}
	}

	pub fn get_material(&self, material_index: u32) -> &Material
	{
		let current_material_index = self.textures_mapping_table[material_index as usize].index;
		&self.textures[current_material_index as usize].material
	}

	pub fn get_texture(&self, material_index: u32) -> &TextureWithMips
	{
		let current_material_index = self.textures_mapping_table[material_index as usize].index as usize;
		let texture_data = &self.textures[current_material_index];
		let texture_data_mutable = &self.textures_mutable[current_material_index];
		if !texture_data_mutable.texture_modified[0].pixels.is_empty()
		{
			// Return texture animated for current frame.
			&texture_data_mutable.texture_modified
		}
		else
		{
			// Return source texture.
			&texture_data.texture
		}
	}

	pub fn get_texture_shift(&self, material_index: u32) -> TextureShift
	{
		let current_material_index = self.textures_mapping_table[material_index as usize].index;
		self.textures_mutable[current_material_index as usize].shift
	}

	// If material has emissive texture - return it together with specified light.
	pub fn get_emissive_texture(&self, material_index: u32) -> Option<(&TextureLiteWithMips, [f32; 3])>
	{
		let current_material_index = self.textures_mapping_table[material_index as usize].index as usize;
		let texture_data = &self.textures[current_material_index];
		let texture_data_mutable = &self.textures_mutable[current_material_index];
		if let (Some(emissive_layer), Some(emissive_texture)) =
			(&texture_data.material.emissive_layer, &texture_data.emissive_texture)
		{
			if !texture_data_mutable.emissive_texture_modified[0].pixels.is_empty()
			{
				// Return emissive texture animated for current frame.
				Some((&texture_data_mutable.emissive_texture_modified, emissive_layer.light))
			}
			else
			{
				// Return source emissive texture.
				Some((emissive_texture, emissive_layer.light))
			}
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
	// Non-empty if emissive texture exists.
	emissive_texture: Option<SharedResourcePtr<TextureLiteWithMips>>,
	// Invalid index if has no framed animation.
	next_frame_texture_index: u32,
	layered_animation_layers_texture_index: Vec<u32>,
}

#[derive(Default, Clone)]
struct MapTextureDataMutable
{
	// Non-empty for textures with animations.
	texture_modified: TextureWithMips,
	// Exists only for emissive texturex with mips.
	emissive_texture_modified: TextureLiteWithMips,
	shift: TextureShift,
}

#[derive(Default, Copy, Clone)]
struct TextureMappingElement
{
	index: u32,
	frame_change_time_point: f32,
}

fn animate_texture(
	texture_data: &mut MapTextureData,
	texture_data_mutable: &mut MapTextureDataMutable,
	current_time_s: f32,
)
{
	if let Some(turb) = &texture_data.material.turb
	{
		for mip_index in 0 .. NUM_MIPS
		{
			let src_mip = &texture_data.texture[mip_index];
			let dst_mip = &mut texture_data_mutable.texture_modified[mip_index];
			if dst_mip.pixels.is_empty()
			{
				*dst_mip = src_mip.clone();
			}

			make_turb_distortion(
				turb,
				current_time_s,
				[src_mip.size[0] as i32, src_mip.size[1] as i32],
				mip_index,
				&src_mip.pixels,
				&mut dst_mip.pixels,
			);
		}

		if let Some(emissive_texture) = &mut texture_data.emissive_texture
		{
			for mip_index in 0 .. NUM_MIPS
			{
				let src_mip = &emissive_texture[mip_index];
				let dst_mip = &mut texture_data_mutable.emissive_texture_modified[mip_index];
				if dst_mip.pixels.is_empty()
				{
					*dst_mip = src_mip.clone();
				}

				make_turb_distortion(
					turb,
					current_time_s,
					[src_mip.size[0] as i32, src_mip.size[1] as i32],
					mip_index,
					&src_mip.pixels,
					&mut dst_mip.pixels,
				);
			}
		}
	}
}

fn make_turb_distortion<T: Copy + Default>(
	turb: &TurbParams,
	current_time_s: f32,
	size: [i32; 2],
	mip: usize,
	src_pixels: &[T],
	dst_pixels: &mut [T],
)
{
	// TODO - speed-up this. Use unsafe f32 -> i32 conversion, use indexing without bounds check.

	let mip_scale = 1.0 / ((1 << mip) as f32);
	let amplitude_corrected = mip_scale * turb.amplitude;
	let frequency_scaled = std::f32::consts::TAU / (turb.wave_length * mip_scale);
	let time_based_shift = current_time_s * turb.frequency * std::f32::consts::TAU;

	// Shift rows.
	for y in 0 .. size[1]
	{
		let shift =
			(f32_mul_add(y as f32, frequency_scaled, time_based_shift).sin() * amplitude_corrected).round() as i32;

		let start_offset = (y * size[0]) as usize;
		let end_offset = ((y + 1) * size[0]) as usize;
		let src_line = &src_pixels[start_offset .. end_offset];
		let dst_line = &mut dst_pixels[start_offset .. end_offset];

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
	const MAX_TURB_TEXTURE_HEIGHT: usize = 1024;
	if size[1] > MAX_TURB_TEXTURE_HEIGHT as i32
	{
		return;
	}

	let mut temp_buffer = [T::default(); MAX_TURB_TEXTURE_HEIGHT]; // TODO - use uninitialized memory

	for x in 0 .. size[0]
	{
		for (temp_dst, y) in temp_buffer.iter_mut().zip(0 .. size[1])
		{
			*temp_dst = dst_pixels[(x + y * size[0]) as usize];
		}

		let shift =
			(f32_mul_add(x as f32, frequency_scaled, time_based_shift).sin() * amplitude_corrected).round() as i32;

		let mut src_y = shift.rem_euclid(size[1]);
		for y in 0 .. size[1]
		{
			dst_pixels[(x + y * size[0]) as usize] = temp_buffer[src_y as usize];
			src_y += 1;
			if src_y == size[1]
			{
				src_y = 0;
			}
		}
	}
}
