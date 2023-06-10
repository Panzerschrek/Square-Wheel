use super::{abstract_color::*, fast_math::*, map_materials_processor_structs::*, surfaces, textures::*};
use crate::common::{color::*, material::*};

pub struct GenerativeTextureEffectLayered
{
	layered_animation: LayeredAnimation,
	animation_textures: Vec<TextureIndex>,
}

impl GenerativeTextureEffectLayered
{
	pub fn new<MaterialLoadFunction: FnMut(&str) -> TextureIndex>(
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
