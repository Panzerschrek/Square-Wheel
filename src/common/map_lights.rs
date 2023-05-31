use super::{bbox::*, bsp_map_compact, map_file_common, math_types::*};
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub struct PointLight
{
	pub pos: Vec3f,
	pub color: [f32; 3], // Color scaled by intensity.
	// Non-empty for spot lights.
	pub direction: Option<LightDirection>,
}

#[derive(Copy, Clone)]
pub struct LightDirection
{
	pub dir_normalized: Vec3f,
	pub half_angle_cos: f32,
}

pub struct SunLight
{
	// Direction towards light, scaled by map size so for every point in map point + dir will be outside map.
	pub dir: Vec3f,
	pub color: [f32; 3],
}

pub fn extract_map_lights(map: &bsp_map_compact::BSPMap) -> Vec<PointLight>
{
	let mut result = Vec::new();

	let mut named_targets = HashMap::new();
	for entity in &map.entities
	{
		let mut origin = None;
		let mut targetname = None;

		for key_value_pair in &map.key_value_pairs[entity.first_key_value_pair as usize ..
			(entity.first_key_value_pair + entity.num_key_value_pairs) as usize]
		{
			let key = bsp_map_compact::get_map_string(key_value_pair.key, map);
			let value = bsp_map_compact::get_map_string(key_value_pair.value, map);
			if key == "origin"
			{
				if let Ok(o) = map_file_common::parse_vec3(value)
				{
					origin = Some(o);
				}
			}
			if key == "targetname"
			{
				targetname = Some(value);
			}
		}

		if let (Some(origin), Some(targetname)) = (origin, targetname)
		{
			named_targets.insert(targetname, origin);
		}
	}

	for entity in &map.entities
	{
		let mut is_light_entity = false;
		let mut origin = None;
		let mut intensity = None;
		let mut color = None;
		let mut target = None;
		let mut angle = None;

		// Parse Quake-style lights.

		for key_value_pair in &map.key_value_pairs[(entity.first_key_value_pair as usize) ..
			((entity.first_key_value_pair + entity.num_key_value_pairs) as usize)]
		{
			let key = bsp_map_compact::get_map_string(key_value_pair.key, map);
			let value = bsp_map_compact::get_map_string(key_value_pair.value, map);
			if key == "classname" && value.starts_with("light")
			{
				is_light_entity = true;
			}
			if key == "origin"
			{
				if let Ok(o) = map_file_common::parse_vec3(value)
				{
					origin = Some(o);
				}
			}
			if key.starts_with("light") || key == "_light"
			{
				if let Ok(i) = map_file_common::parse_key_value_number(value)
				{
					intensity = Some(i);
				}
			}
			if key == "color"
			{
				if let Ok(c) = map_file_common::parse_vec3(value)
				{
					color = Some(c);
				}
			}
			if key == "target"
			{
				target = Some(value);
			}
			if key == "angle"
			{
				if let Ok(a) = map_file_common::parse_key_value_number(value)
				{
					angle = Some(a);
				}
			}
		}

		if is_light_entity
		{
			if let Some(pos) = origin
			{
				let intensity = intensity.unwrap_or(300.0).max(0.0) * MAP_LIGHTS_SCALE;
				let mut out_color = [intensity, intensity, intensity];
				if let Some(color) = color
				{
					out_color[0] *= (color.x / 255.0).max(0.0).min(1.0);
					out_color[1] *= (color.y / 255.0).max(0.0).min(1.0);
					out_color[2] *= (color.z / 255.0).max(0.0).min(1.0);
				}

				let mut direction = None;
				if let Some(target) = target
				{
					if let Some(target_origin) = named_targets.get(target)
					{
						let dir = target_origin - pos;
						let dir_len = dir.magnitude();
						if dir_len > 0.0
						{
							direction = Some(LightDirection {
								dir_normalized: dir / dir_len,
								half_angle_cos: (angle.unwrap_or(40.0) * (0.5 * std::f32::consts::PI / 180.0)).cos(),
							});
						}
					}
				};

				if out_color[0] > 0.0 || out_color[1] > 0.0 || out_color[2] > 0.0
				{
					result.push(PointLight {
						pos,
						color: out_color,
						direction,
					});
				}
			}
		}
	}

	result
}

pub fn extract_sun_lights(map: &bsp_map_compact::BSPMap, map_bbox: &BBox) -> Vec<SunLight>
{
	let bbox_size = map_bbox.get_size();
	let bbox_max_dimension = bbox_size.magnitude();

	let world_entity = map.entities[0];

	let mut sun_angles: Option<Vec3f> = None;
	let mut intensity: Option<f32> = None;
	let mut color: Option<Vec3f> = None;

	// https://ericwa.github.io/ericw-tools/doc/light.html#MODEL%20ENTITY%20KEYS

	for key_value_pair in &map.key_value_pairs[world_entity.first_key_value_pair as usize ..
		(world_entity.first_key_value_pair + world_entity.num_key_value_pairs) as usize]
	{
		let key = bsp_map_compact::get_map_string(key_value_pair.key, map);
		let value = bsp_map_compact::get_map_string(key_value_pair.value, map);
		if key == "_sun_mangle"
		{
			if let Ok(a) = map_file_common::parse_vec3(value)
			{
				sun_angles = Some(a);
			}
		}
		if key == "_sunlight"
		{
			if let Ok(i) = map_file_common::parse_key_value_number(value)
			{
				intensity = Some(i);
			}
		}
		if key == "_sunlight_color"
		{
			if let Ok(c) = map_file_common::parse_vec3(value)
			{
				color = Some(c);
			}
		}
	}

	if let Some(intensity) = intensity
	{
		let intensity = intensity.max(0.0) * MAP_LIGHTS_SCALE;
		let mut out_color = [intensity, intensity, intensity];
		if let Some(color) = color
		{
			out_color[0] *= (color.x / 255.0).max(0.0).min(1.0);
			out_color[1] *= (color.y / 255.0).max(0.0).min(1.0);
			out_color[2] *= (color.z / 255.0).max(0.0).min(1.0);
		}

		if out_color[0] > 0.0 || out_color[1] > 0.0 || out_color[2] > 0.0
		{
			let dir = if let Some(sun_angles) = sun_angles
			{
				let deg2rad = std::f32::consts::PI / 180.0;
				// TODO - check yaw usage.
				let yaw = sun_angles.x * deg2rad;
				let pitch = sun_angles.y * deg2rad;
				let pitch_cos = pitch.cos();
				-Vec3f::new(pitch_cos * yaw.cos(), pitch_cos * yaw.sin(), pitch.sin())
			}
			else
			{
				Vec3f::unit_z()
			};

			let dir_scaled = dir * bbox_max_dimension;

			return vec![SunLight {
				dir: dir_scaled,
				color: out_color,
			}];
		}
	}

	Vec::new()
}

const MAP_LIGHTS_SCALE: f32 = 32.0;
