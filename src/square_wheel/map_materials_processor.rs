use super::{
	abstract_color::*, config, fast_math::*, map_materials_processor_config::*, resources_manager::*, surfaces,
	textures::*,
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
	textures_mutable: Vec<MapTextureDataMutable>,
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
				animated_texture_order: 0,
				next_frame_texture_index: invalid_texture_index,
			});
		}

		// Load additional materials for animation frames and generative effects.
		// TODO - try to load textures in parallel.
		// Can't use "for" loop here, because range is calculated once, but we need to iterate over all textures, including newly loaded.
		let mut textures_mutable = Vec::with_capacity(textures.len());
		let mut animated_texture_order = 0;
		let mut i = 0;
		while i < textures.len()
		{
			if let Some(framed_animation) = &textures[i].material.framed_animation
			{
				if let Some(already_loaded_texture_index) =
					material_name_to_texture_index.get(&framed_animation.next_material_name)
				{
					textures[i].next_frame_texture_index = *already_loaded_texture_index;
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
						animated_texture_order: 0,
						next_frame_texture_index: invalid_texture_index,
					});

					textures[i].next_frame_texture_index = texture_index;
				}
				else
				{
					println!("Can't find material {}", framed_animation.next_material_name);
				}
			}

			let mut texture_mutable = MapTextureDataMutable::default();

			texture_mutable.generative_effect =
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
							animated_texture_order: 0,
							next_frame_texture_index: invalid_texture_index,
						});

						texture_index
					}
					else
					{
						println!("Can't find material {}", material_name);
						0
					}
				});

			// Calculate animated textures order.
			// We need to enumerate only animated texture sequentially in order to perform balanced sparse update.
			if texture_mutable.generative_effect.is_some()
			{
				textures[i].animated_texture_order = animated_texture_order;
				animated_texture_order += 1;
			}

			textures_mutable.push(texture_mutable);

			i += 1;
		}

		debug_assert!(textures.len() == textures_mutable.len());

		// Calculate amount of animated textures texels.
		// TODO - fix this.
		let num_animated_texels = 0;

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
			textures_mutable,
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
			if (self.textures[mapping_element.index as usize].next_frame_texture_index as usize) < self.textures.len()
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
	}

	fn update_shifts(&mut self, current_time_s: f32)
	{
		if !self.config.enable_scrolling
		{
			return;
		}

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
		let textures_mutable = &mut self.textures_mutable;
		let textures_mapping_table = &self.textures_mapping_table;

		let animate_func = |(tm, t): (&mut MapTextureDataMutable, &MapTextureData)| {
			// Perform sparse update - update each frame only one fraction of all animated textures.
			if t.animated_texture_order % update_period == current_update_order
			{
				if let Some(generative_effect) = &mut tm.generative_effect
				{
					generative_effect.update(
						&mut tm.generative_texture_data,
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
			textures_mutable.iter_mut().zip(textures).for_each(animate_func);
		}
		else
		{
			textures_mutable.par_iter_mut().zip_eq(textures).for_each(animate_func);
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
		if !texture_data_mutable.generative_texture_data.texture_modified[0]
			.pixels
			.is_empty()
		{
			// Return texture animated for current frame.
			&texture_data_mutable.generative_texture_data.texture_modified
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
			if !texture_data_mutable.generative_texture_data.emissive_texture_modified[0]
				.pixels
				.is_empty()
			{
				// Return emissive texture animated for current frame.
				Some((
					&texture_data_mutable.generative_texture_data.emissive_texture_modified,
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

struct MapTextureData
{
	material: Material,
	texture: SharedResourcePtr<TextureWithMips>,
	// Non-empty if emissive texture exists.
	emissive_texture: Option<SharedResourcePtr<TextureLiteWithMips>>,
	// Used only for sparse texture update.
	animated_texture_order: u32,
	// Invalid index if has no framed animation.
	next_frame_texture_index: TextureIndex,
}

#[derive(Default)]
struct MapTextureDataMutable
{
	generative_texture_data: GenerativeTextureData,
	generative_effect: OptDynGenerativeTextureEffect,
	shift: TextureShift,
}

// Data created/modified by generative effect.
#[derive(Default)]
struct GenerativeTextureData
{
	// Non-empty for textures with animations.
	texture_modified: TextureWithMips,
	// Exists only for emissive textures with animations.
	emissive_texture_modified: TextureLiteWithMips,
}

type OptDynGenerativeTextureEffect = Option<Box<dyn GenerativeTextureEffect + Send + Sync>>;

trait GenerativeTextureEffect
{
	fn update(
		&mut self,
		texture_data_mutable: &mut GenerativeTextureData,
		texture_data: &MapTextureData,
		all_textures_data: &[MapTextureData],
		textures_mapping_table: &[TextureMappingElement],
		current_time_s: f32,
	);
}

#[derive(Default, Copy, Clone)]
struct TextureMappingElement
{
	index: TextureIndex,
	frame_change_time_point: f32,
}

// Use indeces instead of strings in order to access map textures.
type TextureIndex = u32;

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

struct GenerativeTextureEffectTurb
{
	turb_params: TurbParams,
	// TODO - put temp buffer here.
}

impl GenerativeTextureEffectTurb
{
	fn new(turb_params: TurbParams) -> Self
	{
		Self { turb_params }
	}
}

impl GenerativeTextureEffect for GenerativeTextureEffectTurb
{
	fn update(
		&mut self,
		texture_data_mutable: &mut GenerativeTextureData,
		texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		current_time_s: f32,
	)
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
				&self.turb_params,
				current_time_s,
				[src_mip.size[0] as i32, src_mip.size[1] as i32],
				mip_index,
				&src_mip.pixels,
				&mut dst_mip.pixels,
			);
		}

		if let Some(emissive_texture) = &texture_data.emissive_texture
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
					&self.turb_params,
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

struct GenerativeTextureEffectLayered
{
	layered_animation: LayeredAnimation,
	animation_textures: Vec<TextureIndex>,
}

impl GenerativeTextureEffectLayered
{
	fn new<MaterialLoadFunction: FnMut(&str) -> TextureIndex>(
		layered_animation: LayeredAnimation,
		material_load_function: &mut MaterialLoadFunction,
	) -> Self
	{
		let mut animation_textures = Vec::with_capacity(layered_animation.layers.len());
		for layer in &layered_animation.layers
		{
			animation_textures.push(material_load_function(&layer.material_name));
		}

		Self {
			layered_animation,
			animation_textures,
		}
	}
}

impl GenerativeTextureEffect for GenerativeTextureEffectLayered
{
	fn update(
		&mut self,
		texture_data_mutable: &mut GenerativeTextureData,
		_texture_data: &MapTextureData,
		all_textures_data: &[MapTextureData],
		textures_mapping_table: &[TextureMappingElement],
		current_time_s: f32,
	)
	{
		for mip_index in 0 .. NUM_MIPS
		{
			for (animation_layer, texture_index) in self.layered_animation.layers.iter().zip(&self.animation_textures)
			{
				let shift = animation_layer
					.tex_coord_shift
					.map(|f| (f.evaluate(current_time_s) as i32) >> mip_index);

				const MAX_LIGHT: f32 = 127.0;
				let light = if let Some(modulate_color) = &animation_layer.modulate_color
				{
					modulate_color.map(|f| f.evaluate(current_time_s).max(0.0).min(MAX_LIGHT))
				}
				else if let Some(modulate) = &animation_layer.modulate
				{
					[modulate.evaluate(current_time_s).max(0.0).min(MAX_LIGHT); 3]
				}
				else
				{
					[1.0; 3]
				};

				const ALMOST_ZERO_LIGHT: f32 = 1.0 / 128.0;
				let light_is_zero =
					light[0] <= ALMOST_ZERO_LIGHT && light[1] <= ALMOST_ZERO_LIGHT && light[2] <= ALMOST_ZERO_LIGHT;

				let texture_index_corrected = if animation_layer.follow_framed_animation
				{
					textures_mapping_table[*texture_index as usize].index
				}
				else
				{
					*texture_index
				};
				let layer_texture = &all_textures_data[texture_index_corrected as usize];
				let blending_mode = layer_texture.material.blending_mode;

				// Adding zero has no effect. So, if light is zero skip applying this layer textures.
				let adding_zero = blending_mode == BlendingMode::Additive && light_is_zero;

				if layer_texture.material.diffuse.is_some()
				{
					// Mix diffuse layer only if it exists.
					let src_mip = &layer_texture.texture[mip_index];
					let dst_mip = &mut texture_data_mutable.texture_modified[mip_index];
					if dst_mip.pixels.is_empty()
					{
						*dst_mip = src_mip.clone();
						dst_mip.has_normal_map = false;
						dst_mip.has_non_one_roughness = false;
						dst_mip.is_metal = false;
					}

					if !adding_zero
					{
						apply_texture_layer(dst_mip.size, &mut dst_mip.pixels, src_mip, shift, light, blending_mode);
					}

					dst_mip.has_normal_map |= src_mip.has_normal_map;
					dst_mip.has_non_one_roughness |= src_mip.has_non_one_roughness;
					dst_mip.is_metal |= src_mip.is_metal;
				}

				if let Some(emissive_texture) = &layer_texture.emissive_texture
				{
					// Mix emissive layer only if it exists.
					let src_mip = &emissive_texture[mip_index];
					let dst_mip = &mut texture_data_mutable.emissive_texture_modified[mip_index];
					if dst_mip.pixels.is_empty()
					{
						*dst_mip = src_mip.clone();
					}

					if !adding_zero
					{
						// Use for emissive texture blending same code, as for surfaces.
						surfaces::mix_surface_with_texture(
							dst_mip.size,
							shift,
							src_mip,
							blending_mode,
							light,
							&mut dst_mip.pixels,
						);
					}
				}
			}
		}
	}
}

fn apply_texture_layer(
	texture_size: [u32; 2],
	texture_data: &mut [TextureElement],
	layer_texture: &Texture,
	layer_texture_offset: [i32; 2],
	light: [f32; 3],
	blending_mode: BlendingMode,
)
{
	if blending_mode == BlendingMode::None &&
		texture_size == layer_texture.size &&
		layer_texture_offset == [0, 0] &&
		light == [1.0, 1.0, 1.0]
	{
		// Fast path - just copy source into destination without any modulation, shift, tiling and blending.
		texture_data.copy_from_slice(&layer_texture.pixels);
		return;
	}

	match blending_mode
	{
		BlendingMode::None => apply_texture_layer_impl_1::<BLENDING_MODE_NONE>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		),
		BlendingMode::Average => apply_texture_layer_impl_1::<BLENDING_MODE_AVERAGE>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		),
		BlendingMode::Additive => apply_texture_layer_impl_1::<BLENDING_MODE_ADDITIVE>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		),
		BlendingMode::AlphaTest => apply_texture_layer_impl_1::<BLENDING_MODE_ALPHA_TEST>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		),
		BlendingMode::AlphaBlend => apply_texture_layer_impl_1::<BLENDING_MODE_ALPHA_BLEND>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		),
	}
}

fn apply_texture_layer_impl_1<const BLENDING_MODE: usize>(
	texture_size: [u32; 2],
	texture_data: &mut [TextureElement],
	layer_texture: &Texture,
	layer_texture_offset: [i32; 2],
	light: [f32; 3],
)
{
	let mut modulate = false;
	for component in light
	{
		modulate |= component < 0.98 || component > 1.02
	}

	if modulate
	{
		apply_texture_layer_impl_2::<BLENDING_MODE, true>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		);
	}
	else
	{
		apply_texture_layer_impl_2::<BLENDING_MODE, false>(
			texture_size,
			texture_data,
			layer_texture,
			layer_texture_offset,
			light,
		);
	}
}

fn apply_texture_layer_impl_2<const BLENDING_MODE: usize, const MODULATE: bool>(
	texture_size: [u32; 2],
	texture_data: &mut [TextureElement],
	layer_texture: &Texture,
	layer_texture_offset: [i32; 2],
	light: [f32; 3],
)
{
	const LIGHT_SHIFT: i32 = 8;
	let light_scale = (1 << LIGHT_SHIFT) as f32;
	let light_vec =
		ColorVecI::from_color_f32x3(&[light[0] * light_scale, light[1] * light_scale, light[2] * light_scale]);

	for dst_v in 0 .. texture_size[1]
	{
		let dst_line_start = (dst_v * texture_size[0]) as usize;
		let dst_line = &mut texture_data[dst_line_start .. dst_line_start + (texture_size[0] as usize)];

		let src_v = (layer_texture_offset[1] + (dst_v as i32)).rem_euclid(layer_texture.size[1] as i32);
		let src_line_start = ((src_v as u32) * layer_texture.size[0]) as usize;
		let src_line = &layer_texture.pixels[src_line_start .. src_line_start + (layer_texture.size[0] as usize)];
		let mut src_u = layer_texture_offset[0].rem_euclid(layer_texture.size[0] as i32);

		for dst_texel in dst_line.iter_mut()
		{
			let texel_value = unsafe { debug_only_checked_fetch(src_line, src_u as usize) };
			if MODULATE
			{
				// Mix with modulated by light layer.
				let texel_value_modulated = ColorVecI::shift_right::<LIGHT_SHIFT>(&ColorVecI::mul(
					&ColorVecI::from_color32(texel_value.diffuse),
					&light_vec,
				));

				if BLENDING_MODE == BLENDING_MODE_NONE
				{
					dst_texel.diffuse = texel_value_modulated.into();
					dst_texel.packed_normal_roughness = texel_value.packed_normal_roughness;
				}
				else if BLENDING_MODE == BLENDING_MODE_AVERAGE
				{
					// TODO - support normals/roughness blending.
					dst_texel.diffuse = ColorVecI::shift_right::<1>(&ColorVecI::add(
						&texel_value_modulated,
						&ColorVecI::from_color32(dst_texel.diffuse),
					))
					.into();
				}
				else if BLENDING_MODE == BLENDING_MODE_ADDITIVE
				{
					// TODO - support normals/roughness blending.
					dst_texel.diffuse =
						ColorVecI::add(&texel_value_modulated, &ColorVecI::from_color32(dst_texel.diffuse)).into();
				}
				else if BLENDING_MODE == BLENDING_MODE_ALPHA_TEST
				{
					if texel_value.diffuse.test_alpha()
					{
						dst_texel.diffuse = texel_value_modulated.into();
						dst_texel.packed_normal_roughness = texel_value.packed_normal_roughness;
					}
				}
				else if BLENDING_MODE == BLENDING_MODE_ALPHA_BLEND
				{
					// TODO - support normals/roughness blending.
					let alpha = texel_value.diffuse.get_alpha();
					dst_texel.diffuse = ColorVecI::shift_right::<8>(&ColorVecI::add(
						&ColorVecI::mul_scalar(&texel_value_modulated, alpha),
						&ColorVecI::mul_scalar(&ColorVecI::from_color32(dst_texel.diffuse), 255 - alpha),
					))
					.into();
				}
			}
			else
			{
				// Mix with initial texture (without modulation).
				if BLENDING_MODE == BLENDING_MODE_NONE
				{
					*dst_texel = texel_value;
					dst_texel.packed_normal_roughness = texel_value.packed_normal_roughness;
				}
				else if BLENDING_MODE == BLENDING_MODE_AVERAGE
				{
					// TODO - support normals/roughness blending.
					dst_texel.diffuse = Color32::get_average(dst_texel.diffuse, texel_value.diffuse);
				}
				else if BLENDING_MODE == BLENDING_MODE_ADDITIVE
				{
					// TODO - support normals/roughness blending.
					dst_texel.diffuse = ColorVecI::add(
						&ColorVecI::from_color32(texel_value.diffuse),
						&ColorVecI::from_color32(dst_texel.diffuse),
					)
					.into();
				}
				else if BLENDING_MODE == BLENDING_MODE_ALPHA_TEST
				{
					if texel_value.diffuse.test_alpha()
					{
						*dst_texel = texel_value;
						dst_texel.packed_normal_roughness = texel_value.packed_normal_roughness;
					}
				}
				else if BLENDING_MODE == BLENDING_MODE_ALPHA_BLEND
				{
					// TODO - support normals/roughness blending.
					let alpha = texel_value.diffuse.get_alpha();
					dst_texel.diffuse = ColorVecI::shift_right::<8>(&ColorVecI::add(
						&ColorVecI::mul_scalar(&ColorVecI::from_color32(texel_value.diffuse), alpha),
						&ColorVecI::mul_scalar(&ColorVecI::from_color32(dst_texel.diffuse), 255 - alpha),
					))
					.into();
				}
			}

			src_u += 1;
			if src_u == (layer_texture.size[0] as i32)
			{
				src_u = 0;
			}
		}
	}
}
