use super::{bsp_map_compact, map_file_common, math_types::*};
use std::io::Write;

pub struct LightmappingSettings
{
	pub sample_grid_size: u32,
	pub light_scale: f32,
}

pub fn build_lightmaps(settings: &LightmappingSettings, map: &mut bsp_map_compact::BSPMap)
{
	let sample_grid_size = settings.sample_grid_size.min(MAX_SAMPLE_GRID_SIZE);

	let mut lights = extract_map_lights(map);
	println!("Point lights: {}", lights.len());

	for l in &mut lights
	{
		l.color[0] *= settings.light_scale;
		l.color[1] *= settings.light_scale;
		l.color[2] *= settings.light_scale;
	}

	allocate_lightmaps(map);
	println!("Lightmap texels: {}", map.lightmaps_data.len());

	test_fill_lightmaps(map);

	build_primary_lightmaps(sample_grid_size, &lights, map);

	println!("\nDone!");
}

// If this chaged, map file version must be changed too!
pub const LIGHTMAP_SCALE_LOG2: u32 = 4;
pub const LIGHTMAP_SCALE: u32 = 1 << LIGHTMAP_SCALE_LOG2;

// Limit used for polygons splitting.
// Actual size may be a little big greater.
pub const MAX_LIGHTMAP_SIZE: u32 = 17;

pub fn get_polygon_lightmap_size(polygon: &bsp_map_compact::Polygon) -> [u32; 2]
{
	[
		get_lightmap_size(polygon.tex_coord_min[0], polygon.tex_coord_max[0]),
		get_lightmap_size(polygon.tex_coord_min[1], polygon.tex_coord_max[1]),
	]
}

pub fn get_lightmap_size(tc_min: i32, tc_max: i32) -> u32
{
	// If this chaged, map file version must be changed too!
	debug_assert!(tc_min < tc_max);
	let result =
		((tc_max + ((LIGHTMAP_SCALE - 1) as i32) >> LIGHTMAP_SCALE_LOG2) - (tc_min >> LIGHTMAP_SCALE_LOG2) + 1) as u32;
	debug_assert!(result >= 2);
	result
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
				if let Ok(o) = map_file_common::parse_vec3(value)
				{
					origin = Some(o);
				}
			}
			if key.starts_with("light") || key == "_light"
			{
				if let Ok(i) = map_file_common::parse_number(&mut value.clone())
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

fn build_primary_lightmaps(sample_grid_size: u32, lights: &[PointLight], map: &mut bsp_map_compact::BSPMap)
{
	let mut texels_complete = 0;
	let texels_total = map.lightmaps_data.len();
	for i in 0 .. map.polygons.len()
	{
		build_primary_lightmap(sample_grid_size, lights, i, map);

		// Calculate and show progress.
		let lightmap_size = get_polygon_lightmap_size(&map.polygons[i]);
		let lightmap_texels = (lightmap_size[0] * lightmap_size[1]) as usize;

		let ratio_before = texels_complete * 256 / texels_total;
		texels_complete += lightmap_texels;
		let ratio_after = texels_complete * 256 / texels_total;
		if ratio_after > ratio_before
		{
			print!(
				"\r{:03.2}% complete ({} of {} texels),  {} of {} polygons",
				(texels_complete as f32) * 100.0 / (texels_total as f32),
				texels_complete,
				texels_total,
				i,
				map.polygons.len()
			);
			let _ignore_errors = std::io::stdout().flush();
		}
	}
}

fn build_primary_lightmap(
	sample_grid_size: u32,
	lights: &[PointLight],
	polygon_index: usize,
	map: &mut bsp_map_compact::BSPMap,
)
{
	let polygon = &map.polygons[polygon_index];
	let lightmap_size = get_polygon_lightmap_size(polygon);

	const TEXEL_NORMAL_SHIFT: f32 = 1.0 / 16.0;
	let plane_normal_normalized = polygon.plane.vec / polygon.plane.vec.magnitude();

	let mut polygon_vertices_average = Vec3f::new(0.0, 0.0, 0.0);
	for &v in &map.vertices[polygon.first_vertex as usize .. (polygon.first_vertex + polygon.num_vertices) as usize]
	{
		polygon_vertices_average += v;
	}
	let polygon_center =
		polygon_vertices_average / (polygon.num_vertices as f32) + TEXEL_NORMAL_SHIFT * plane_normal_normalized;

	// Calculate inverse matrix for tex_coord equation and plane equation in order to calculate world position for UV.
	let tc_basis_scale = 1.0 / (LIGHTMAP_SCALE as f32);
	let tex_coord_basis = Mat4f::from_cols(
		polygon.tex_coord_equation[0]
			.vec
			.extend(polygon.tex_coord_equation[0].dist) *
			tc_basis_scale,
		polygon.tex_coord_equation[1]
			.vec
			.extend(polygon.tex_coord_equation[1].dist) *
			tc_basis_scale,
		polygon.plane.vec.extend(-polygon.plane.dist),
		Vec4f::new(0.0, 0.0, 0.0, 1.0),
	);
	let tex_coord_basis_inverted = tex_coord_basis.transpose().invert().unwrap(); // TODO - avoid "unwrap"?

	let u_vec = tex_coord_basis_inverted.x.truncate();
	let v_vec = tex_coord_basis_inverted.y.truncate();
	// Shift pos slightly towards direction of normal to avoid self-shadowing artifacts.
	let start_pos = tex_coord_basis_inverted.w.truncate() +
		u_vec * ((polygon.tex_coord_min[0] >> LIGHTMAP_SCALE_LOG2) as f32) +
		v_vec * ((polygon.tex_coord_min[1] >> LIGHTMAP_SCALE_LOG2) as f32) +
		plane_normal_normalized * TEXEL_NORMAL_SHIFT;

	// Prepare sample grid shifts.
	let mut sample_grid = [Vec3f::zero(); (MAX_SAMPLE_GRID_SIZE * MAX_SAMPLE_GRID_SIZE) as usize];
	if sample_grid_size > 1
	{
		let grid_size_f = sample_grid_size as f32;
		let u_step = u_vec / grid_size_f;
		let v_step = v_vec / grid_size_f;
		let grid_start = (-0.5 * (grid_size_f - 1.0)) * (v_step + u_step);
		for v in 0 .. sample_grid_size
		{
			let v_vec = grid_start + (v as f32) * v_step;
			for u in 0 .. sample_grid_size
			{
				let vec = (u as f32) * u_step + v_vec;
				sample_grid[(u + v * sample_grid_size) as usize] = vec;
			}
		}
	}
	let num_sample_grid_samples = (sample_grid_size * sample_grid_size) as usize;
	let multi_sampling_scale = 1.0 / (num_sample_grid_samples as f32);

	for v in 0 .. lightmap_size[1]
	{
		let start_pos_v = start_pos + (v as f32) * v_vec;
		let line_dst_start = polygon.lightmap_data_offset + v * lightmap_size[0];
		for u in 0 .. lightmap_size[0]
		{
			let mut total_light = [0.0, 0.0, 0.0];
			let texel_pos = start_pos_v + (u as f32) * u_vec;
			// Calculate light for multiple samples withing current texel, than use average value.
			// This allow us to get (reltively) soft shadows.
			for &sample_shift in &sample_grid[.. num_sample_grid_samples]
			{
				let mut pos = texel_pos + sample_shift;
				// Correct texel position if can't see from texel to polygon center.
				// TODO - improve this. Fix cases where texel position is exactly on some polygon plane.
				for i in 0 .. 16
				{
					if can_see(&pos, &polygon_center, map)
					{
						break;
					}
					if i < 4
					{
						// Special cases - shift postion along U/V axis for texels on border.
						if u == 0
						{
							pos += 0.5 * u_vec;
						}
						if u == lightmap_size[0] - 1
						{
							pos -= 0.5 * u_vec;
						}
						if v == 0
						{
							pos += 0.5 * v_vec;
						}
						if v == lightmap_size[1] - 1
						{
							pos -= 0.5 * v_vec;
						}
					}
					else
					{
						// Hard case - shift towards center.
						let vec_to_center = polygon_center - pos;
						let vec_to_center_len = vec_to_center.magnitude().max(MIN_POSITIVE_VALUE);
						let vec_to_center_normalized = vec_to_center / vec_to_center_len;
						pos +=
							vec_to_center_normalized * u_vec.magnitude().max(v_vec.magnitude()).min(vec_to_center_len);
					}
				}

				for light in lights
				{
					let vec_to_light = light.pos - pos;
					let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);
					let angle_cos = plane_normal_normalized.dot(vec_to_light) / vec_to_light_len2.sqrt();

					if angle_cos <= 0.0
					{
						// Do not determine visibility for light behind polygon plane.
						continue;
					}

					let light_scale = angle_cos / vec_to_light_len2;
					let color_scaled = [
						light.color[0] * light_scale,
						light.color[1] * light_scale,
						light.color[2] * light_scale,
					];

					if color_scaled[0].max(color_scaled[1]).max(color_scaled[2]) <= MIN_LIGHT_VALUE
					{
						// Light value is too small. Do not perform shadow check.
						// This check allows us to significantly reduce light computation time by skipping shadow check for distant lights.
						continue;
					}

					if !can_see(&light.pos, &pos, map)
					{
						// In shadow.
						continue;
					}

					total_light[0] += multi_sampling_scale * color_scaled[0];
					total_light[1] += multi_sampling_scale * color_scaled[1];
					total_light[2] += multi_sampling_scale * color_scaled[2];
				}
			}

			map.lightmaps_data[(u + line_dst_start) as usize] = total_light;
		}
	}
}

const MIN_POSITIVE_VALUE: f32 = 1.0 / ((1 << 30) as f32);
const MAP_LIGHTS_SCALE: f32 = 32.0; // TODO - tune this.
const MIN_LIGHT_VALUE: f32 = 1.0 / 256.0; // TODO - tune this.
const MAX_SAMPLE_GRID_SIZE: u32 = 8;

fn can_see(from: &Vec3f, to: &Vec3f, map: &bsp_map_compact::BSPMap) -> bool
{
	let root_node = (map.nodes.len() - 1) as u32;
	can_see_r(from, to, root_node, map)
	// TODO - check intersection with submodel polygons?
}

// Speed-up intersection calculation - recursively determine loction of check edge withing BSP tree.
// Than check only leafs where edge is actually located.
fn can_see_r(v0: &Vec3f, v1: &Vec3f, current_index: u32, map: &bsp_map_compact::BSPMap) -> bool
{
	if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
	{
		let leaf_index = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
		let leaf = &map.leafs[leaf_index as usize];
		for i in 0 .. leaf.num_polygons
		{
			if edge_intersects_with_polygon(v0, v1, (leaf.first_polygon + i) as usize, map)
			{
				return false;
			}
		}
		return true;
	}
	else
	{
		let node = &map.nodes[current_index as usize];
		let dist0 = v0.dot(node.plane.vec) - node.plane.dist;
		let dist1 = v1.dot(node.plane.vec) - node.plane.dist;
		if dist0 >= 0.0 && dist1 >= 0.0
		{
			return can_see_r(v0, v1, node.children[0], map);
		}
		if dist0 <= 0.0 && dist1 <= 0.0
		{
			return can_see_r(v0, v1, node.children[1], map);
		}

		// Split edge using BSP node plane.

		let dist_sum = dist1 - dist0;
		if dist_sum.abs() < MIN_POSITIVE_VALUE
		{
			// Edge is almost on polygon plane.
			return true;
		}
		let k0 = dist0 / dist_sum;
		let k1 = dist1 / dist_sum;
		let intersection_pos = v0 * k1 - v1 * k0;

		let (v_front, v_back) = if dist0 > 0.0 { (v0, v1) } else { (v1, v0) };

		// HACK!
		// There is some problems with intersection detection if intersection polygon plane is same as BSP plane.
		// So, extend edge a little bit behind splitter plane.
		let eps = 1.0 / 1024.0;
		let intersection_pos_front = intersection_pos * (1.0 - eps) + v_back * eps;
		let intersection_pos_back = intersection_pos * (1.0 - eps) + v_front * eps;

		if !can_see_r(v_front, &intersection_pos_front, node.children[0], map)
		{
			return false;
		}
		if !can_see_r(&intersection_pos_back, v_back, node.children[1], map)
		{
			return false;
		}

		return true;
	}
}

fn edge_intersects_with_polygon(v0: &Vec3f, v1: &Vec3f, polygon_index: usize, map: &bsp_map_compact::BSPMap) -> bool
{
	let polygon = &map.polygons[polygon_index];
	let plane = &polygon.plane;

	let dist0 = v0.dot(plane.vec) - plane.dist;
	let dist1 = v1.dot(plane.vec) - plane.dist;
	if dist0.signum() == dist1.signum()
	{
		// Edge is located at one side of polygon plane.
		return false;
	}
	let dist_sum = dist1 - dist0;
	if dist_sum.abs() < MIN_POSITIVE_VALUE
	{
		// Edge is almost on polygon plane.
		return false;
	}
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	let intersection_pos = v0 * k1 - v1 * k0;

	// TODO - extend polygon just a bit, to prevent light leakage between adjusted polygons.
	for i in 0 .. polygon.num_vertices
	{
		let v = map.vertices[(polygon.first_vertex + i) as usize];
		let next_v = map.vertices[(polygon.first_vertex + (i + 1) % polygon.num_vertices) as usize];
		let edge_vec = next_v - v;
		let vec_to_instersection_pos = intersection_pos - v;
		let cross = vec_to_instersection_pos.cross(edge_vec);
		let normal_dot = cross.dot(plane.vec);
		if normal_dot < 0.0
		{
			return false;
		}
	}

	true
}