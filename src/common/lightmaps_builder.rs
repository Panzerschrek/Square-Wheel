use super::{bsp_map_compact, map_file, math_types::*};

pub fn build_lightmaps(map: &mut bsp_map_compact::BSPMap)
{
	let lights = extract_map_lights(map);
	allocate_lightmaps(map);
	test_fill_lightmaps(map);
}

// If this chaged, map file version must be changed too!
pub const LIGHTMAP_SCALE_LOG2: u32 = 4;
pub const LIGHTMAP_SCALE: u32 = 1 << LIGHTMAP_SCALE_LOG2;

pub fn get_polygon_lightmap_size(polygon: &bsp_map_compact::Polygon) -> [u32; 2]
{
	[
		get_lightmap_size(polygon.tex_coord_min[0], polygon.tex_coord_max[0]),
		get_lightmap_size(polygon.tex_coord_min[1], polygon.tex_coord_max[1]),
	]
}

fn get_lightmap_size(tc_min: i32, tc_max: i32) -> u32
{
	// If this chaged, map file version must be changed too!
	debug_assert!(tc_min < tc_max);
	(((tc_max - tc_min) as u32) + (LIGHTMAP_SCALE - 1)) / LIGHTMAP_SCALE + 1
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

fn allocate_lightmaps(map: &mut bsp_map_compact::BSPMap)
{
	let mut offset = 0;
	for polygon in &mut map.polygons
	{
		let size = get_polygon_lightmap_size(polygon);
		polygon.lightmap_data_offset = offset as u32;
		offset += (size[0] * size[1]) as usize;
	}

	map.lightmaps_data.clear();
	map.lightmaps_data.resize(offset, [0.0, 0.0, 0.0]);

	println!("Lightmap texels: {}", offset);
}

fn test_fill_lightmaps(map: &mut bsp_map_compact::BSPMap)
{
	for polygon in &map.polygons
	{
		let size = get_polygon_lightmap_size(polygon);
		for v in 0 .. size[1]
		{
			for u in 0 .. size[0]
			{
				let r = (u as f32) / 8.0;
				let g = (v as f32) / 8.0;
				map.lightmaps_data[(polygon.lightmap_data_offset + u + v * size[0]) as usize] = [r, g, 0.1];
			}
		}
	}
}
