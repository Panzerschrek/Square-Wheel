use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::material::*;

pub struct GenerativeTextureEffectTurb
{
	turb_params: TurbParams,
	temp_buffer: Vec<i32>,
}

impl GenerativeTextureEffectTurb
{
	pub fn new(turb_params: TurbParams) -> Self
	{
		Self {
			turb_params,
			temp_buffer: Vec::new(),
		}
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
		out_texture_data: &mut GenerativeTextureData,
		texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		current_time_s: f32,
	)
	{
		for mip_index in 0 .. NUM_MIPS
		{
			let src_mip = &texture_data.texture[mip_index];
			let dst_mip = &mut out_texture_data.texture[mip_index];
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
				&mut self.temp_buffer,
			);
		}

		if let Some(emissive_texture) = &texture_data.emissive_texture
		{
			for mip_index in 0 .. NUM_MIPS
			{
				let src_mip = &emissive_texture[mip_index];
				let dst_mip = &mut out_texture_data.emissive_texture[mip_index];
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
					&mut self.temp_buffer,
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
	temp_buffer: &mut Vec<i32>,
)
{
	let mip_scale = 1.0 / ((1 << mip) as f32);
	let amplitude_corrected = mip_scale * turb.amplitude;
	let frequency_scaled = std::f32::consts::TAU / (turb.wave_length * mip_scale);
	let time_based_shift = current_time_s * turb.frequency * std::f32::consts::TAU;

	// Precalculate shifts.
	temp_buffer.resize(std::cmp::max(size[0], size[1]) as usize, 0);
	for (dst_shift, x) in temp_buffer.iter_mut().zip(0 .. size[0])
	{
		// TODO - speed-up this. Use unsafe f32 -> i32 conversion.
		*dst_shift =
			(f32_mul_add(x as f32, frequency_scaled, time_based_shift).sin() * amplitude_corrected).round() as i32;
	}

	debug_assert!((size[0] & (size[0] - 1)) == 0);
	debug_assert!((size[1] & (size[1] - 1)) == 0);
	let mask = [size[0] - 1, size[1] - 1];

	// Perform copy with shifts along both axes.
	// This is a fastest way to perform turb transformation.
	// Other ways, for example with shift along X and that along Y, are significantly slower.
	for y in 0 .. size[1]
	{
		let dst_row = &mut dst_pixels[(y * size[0]) as usize .. ((y + 1) * size[0]) as usize];

		let row_shift = unsafe { debug_only_checked_fetch(temp_buffer, y as usize) };
		for x in 0 .. size[0]
		{
			unsafe {
				let column_shift = debug_only_checked_fetch(temp_buffer, x as usize);
				let u = x + row_shift;
				let v = y + column_shift;
				debug_only_checked_write(
					dst_row,
					x as usize,
					debug_only_checked_fetch(src_pixels, ((u & mask[0]) + (v & mask[1]) * size[0]) as usize),
				);
			}
		}
	}
}
