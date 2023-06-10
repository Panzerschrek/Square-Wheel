use super::{
	abstract_color::*, config, generative_texture_effect_layered::*, generative_texture_effect_turb::*,
	map_materials_processor_config::*, map_materials_processor_structs::*, resources_manager::*, textures::*,
};
use crate::common::{bsp_map_compact, color::*, material::*};
use rayon::prelude::*;
use std::{borrow::Borrow, collections::HashMap};

pub struct MapMaterialsProcessor
{
	app_config: config::ConfigSharedPtr,
	config: MapMaterialsProcessorConfig,
	// Store here first map textures, than additional textures.
	// Use two vectors in order to have immutable access to one, while modifying another.
	textures: Vec<MapTextureData>,
	textures_internal: Vec<MapTextureDataInternal>,
	textures_mapping_table: Vec<TextureMappingElement>,
	skybox_textures_32: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color32>>>,
	skybox_textures_64: HashMap<u32, SharedResourcePtr<SkyboxTextures<Color64>>>,
	num_animated_texels: u32,
	current_frame: u32,
}

pub type TextureShift = [i32; 2];

impl MapMaterialsProcessor
{
	pub fn new(
		resources_manager: ResourcesManagerSharedPtr,
		app_config: config::ConfigSharedPtr,
		map: &bsp_map_compact::BSPMap,
	) -> Self
	{
		let config_parsed = MapMaterialsProcessorConfig::from_app_config(&app_config);
		config_parsed.update_app_config(&app_config); // Update JSON with struct fields.

		let mut r = resources_manager.lock().unwrap();
		let all_materials = r.get_materials();

		let mut skybox_textures_32 = HashMap::new();
		let mut skybox_textures_64 = HashMap::new();
		let mut textures = Vec::with_capacity(map.textures.len());
		let mut material_name_to_texture_index = HashMap::<String, TextureIndex>::new();

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
			if matches!(&material.special_effect, SpecialMaterialEffect::Skybox(..))
			{
				skybox_textures_32.insert(texture_index as u32, r.get_skybox_textures_32(material_name));
				skybox_textures_64.insert(texture_index as u32, r.get_skybox_textures_64(material_name));
			}

			material_name_to_texture_index.insert(material_name.to_string(), textures.len() as TextureIndex);
			textures.push(MapTextureData {
				material,
				texture,
				emissive_texture,
			});
		}

		let invalid_texture_index = !0;

		// Load additional materials for animation frames and generative effects.
		// TODO - try to load textures in parallel.
		// Fill internal texture data struct.
		// Can't use "for" loop here, because range is calculated once, but we need to iterate over all textures, including newly loaded.
		let mut textures_internal = Vec::with_capacity(textures.len());
		let mut animated_texture_order = 0;
		let mut num_animated_texels = 0;
		let mut i = 0;
		while i < textures.len()
		{
			let mut texture_data_internal = MapTextureDataInternal::default();

			texture_data_internal.next_frame_texture_index = if let Some(framed_animation) =
				&textures[i].material.framed_animation
			{
				if let Some(already_loaded_texture_index) =
					material_name_to_texture_index.get(&framed_animation.next_material_name)
				{
					*already_loaded_texture_index
				}
				else if let Some(material) = all_materials.get(&framed_animation.next_material_name).cloned()
				{
					let texture = r.get_material_texture(&framed_animation.next_material_name);
					let emissive_texture = material.emissive_layer.as_ref().map(|l| r.get_texture_lite(&l.image));

					let texture_index = textures.len() as TextureIndex;
					material_name_to_texture_index.insert(framed_animation.next_material_name.clone(), texture_index);
					textures.push(MapTextureData {
						material,
						texture,
						emissive_texture,
					});

					texture_index
				}
				else
				{
					println!("Can't find material {}", framed_animation.next_material_name);
					invalid_texture_index
				}
			}
			else
			{
				invalid_texture_index
			};

			texture_data_internal.generative_effect =
				create_generative_texture_effect(textures[i].material.special_effect.clone(), &mut |material_name| {
					if let Some(already_loaded_texture_index) = material_name_to_texture_index.get(material_name)
					{
						*already_loaded_texture_index
					}
					else if let Some(material) = all_materials.get(material_name).cloned()
					{
						let texture = r.get_material_texture(&material_name);
						let emissive_texture = material.emissive_layer.as_ref().map(|l| r.get_texture_lite(&l.image));

						let texture_index = textures.len() as TextureIndex;
						material_name_to_texture_index.insert(material_name.to_string(), texture_index);
						textures.push(MapTextureData {
							material,
							texture,
							emissive_texture,
						});

						texture_index
					}
					else
					{
						println!("Can't find material {}", material_name);
						0
					}
				});

			// Calculate animated textures order, count amount of animated textures texels.
			// We need to enumerate only animated texture sequentially in order to perform balanced sparse update.
			if let Some(generative_effect) = &texture_data_internal.generative_effect
			{
				num_animated_texels += generative_effect.get_estimated_texel_count(&textures[i], &textures);
				texture_data_internal.animated_texture_order = animated_texture_order;
				animated_texture_order += 1;
			}

			textures_internal.push(texture_data_internal);

			i += 1;
		}

		debug_assert!(textures.len() == textures_internal.len());

		// Prepare mapping table. Initially all textures are mapped to themselves.
		let mut textures_mapping_table = vec![TextureMappingElement::default(); textures.len()];
		for (index, table_element) in textures_mapping_table.iter_mut().enumerate()
		{
			*table_element = TextureMappingElement {
				index: index as TextureIndex,
				frame_change_time_point: 0.0,
			};
		}

		Self {
			app_config,
			config: config_parsed,
			textures,
			textures_internal,
			textures_mapping_table,
			skybox_textures_32,
			skybox_textures_64,
			num_animated_texels,
			current_frame: 0,
		}
	}

	pub fn get_num_animated_texels(&self) -> u32
	{
		self.num_animated_texels
	}

	pub fn update(&mut self, current_time_s: f32)
	{
		self.current_frame += 1;
		self.synchronize_config();

		self.update_framed_animations(current_time_s);
		self.update_shifts(current_time_s);
		self.update_animations(current_time_s);
	}

	fn update_framed_animations(&mut self, current_time_s: f32)
	{
		if !self.config.enable_framed_animations
		{
			return;
		}

		// Assume time never goes backwards.
		for mapping_element in &mut self.textures_mapping_table
		{
			if (self.textures_internal[mapping_element.index as usize].next_frame_texture_index as usize) <
				self.textures.len()
			{
				// Valid next frame index - this is animated texture.
				if current_time_s >= mapping_element.frame_change_time_point
				{
					// Reached frame switch time point.
					// Assume, that materials update frame rate is higher than animation frequency.

					// Set new index.
					let current_index = self.textures_internal[mapping_element.index as usize].next_frame_texture_index;
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
	}

	fn update_shifts(&mut self, current_time_s: f32)
	{
		if !self.config.enable_scrolling
		{
			return;
		}

		for (texture_data, texture_data_internal) in self.textures.iter().zip(self.textures_internal.iter_mut())
		{
			for i in 0 .. 2
			{
				if texture_data.material.scroll_speed[i] != 0.0
				{
					texture_data_internal.shift[i] = ((texture_data.material.scroll_speed[i] * current_time_s) as i32)
						.rem_euclid(texture_data.texture[0].size[i] as i32);
				}
			}
		}
	}

	fn update_animations(&mut self, current_time_s: f32)
	{
		if !self.config.enable_generative_animations
		{
			return;
		}

		// TODO - maybe perform lazy update (on demand)?
		let dynamic_period = if self.config.animated_textures_update_texels_limit == 0
		{
			// No limit - try to update all textures each frame.
			1
		}
		else
		{
			// More animated texels - greater update period.
			(self.num_animated_texels / self.config.animated_textures_update_texels_limit)
				.max(ANIMATIONS_UPDATE_PERIOD_MIN)
				.min(ANIMATIONS_UPDATE_PERIOD_MAX)
		};
		// use maximum period of two values - from config and dynamically-calculated one.
		let update_period = std::cmp::max(self.config.animated_textures_update_period, dynamic_period);
		let current_update_order = self.current_frame % update_period;

		let textures = &self.textures;
		let textures_internal = &mut self.textures_internal;
		let textures_mapping_table = &self.textures_mapping_table;

		let animate_func = |(ti, t): (&mut MapTextureDataInternal, &MapTextureData)| {
			// Perform sparse update - update each frame only one fraction of all animated textures.
			if ti.animated_texture_order % update_period == current_update_order
			{
				if let Some(generative_effect) = &mut ti.generative_effect
				{
					generative_effect.update(
						&mut ti.generative_texture_data,
						t,
						textures,
						textures_mapping_table,
						current_time_s,
					);
				}
			}
		};

		// Perform animation in parallel (if has enough threads).
		if rayon::current_num_threads() == 1
		{
			textures_internal.iter_mut().zip(textures).for_each(animate_func);
		}
		else
		{
			textures_internal.par_iter_mut().zip_eq(textures).for_each(animate_func);
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
		let texture_data_internal = &self.textures_internal[current_material_index];
		if !texture_data_internal.generative_texture_data.texture[0]
			.pixels
			.is_empty()
		{
			// Return texture animated for current frame.
			&texture_data_internal.generative_texture_data.texture
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
		self.textures_internal[current_material_index as usize].shift
	}

	// If material has emissive texture - return it together with specified light.
	pub fn get_emissive_texture(&self, material_index: u32) -> Option<(&TextureLiteWithMips, [f32; 3])>
	{
		let current_material_index = self.textures_mapping_table[material_index as usize].index as usize;
		let texture_data = &self.textures[current_material_index];
		let texture_data_internal = &self.textures_internal[current_material_index];
		if let (Some(emissive_layer), Some(emissive_texture)) =
			(&texture_data.material.emissive_layer, &texture_data.emissive_texture)
		{
			if !texture_data_internal.generative_texture_data.emissive_texture[0]
				.pixels
				.is_empty()
			{
				// Return emissive texture animated for current frame.
				Some((
					&texture_data_internal.generative_texture_data.emissive_texture,
					emissive_layer.light,
				))
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

	fn synchronize_config(&mut self)
	{
		let config_updated = MapMaterialsProcessorConfig::from_app_config(&self.app_config);
		self.config = config_updated;

		// Make sure that config values are reasonable.
		let mut config_is_dirty = false;
		if self.config.animated_textures_update_period < ANIMATIONS_UPDATE_PERIOD_MIN
		{
			self.config.animated_textures_update_period = ANIMATIONS_UPDATE_PERIOD_MIN;
			config_is_dirty = true;
		}
		if self.config.animated_textures_update_period > ANIMATIONS_UPDATE_PERIOD_MAX
		{
			self.config.animated_textures_update_period = ANIMATIONS_UPDATE_PERIOD_MAX;
			config_is_dirty = true;
		}

		if config_is_dirty
		{
			self.config.update_app_config(&self.app_config);
		}
	}
}

#[derive(Default)]
struct MapTextureDataInternal
{
	shift: TextureShift,
	// Invalid index if has no framed animation.
	next_frame_texture_index: TextureIndex,
	// Effect instance itself (if exists).
	generative_effect: OptDynGenerativeTextureEffect,
	// Data for generative texture effects.
	generative_texture_data: GenerativeTextureData,
	// Used only for sparse texture update.
	animated_texture_order: u32,
}

const ANIMATIONS_UPDATE_PERIOD_MIN: u32 = 1;
const ANIMATIONS_UPDATE_PERIOD_MAX: u32 = 16;

fn create_generative_texture_effect<MaterialLoadFunction: FnMut(&str) -> TextureIndex>(
	special_effect: SpecialMaterialEffect,
	material_load_function: &mut MaterialLoadFunction,
) -> OptDynGenerativeTextureEffect

{
	match special_effect
	{
		SpecialMaterialEffect::None => None,
		SpecialMaterialEffect::Turb(turb_params) => Some(Box::new(GenerativeTextureEffectTurb::new(turb_params))),
		SpecialMaterialEffect::LayeredAnimation(layered_animation) => Some(Box::new(
			GenerativeTextureEffectLayered::new(layered_animation, material_load_function),
		)),
		SpecialMaterialEffect::Skybox(..) => None,
	}
}
