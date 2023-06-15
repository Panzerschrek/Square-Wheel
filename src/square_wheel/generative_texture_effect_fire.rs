use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, fixed_math::*, material_fire::*, math_types::*};

pub struct GenerativeTextureEffectFire
{
	// Corrected config value.
	update_frequency: f32,
	fire_effect: FireEffect,
	heat_map: Vec<HeatMapElemement>,
	palette: Palette,
	update_step: u32,
	particles: Vec<Particle>,
}

impl GenerativeTextureEffectFire
{
	pub fn new(fire_effect: FireEffect) -> Self
	{
		if fire_effect.resolution_log2[0] < 2 || fire_effect.resolution_log2[1] < 2
		{
			panic!("Fire texture must have size at least 4x4!");
		}

		let area = 1 << (fire_effect.resolution_log2[0] + fire_effect.resolution_log2[1]);
		if area >= (1 << 22)
		{
			panic!("Fire texture is too big!");
		}

		Self {
			update_frequency: fire_effect.update_frequency.max(1.0).min(200.0),
			fire_effect,
			heat_map: vec![0; area],
			palette: [Color32::black(); 256],
			update_step: 0,
			particles: Vec::with_capacity(MAX_PARTICLES),
		}
	}

	fn step_update_heat_map(&mut self)
	{
		self.update_step += 1;

		let size = [
			1 << self.fire_effect.resolution_log2[0],
			1 << self.fire_effect.resolution_log2[1],
		];
		let size_mask = [size[0] - 1, size[1] - 1];
		let v_shift = self.fire_effect.resolution_log2[0];

		let heat_map = &mut self.heat_map;
		let mut set_heat = |x, y, heat: f32| {
			let address = ((x & size_mask[0]) + ((y & size_mask[1]) << v_shift)) as usize;
			// TODO - use unchecked to int conversion.
			heat_map[address] = std::cmp::max(
				heat_map[address],
				(heat * 255.0).max(0.0).min(255.0) as HeatMapElemement,
			)
		};

		for heat_source in &self.fire_effect.heat_sources
		{
			match heat_source
			{
				HeatSource::ConstantPoint { center, heat } => set_heat(center[0], center[1], *heat),
				HeatSource::ConstantLine { points, heat } =>
				{
					// Perform simple line reasterization (with integer coords of points).
					let mut points = *points;
					let abs_dx = ((points[0][0] as i32) - (points[1][0] as i32)).abs();
					let abs_dy = ((points[0][1] as i32) - (points[1][1] as i32)).abs();
					if abs_dx >= abs_dy
					{
						if points[0][0] > points[1][0]
						{
							points.swap(0, 1);
						}
						let y_step = int_to_fixed16((points[1][1] as i32) - (points[0][1] as i32)) / abs_dx;
						let mut y_fract = int_to_fixed16(points[0][1] as i32);
						for x_offset in 0 ..= abs_dx as i32
						{
							let x = (points[0][0] as i32) + x_offset;
							let y = fixed16_round_to_int(y_fract);
							y_fract += y_step;
							set_heat(x as u32, y as u32, *heat);
						}
					}
					else
					{
						if points[0][1] > points[1][1]
						{
							points.swap(0, 1);
						}
						let x_step = int_to_fixed16((points[1][0] as i32) - (points[0][0] as i32)) / abs_dy;
						let mut x_fract = int_to_fixed16(points[0][0] as i32);
						for y_offset in 1 ..= abs_dy as i32
						{
							let y = (points[0][1] as i32) + y_offset;
							let x = fixed16_round_to_int(x_fract);
							x_fract += x_step;
							set_heat(x as u32, y as u32, *heat);
						}
					}
				},
				HeatSource::Fountain {
					center,
					frequency,
					heat,
				} =>
				{
					let inv_frequency_int = (self.update_frequency / frequency).max(1.0) as u32;
					if self.update_step % inv_frequency_int == 0
					{
						if self.particles.len() < MAX_PARTICLES
						{
							self.particles.push(Particle {
								position: Vec2f::new(center[0] as f32, center[1] as f32),
								velocity: Vec2f::new(10.0, 20.0) / self.update_frequency,
								despawn_time: self.update_step + 30,
								heat: *heat,
							});
						}
					}
				},
			}
		}

		// Process particles
		// Can't use for-loop here, because particle can be despawned.
		let mut i = 0;
		while i < self.particles.len()
		{
			let particle = &mut self.particles[i];
			if particle.despawn_time <= self.update_step
			{
				self.particles.swap_remove(i);
				// Skip incrementing counter.
				continue;
			}

			particle.position += particle.velocity;

			set_heat(
				particle.position.x.floor() as i32 as u32,
				particle.position.y.floor() as i32 as u32,
				particle.heat,
			);

			// No particle was removed - advance
			i += 1;
		}

		let attenuation = 1.0 - 1.0 / self.fire_effect.heat_conductivity.max(1.0).min(1000000.0);
		update_heat_map(size, &mut self.heat_map, attenuation);
	}
}

impl GenerativeTextureEffect for GenerativeTextureEffectFire
{
	fn get_estimated_texel_count(&self, _texture_data: &MapTextureData, _all_textures_data: &[MapTextureData]) -> u32
	{
		// TODO
		0
	}

	fn update(
		&mut self,
		out_texture_data: &mut GenerativeTextureData,
		_texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		_current_time_s: f32,
	)
	{
		self.step_update_heat_map();

		if out_texture_data.emissive_texture[0].pixels.is_empty()
		{
			// First update - generate palette.
			// TODO - use emissive image for it.
			for i in 0 .. 256
			{
				self.palette[i] = Color32::from_rgb(i as u8, i as u8, i as u8);
			}
		}

		let size = [
			1 << self.fire_effect.resolution_log2[0],
			1 << self.fire_effect.resolution_log2[1],
		];

		generate_emissive_texture_based_on_heat_map(
			size,
			&self.heat_map,
			&self.palette,
			&mut out_texture_data.emissive_texture[0],
		);

		// Generate mips.
		// TODO - maybe reduce frequency of mips update?
		for i in 1 .. NUM_MIPS
		{
			let (s0, s1) = out_texture_data.emissive_texture.split_at_mut(i);
			build_texture_lite_mip(&mut s1[0], &s0[i - 1]);
		}
	}
}

struct Particle
{
	velocity: Vec2f,   // Pixels/step
	position: Vec2f,   // Pixels
	despawn_time: u32, // in ticks
	// TODO - add also gravity.
	heat: f32, // TODO - store byte value instead
}

const MAX_PARTICLES: usize = 256;

type HeatMapElemement = u8;

fn update_heat_map(size: [u32; 2], heat_map: &mut [HeatMapElemement], attenuation: f32)
{
	debug_assert!(size[0] >= 4);
	debug_assert!(size[1] >= 4);
	debug_assert!(heat_map.len() == (size[0] * size[1]) as usize);

	const SHIFT: u32 = 20;
	let attenuation_i = (attenuation.max(0.0).min(1.0) * ((1 << (SHIFT - 2)) as f32)) as u32;

	let mut update_func = |offset, offset_y_plus_x_minus, offset_y_plus, offset_y_plus_x_plus, offset_y_plus_plus| unsafe {
		let sum = (debug_only_checked_fetch(heat_map, offset_y_plus_x_minus as usize) as u32) +
			(debug_only_checked_fetch(heat_map, offset_y_plus as usize) as u32) +
			(debug_only_checked_fetch(heat_map, offset_y_plus_x_plus as usize) as u32) +
			(debug_only_checked_fetch(heat_map, offset_y_plus_plus as usize) as u32);
		debug_only_checked_write(heat_map, offset as usize, ((sum * attenuation_i) >> SHIFT) as u8);
	};

	for y in 0 .. size[1] - 2
	{
		let line_start = y * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - left border.
		update_func(
			line_start,
			line_end_minus_one + size[0],
			line_start + size[0],
			line_start + 1 + size[0],
			line_start + size[0] * 2,
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1 + size[0], x + size[0], x + 1 + size[0], x + size[0] * 2);
		}

		// Special case - right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1 + size[0],
			line_end_minus_one + size[0],
			line_start + size[0],
			line_end_minus_one + size[0] * 2,
		);
	}

	{
		// Special case - line before last.
		// Clamp y + 2 to y + 1.
		let line_start = (size[1] - 2) * size[0];
		let line_start_plus_one = line_start + 1;
		let line_end_minus_one = line_start + size[0] - 1;

		// Special case - left border.
		update_func(
			line_start,
			line_end_minus_one + size[0],
			line_start + size[0],
			line_start + 1 + size[0],
			line_start + size[0],
		);

		for x in line_start_plus_one .. line_end_minus_one
		{
			update_func(x, x - 1 + size[0], x + size[0], x + 1 + size[0], x + size[0]);
		}

		// Special case - right border.
		update_func(
			line_end_minus_one,
			line_end_minus_one - 1 + size[0],
			line_end_minus_one + size[0],
			line_start + size[0],
			line_end_minus_one + size[0],
		);
	}

	// TODO - handle last line.
}

type Palette = [Color32; 256];

fn generate_emissive_texture_based_on_heat_map(
	size: [u32; 2],
	heat_map: &[HeatMapElemement],
	palette: &Palette,
	texture: &mut TextureLite,
)
{
	texture.size = size;
	texture.pixels.resize((size[0] * size[1]) as usize, Color32::black());

	for (dst_texel, &heat_value) in texture.pixels.iter_mut().zip(heat_map)
	{
		// TODO - use unchecked palette fetch.
		*dst_texel = palette[heat_value as usize]
	}
}
