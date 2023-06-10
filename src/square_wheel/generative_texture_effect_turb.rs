use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::material::*;

pub struct GenerativeTextureEffectTurb
{
	turb_params: TurbParams,
	// TODO - put temp buffer here.
}

impl GenerativeTextureEffectTurb
{
	pub fn new(turb_params: TurbParams) -> Self
	{
		Self { turb_params }
	}
}

impl GenerativeTextureEffect for GenerativeTextureEffectTurb
{
	fn get_estimated_texel_count(&self, texture_data: &MapTextureData, _all_textures_data: &[MapTextureData]) -> u32
	{
		let mut s = texture_data.texture[0].size[0] * texture_data.texture[0].size[1];
		if let Some(emissive_texture) = &texture_data.emissive_texture
		{
			// Count emisive texels as half-texels.
			s += emissive_texture[0].size[0] * emissive_texture[0].size[1] / 2;
		}
		s
	}

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
