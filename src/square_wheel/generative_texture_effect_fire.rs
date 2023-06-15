use super::{fast_math::*, map_materials_processor_structs::*, textures::*};
use crate::common::{color::*, fixed_math::*, material_fire::*, math_types::*};
use rand::{Rng, SeedableRng};

pub struct GenerativeTextureEffectFire
{
	// Corrected config value.
	update_frequency: f32,
	fire_effect: FireEffect,
	heat_map: Vec<HeatMapElemement>,
	palette: Palette,
	update_step: u32,
	// Use random for some heat sources.
	rand_engine: RandEngine,
	particles: Vec<Particle>,
	rand_buffer: Vec<Fixed16>,
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

		// Fill dummy/backup palette.
		let mut palette = [Color32::black(); 256];
		for i in 0 .. 256
		{
			palette[i] = Color32::from_rgb(i as u8, i as u8, i as u8);
		}

		Self {
			update_frequency: fire_effect.update_frequency.max(1.0).min(200.0),
			fire_effect,
			heat_map: vec![0; area],
			palette,
			update_step: 0,
			// Initialize random engine generator with good, but deterministic value.
			rand_engine: RandEngine::seed_from_u64(0b1001100000111010100101010101010111000111010110100101111001010101),
			rand_buffer: Vec::new(),
			particles: Vec::with_capacity(MAX_PARTICLES),
		}
	}

	fn step_update_heat_map(&mut self)
	{
		self.update_step += 1;
		let time_s = (self.update_step as f32) / self.update_frequency;

		let size = [
			1 << self.fire_effect.resolution_log2[0],
			1 << self.fire_effect.resolution_log2[1],
		];

		// Update heat map after setting heat sources.
		// Doing so ve avoid blurring sharp heat sources.
		let attenuation = 1.0 - 1.0 / self.fire_effect.heat_conductivity.max(1.0).min(1000000.0);
		update_heat_map(size, &mut self.heat_map, attenuation);

		let size_mask = [size[0] - 1, size[1] - 1];
		let v_shift = self.fire_effect.resolution_log2[0];

		let heat_map = &mut self.heat_map;
		let mut set_heat = |x, y, heat| {
			let address = ((x & size_mask[0]) + ((y & size_mask[1]) << v_shift)) as usize;
			let value = unsafe { debug_only_checked_access_mut(heat_map, address) };
			*value = std::cmp::max(*value, heat)
		};

		let get_value_with_random_deviation = |rand_engine: &mut RandEngine, v: &ValueWithRandomDeviation| {
			let value = v.value.evaluate(time_s);
			let deviation = v.random_deviation.evaluate(time_s);
			value - deviation + rand_engine.gen_range(0.0 ..= deviation.max(0.0) * 2.0)
		};

		let rand_engine = &mut self.rand_engine;

		for heat_source in &self.fire_effect.heat_sources
		{
			match heat_source
			{
				HeatSource::Point { center, offset, heat } =>
				{
					let offset_x = get_value_with_random_deviation(rand_engine, &offset[0]);
					let offset_y = get_value_with_random_deviation(rand_engine, &offset[1]);
					set_heat(
						(center[0] as f32 + offset_x).floor() as i32 as u32,
						(center[1] as f32 + offset_y).floor() as i32 as u32,
						convert_heat(get_value_with_random_deviation(rand_engine, heat)),
					)
				},
				HeatSource::Line { points, heat } =>
				{
					let heat = convert_heat(get_value_with_random_deviation(rand_engine, heat));

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
							set_heat(x as u32, y as u32, heat);
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
							set_heat(x as u32, y as u32, heat);
						}
					}
				},
				HeatSource::Lightning { points, heat } =>
				{
					let heat = convert_heat(get_value_with_random_deviation(rand_engine, heat));

					// Perform line reasterization (with integer coords of points), shift coordinates by random value.
					let mut points = *points;
					let abs_dx = ((points[0][0] as i32) - (points[1][0] as i32)).abs();
					let abs_dy = ((points[0][1] as i32) - (points[1][1] as i32)).abs();

					self.rand_buffer.resize((std::cmp::max(abs_dx, abs_dy) + 1) as usize, 0);
					let mut rand_offset = 0;
					for rand_val in &mut self.rand_buffer
					{
						// use deviation no more than one - in order to avoid gaps in lightning.
						// TODO - use faster random?
						*rand_val = rand_offset + rand_engine.gen_range(-FIXED16_ONE ..= FIXED16_ONE);
						rand_offset = *rand_val;
					}

					if abs_dx >= abs_dy
					{
						if points[0][0] > points[1][0]
						{
							points.swap(0, 1);
						}
						let y_step =
							(int_to_fixed16((points[1][1] as i32) - (points[0][1] as i32)) - rand_offset) / abs_dx;
						let mut y_fract = int_to_fixed16(points[0][1] as i32);
						for x_offset in 0 ..= abs_dx as i32
						{
							let x = (points[0][0] as i32) + x_offset;
							let y_add = unsafe { debug_only_checked_fetch(&self.rand_buffer, x_offset as usize) };
							let y = fixed16_round_to_int(y_fract + y_add);
							y_fract += y_step;
							set_heat(x as u32, y as u32, heat);
						}
					}
					else
					{
						if points[0][1] > points[1][1]
						{
							points.swap(0, 1);
						}
						let x_step =
							(int_to_fixed16((points[1][0] as i32) - (points[0][0] as i32)) - rand_offset) / abs_dy;
						let mut x_fract = int_to_fixed16(points[0][0] as i32);
						for y_offset in 1 ..= abs_dy as i32
						{
							let y = (points[0][1] as i32) + y_offset;
							let x_add = unsafe { debug_only_checked_fetch(&self.rand_buffer, y_offset as usize) };
							let x = fixed16_round_to_int(x_fract + x_add);
							x_fract += x_step;
							set_heat(x as u32, y as u32, heat);
						}
					}
				},
				HeatSource::Fountain {
					center,
					frequency,
					heat,
					angle,
					speed,
					gravity,
					spawn_angle,
					spawn_distance,
					lifetime,
				} =>
				{
					let inv_frequency_int = (self.update_frequency / frequency).max(1.0) as u32;
					if self.update_step % inv_frequency_int == 0
					{
						if self.particles.len() < MAX_PARTICLES
						{
							let heat = get_value_with_random_deviation(rand_engine, heat);
							let angle = get_value_with_random_deviation(rand_engine, angle);
							let speed = get_value_with_random_deviation(rand_engine, speed);
							let gravity = get_value_with_random_deviation(rand_engine, gravity);
							let spawn_angle = get_value_with_random_deviation(rand_engine, spawn_angle);
							let spawn_distance = get_value_with_random_deviation(rand_engine, spawn_distance);
							let lifetime = get_value_with_random_deviation(rand_engine, lifetime);

							let velocity = Vec2f::new(angle.cos(), angle.sin()) * (speed / self.update_frequency);
							let spawn_vec = Vec2f::new(spawn_angle.cos(), spawn_angle.sin()) * spawn_distance;
							let lifetime = (lifetime * self.update_frequency).max(1.0).min(1024.0) as u32;

							self.particles.push(Particle {
								position: Vec2f::new(center[0] as f32, center[1] as f32) + spawn_vec,
								velocity,
								despawn_time: self.update_step + lifetime,
								gravity: gravity / (self.update_frequency * self.update_frequency),
								heat: convert_heat(heat),
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

			particle.velocity.y += particle.gravity;
			particle.position += particle.velocity;

			set_heat(
				particle.position.x.floor() as i32 as u32,
				particle.position.y.floor() as i32 as u32,
				particle.heat,
			);

			// No particle was removed - advance
			i += 1;
		}
	}
}

type RandEngine = rand::rngs::SmallRng;

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
		texture_data: &MapTextureData,
		_all_textures_data: &[MapTextureData],
		_textures_mapping_table: &[TextureMappingElement],
		_current_time_s: f32,
	)
	{
		self.step_update_heat_map();

		if out_texture_data.emissive_texture[0].pixels.is_empty()
		{
			// First update - generate palette.
			if let Some(color) = self.fire_effect.color
			{
				// use specified color for palette generation.
				for i in 0 .. 256
				{
					let scale = i as f32;
					self.palette[i] = Color32::from_rgb(
						(color[0] * scale).max(0.0).min(255.0) as u8,
						(color[1] * scale).max(0.0).min(255.0) as u8,
						(color[2] * scale).max(0.0).min(255.0) as u8,
					);
				}
			}
			else if let Some(emissive_texture) = &texture_data.emissive_texture
			{
				if !emissive_texture[0].pixels.is_empty()
				{
					// Use emissive layer texture as palette.
					for i in 0 .. 256
					{
						self.palette[i as usize] =
							emissive_texture[0].pixels[((i * emissive_texture[0].size[0]) >> 8) as usize];
					}
				}
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
	gravity: f32,
	heat: HeatMapElemement,
}

const MAX_PARTICLES: usize = 256;

type HeatMapElemement = u8;

// Convert normalized float into byte value and clamp.
fn convert_heat(heat: f32) -> HeatMapElemement
{
	(heat * 255.0).max(0.0).min(255.0) as HeatMapElemement
}

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

	// Zero last line.
	for value in &mut heat_map[((size[1] - 1) * size[0]) as usize .. (size[1] * size[0]) as usize]
	{
		*value = 0;
	}
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
		*dst_texel = unsafe { debug_only_checked_fetch(palette, heat_value as usize) };
	}
}
