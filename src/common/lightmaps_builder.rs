use super::{bsp_map_compact, map_file, math_types::*};

pub fn build_lightmaps(map: &mut bsp_map_compact::BSPMap)
{
	let lights = extract_map_lights(map);
}

struct PointLight
{
	pos: Vec3f,
	color: [f32; 3], // Color scaled by intensity.
}

fn extract_map_lights(map: &bsp_map_compact::BSPMap) -> Vec<PointLight>
{
	let mut result = Vec::new();

	for entity in &map.entities
	{
		let mut is_light_entity = false;
		let mut origin = None;
		let mut intensity = None;
		let mut color = None;

		// Parse Quake-style lights.
		// TODO - support directional lights.

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
				if let Ok(o) = map_file::parse_vec3(value)
				{
					origin = Some(o);
				}
			}
			if key.starts_with("light") || key == "_light"
			{
				if let Ok(i) = map_file::parse_number(&mut value.clone())
				{
					intensity = Some(i);
				}
			}
			if key == "color"
			{
				if let Ok(c) = map_file::parse_vec3(value)
				{
					color = Some(c);
				}
			}
		}

		if is_light_entity
		{
			if let Some(pos) = origin
			{
				let intensity = intensity.unwrap_or(300.0).max(0.0);
				let mut out_color = [intensity, intensity, intensity];
				if let Some(color) = color
				{
					out_color[0] *= (color.x / 255.0).max(0.0).min(1.0);
					out_color[1] *= (color.y / 255.0).max(0.0).min(1.0);
					out_color[2] *= (color.z / 255.0).max(0.0).min(1.0);
				}

				if out_color[0] > 0.0 || out_color[1] > 0.0 || out_color[2] > 0.0
				{
					result.push(PointLight { pos, color: out_color });
				}
			}
		}
	}

	result
}
