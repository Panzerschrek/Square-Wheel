use super::{
	bsp_map_compact, image, light_hemisphere::*, lightmap::*, map_file_common, material, math_types::*, pvs,
	shared_mut_slice::*,
};
use rayon::prelude::*;
use std::{io::Write, sync::atomic};

pub struct LightmappingSettings
{
	pub sample_grid_size: u32,
	pub light_scale: f32,
	pub ambient_light: f32,
	pub save_primary_light: bool,
	pub save_secondary_light: bool,
	pub build_emissive_surfaces_light: bool,
	pub build_directional_lightmap: bool,
	pub num_passes: u32,
	pub light_grid_cell_width: f32,
	pub light_grid_cell_height: f32,
}

pub fn build_lightmaps<AlbedoImageGetter: FnMut(&str) -> Option<image::Image>>(
	settings: &LightmappingSettings,
	materials: &material::MaterialsMap,
	map: &mut bsp_map_compact::BSPMap,
	albedo_image_getter: AlbedoImageGetter,
)
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

	let lights_by_leaf = group_lights_by_leafs(map, &lights);

	let mut primary_lightmaps_data = allocate_lightmaps(materials, map);
	println!("Lightmap texels: {}", primary_lightmaps_data.len());

	let visibility_matrix = pvs::calculate_visibility_matrix(&map);

	build_primary_lightmaps(
		sample_grid_size,
		&lights_by_leaf,
		map,
		&visibility_matrix,
		&mut primary_lightmaps_data,
	);

	let materials_albedo = get_map_textures_albedo(map, albedo_image_getter);
	let emissive_light = get_map_textures_emissive_light(map, materials, settings);

	let mut emissive_light_sources = Vec::new();
	if settings.build_emissive_surfaces_light
	{
		emissive_light_sources =
			create_emissive_surfaces_light_sources(&emissive_light, map, primary_lightmaps_data.len());

		let mut emissive_surfaces_lightmaps_data = vec![[0.0, 0.0, 0.0]; primary_lightmaps_data.len()];

		build_secondary_lightmaps(
			&emissive_light_sources,
			map,
			&visibility_matrix,
			&mut emissive_surfaces_lightmaps_data,
		);

		// Add emissive surfaces lightmap to primary lightap in order to generate secondary light for emissive surfaces lights.
		for (dst, src) in primary_lightmaps_data
			.iter_mut()
			.zip(emissive_surfaces_lightmaps_data.iter())
		{
			for i in 0 .. 3
			{
				dst[i] += src[i];
			}
		}
	}

	let mut passes_lightmaps = vec![primary_lightmaps_data];

	if settings.save_secondary_light && settings.num_passes > 1
	{
		for _pass_num in 1 .. settings.num_passes.min(8)
		{
			let prev_pass_lightmap = passes_lightmaps.last().unwrap();
			let secondary_light_sources = create_secondary_light_sources(&materials_albedo, map, &prev_pass_lightmap);

			let mut secondary_lightmaps_data = vec![[0.0, 0.0, 0.0]; prev_pass_lightmap.len()];

			build_secondary_lightmaps(
				&secondary_light_sources,
				map,
				&visibility_matrix,
				&mut secondary_lightmaps_data,
			);

			passes_lightmaps.push(secondary_lightmaps_data);
		}
	}

	println!("Combining lightmaps");
	let primary_lightmap = passes_lightmaps.first().unwrap();
	let secondary_lightmaps = &passes_lightmaps[1 ..];

	map.lightmaps_data =
		vec![[settings.ambient_light, settings.ambient_light, settings.ambient_light]; primary_lightmap.len()];
	let primary_light_scale = if settings.save_primary_light { 1.0 } else { 0.0 };
	let secondary_light_scale = if settings.save_secondary_light { 1.0 } else { 0.0 };

	for i in 0 .. map.lightmaps_data.len()
	{
		let dst = &mut map.lightmaps_data[i];

		for j in 0 .. 3
		{
			dst[j] += primary_lightmap[i][j] * primary_light_scale;
		}
		for lightmap in secondary_lightmaps
		{
			let src = &lightmap[i];
			for j in 0 .. 3
			{
				dst[j] += src[j] * secondary_light_scale;
			}
		}
	}

	// Build directional lightmaps and light grid using initial lights and secondary light sources based on combined lightmap.
	let secondary_light_sources = if settings.save_secondary_light
	{
		create_secondary_light_sources(&materials_albedo, map, &map.lightmaps_data)
	}
	else
	{
		Vec::new()
	};

	if settings.build_directional_lightmap
	{
		let mut directional_lightmaps_data = vec![
			bsp_map_compact::DirectionalLightmapElement {
				ambient_light: [0.0, 0.0, 0.0],
				light_direction_vector_scaled: Vec3f::zero(),
				directional_light_deviation: 0.0,
				directional_light_color: [0.0, 0.0, 0.0]
			};
			primary_lightmap.len()
		];

		build_directional_lightmaps(
			sample_grid_size,
			&lights_by_leaf,
			&secondary_light_sources,
			&emissive_light_sources,
			map,
			&visibility_matrix,
			&mut directional_lightmaps_data,
		);

		map.directional_lightmaps_data = directional_lightmaps_data;
	}
	else
	{
		map.directional_lightmaps_data = Vec::new();
	}

	prepare_light_grid(map, settings);

	let light_grid_uncompressed = calculate_light_grid(
		sample_grid_size,
		&lights_by_leaf,
		&secondary_light_sources,
		&emissive_light_sources,
		map,
		&visibility_matrix,
	);
	let (light_grid_columns, light_grid_samples) = compress_light_grid(map, &light_grid_uncompressed);
	map.light_grid_columns = light_grid_columns;
	map.light_grid_samples = light_grid_samples;

	println!("Done!");
}

type LightsByLeaf = Vec<Vec<PointLight>>;

fn group_lights_by_leafs(map: &bsp_map_compact::BSPMap, light_sources: &[PointLight]) -> LightsByLeaf
{
	let mut lights_by_leaf = vec![Vec::new(); map.leafs.len()];

	for light_source in light_sources
	{
		let leaf_index = bsp_map_compact::get_leaf_for_point(map, &light_source.pos);
		lights_by_leaf[leaf_index as usize].push(*light_source);
	} // for light sources

	lights_by_leaf
}

fn get_map_textures_emissive_light(
	map: &bsp_map_compact::BSPMap,
	materials: &material::MaterialsMap,
	settings: &LightmappingSettings,
) -> Vec<[f32; 3]>
{
	let mut emissive_light = vec![[0.0, 0.0, 0.0]; map.textures.len()];
	for (dst_light, texture_name) in emissive_light.iter_mut().zip(map.textures.iter())
	{
		if let Some(material) = materials.get(bsp_map_compact::get_texture_string(texture_name))
		{
			for i in 0 .. 3
			{
				dst_light[i] = material.emissive_light[i] * settings.light_scale;
			}
		}
	}
	emissive_light
}

fn get_map_textures_albedo<AlbedoImageGetter: FnMut(&str) -> Option<image::Image>>(
	map: &bsp_map_compact::BSPMap,
	mut albedo_image_getter: AlbedoImageGetter,
) -> Vec<MaterialAlbedo>
{
	let mut materials_albedo = vec![DEFAULT_ALBEDO; map.textures.len()];
	// Load textures in order to know albedo.
	for (dst_albedo, texture_name) in materials_albedo.iter_mut().zip(map.textures.iter())
	{
		let texture_str = bsp_map_compact::get_texture_string(texture_name);
		if let Some(img) = albedo_image_getter(texture_str)
		{
			let mut pixels_sum: [u32; 3] = [0, 0, 0];
			for pixel in &img.pixels
			{
				let rgb = pixel.get_rgb();
				pixels_sum[0] += rgb[0] as u32;
				pixels_sum[1] += rgb[1] as u32;
				pixels_sum[2] += rgb[2] as u32;
			}
			let scale = 1.0 / (img.pixels.len() as f32 * 255.0);
			*dst_albedo = [
				pixels_sum[0] as f32 * scale,
				pixels_sum[1] as f32 * scale,
				pixels_sum[2] as f32 * scale,
			];
		}
		else
		{
			println!("Can't load texture for material {}", texture_str);
		}
	}
	materials_albedo
}

#[derive(Copy, Clone)]
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

pub type LightmapsData = Vec<bsp_map_compact::LightmapElement>;

pub fn allocate_lightmaps(materials: &material::MaterialsMap, map: &mut bsp_map_compact::BSPMap) -> LightmapsData
{
	// Reserve offset=0 as "no lightmap" flag.
	let mut offset = 1;

	for polygon in &mut map.polygons
	{
		let has_lightmap = if let Some(material) = materials.get(bsp_map_compact::get_texture_string(
			&map.textures[polygon.texture as usize],
		))
		{
			material.light
		}
		else
		{
			true
		};

		if has_lightmap
		{
			let size = get_polygon_lightmap_size(polygon);
			polygon.lightmap_data_offset = offset as u32;
			offset += (size[0] * size[1]) as usize;
		}
		else
		{
			polygon.lightmap_data_offset = 0;
		}
	}

	vec![[0.0, 0.0, 0.0]; offset]
}

struct ProgressTracker
{
	texels_total: u32,
	polygons_total: u32,
	texels_complete: atomic::AtomicU32,
	polygons_complete: atomic::AtomicU32,
}

impl ProgressTracker
{
	fn new(map: &bsp_map_compact::BSPMap, texels_total: usize, message: &str) -> Self
	{
		println!("{}", message);

		Self {
			texels_total: texels_total as u32,
			polygons_total: map.polygons.len() as u32,
			texels_complete: atomic::AtomicU32::new(0),
			polygons_complete: atomic::AtomicU32::new(0),
		}
	}

	fn process_polygon(&self, polygon: &bsp_map_compact::Polygon)
	{
		let lightmap_size = get_polygon_lightmap_size(polygon);
		let lightmap_texels = lightmap_size[0] * lightmap_size[1];

		let texels_complete_before = self
			.texels_complete
			.fetch_add(lightmap_texels, atomic::Ordering::SeqCst);
		let texels_complete_afrer = texels_complete_before + lightmap_texels;
		let ratio_before = texels_complete_before * 256 / self.texels_total;
		let ratio_after = texels_complete_afrer * 256 / self.texels_total;
		let polygons_before = self.polygons_complete.fetch_add(1, atomic::Ordering::SeqCst);
		if ratio_after > ratio_before
		{
			print!(
				"\r{:03.2}% complete ({} of {} texels),  {} of {} polygons",
				(texels_complete_afrer as f32) * 100.0 / (self.texels_total as f32),
				texels_complete_afrer,
				self.texels_total,
				polygons_before + 1,
				self.polygons_total
			);
			let _ignore_errors = std::io::stdout().flush();
		}
	}
}

impl Drop for ProgressTracker
{
	fn drop(&mut self)
	{
		println!("\nDone!");
	}
}

type MaterialAlbedo = [f32; 3];

fn build_primary_lightmaps(
	sample_grid_size: u32,
	lights: &LightsByLeaf,
	map: &bsp_map_compact::BSPMap,
	visibility_matrix: &pvs::VisibilityMatrix,
	lightmaps_data: &mut [bsp_map_compact::LightmapElement],
)
{
	let progress_tracker = ProgressTracker::new(map, lightmaps_data.len(), "Building primary lightmaps");

	// It is safe to share lightmaps data across threads since each polygons uses its own region.
	let lightmaps_data_shared = SharedMutSlice::new(lightmaps_data);

	map.leafs.par_iter().enumerate().for_each(|(leaf_index, leaf)| {
		let lightmaps_data_unshared = unsafe { lightmaps_data_shared.get() };

		let visible_leafs_list = get_visible_leafs(map, leaf_index, visibility_matrix);
		let visible_lights_list = get_visible_lights(lights, &visible_leafs_list);

		for polygon_index in leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize
		{
			let polygon = &map.polygons[polygon_index];
			if polygon.lightmap_data_offset == 0
			{
				// No lightmap for this polygon.
				continue;
			}
			build_primary_lightmap(
				sample_grid_size,
				&visible_lights_list,
				polygon,
				map,
				lightmaps_data_unshared,
			);

			progress_tracker.process_polygon(polygon);
		} // for leaf polygons.
	});

	map.submodels.par_iter().for_each(|submodel| {
		let lightmaps_data_unshared = unsafe { lightmaps_data_shared.get() };

		let visible_leafs_list = get_visible_leafs_for_submodel(map, submodel, visibility_matrix);
		let visible_lights_list = get_visible_lights(lights, &visible_leafs_list);

		for polygon_index in
			submodel.first_polygon as usize .. (submodel.first_polygon + submodel.num_polygons) as usize
		{
			let polygon = &map.polygons[polygon_index];
			if polygon.lightmap_data_offset == 0
			{
				// No lightmap for this polygon.
				continue;
			}
			build_primary_lightmap(
				sample_grid_size,
				&visible_lights_list,
				polygon,
				map,
				lightmaps_data_unshared,
			);

			progress_tracker.process_polygon(&map.polygons[polygon_index]);
		} // for submodel polygons.
	}); // for submodels
}

fn build_primary_lightmap(
	sample_grid_size: u32,
	lights: &[PointLight],
	polygon: &bsp_map_compact::Polygon,
	map: &bsp_map_compact::BSPMap,
	lightmaps_data: &mut [bsp_map_compact::LightmapElement],
)
{
	let lightmap_size = get_polygon_lightmap_size(polygon);

	let plane_normal_normalized = polygon.plane.vec / polygon.plane.vec.magnitude();

	let polygon_center = get_polygon_center(map, polygon) + TEXEL_NORMAL_SHIFT * plane_normal_normalized;

	let lightmap_basis = calculate_lightmap_basis(polygon);

	// Shift pos slightly towards direction of normal to avoid self-shadowing artifacts.
	let start_pos = lightmap_basis.pos + plane_normal_normalized * TEXEL_NORMAL_SHIFT;

	// Prepare sample grid shifts.
	let mut sample_grid = [Vec3f::zero(); (MAX_SAMPLE_GRID_SIZE * MAX_SAMPLE_GRID_SIZE) as usize];
	if sample_grid_size > 1
	{
		let grid_size_f = sample_grid_size as f32;
		let u_step = lightmap_basis.u_vec / grid_size_f;
		let v_step = lightmap_basis.v_vec / grid_size_f;
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
		let start_pos_v = start_pos + (v as f32) * lightmap_basis.v_vec;
		let line_dst_start = polygon.lightmap_data_offset + v * lightmap_size[0];
		for u in 0 .. lightmap_size[0]
		{
			let mut total_light = [0.0, 0.0, 0.0];
			let texel_pos = start_pos_v + (u as f32) * lightmap_basis.u_vec;
			// Calculate light for multiple samples withing current texel, than use average value.
			// This allow us to get (reltively) soft shadows.
			for &sample_shift in &sample_grid[.. num_sample_grid_samples]
			{
				let pos = correct_sample_position(map, &(texel_pos + sample_shift), &lightmap_basis, &polygon_center);
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

					if !can_see(&light.pos, &pos, map)
					{
						// In shadow.
						continue;
					}

					let light_scale = angle_cos / vec_to_light_len2;
					let color_scaled = [
						light.color[0] * light_scale,
						light.color[1] * light_scale,
						light.color[2] * light_scale,
					];

					total_light[0] += multi_sampling_scale * color_scaled[0];
					total_light[1] += multi_sampling_scale * color_scaled[1];
					total_light[2] += multi_sampling_scale * color_scaled[2];
				}
			}

			lightmaps_data[(u + line_dst_start) as usize] = total_light;
		}
	}
}

fn build_secondary_lightmaps(
	lights: &[SecondaryLightSource],
	map: &bsp_map_compact::BSPMap,
	visibility_matrix: &pvs::VisibilityMatrix,
	lightmaps_data: &mut [bsp_map_compact::LightmapElement],
)
{
	let progress_tracker = ProgressTracker::new(map, lightmaps_data.len(), "Building secondary lightmaps");

	// It is safe to share lightmaps data across threads since each polygons uses its own region.
	let lightmaps_data_shared = SharedMutSlice::new(lightmaps_data);

	map.leafs.par_iter().enumerate().for_each(|(leaf_index, leaf)| {
		let lightmaps_data_unshared = unsafe { lightmaps_data_shared.get() };

		let visible_leafs_list = get_visible_leafs(map, leaf_index, visibility_matrix);

		for polygon_index in leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize
		{
			if map.polygons[polygon_index].lightmap_data_offset == 0
			{
				// No lightmap for this polygon.
				continue;
			}
			build_polygon_secondary_lightmap(lights, polygon_index, map, &visible_leafs_list, lightmaps_data_unshared);

			progress_tracker.process_polygon(&map.polygons[polygon_index]);
		} // for leaf polygons.
	});

	map.submodels.par_iter().for_each(|submodel| {
		let lightmaps_data_unshared = unsafe { lightmaps_data_shared.get() };

		let visible_leafs_list = get_visible_leafs_for_submodel(map, submodel, visibility_matrix);

		for polygon_index in
			submodel.first_polygon as usize .. (submodel.first_polygon + submodel.num_polygons) as usize
		{
			if map.polygons[polygon_index].lightmap_data_offset == 0
			{
				// No lightmap for this polygon.
				continue;
			}
			build_polygon_secondary_lightmap(lights, polygon_index, map, &visible_leafs_list, lightmaps_data_unshared);

			progress_tracker.process_polygon(&map.polygons[polygon_index]);
		} // for submodel polygons.
	}); // for submodels
}

fn get_visible_lights(lights: &LightsByLeaf, visible_leafs: &[u32]) -> Vec<PointLight>
{
	let mut visible_lights_list = Vec::new();
	for &leaf_index in visible_leafs
	{
		for light in &lights[leaf_index as usize]
		{
			visible_lights_list.push(*light);
		}
	}
	visible_lights_list
}

fn get_visible_leafs(
	map: &bsp_map_compact::BSPMap,
	leaf_index: usize,
	visibility_matrix: &pvs::VisibilityMatrix,
) -> Vec<u32>
{
	let visibility_matrix_row = &visibility_matrix[leaf_index * map.leafs.len() .. (leaf_index + 1) * map.leafs.len()];

	let mut visible_leafs_list = Vec::new();
	for other_leaf_index in 0 .. map.leafs.len()
	{
		if visibility_matrix_row[other_leaf_index]
		{
			visible_leafs_list.push(other_leaf_index as u32);
		}
	}
	visible_leafs_list
}

fn get_visible_leafs_for_submodel(
	map: &bsp_map_compact::BSPMap,
	submodel: &bsp_map_compact::Submodel,
	visibility_matrix: &pvs::VisibilityMatrix,
) -> Vec<u32>
{
	// Know in which leafs this submodel is located.
	let bbox = bsp_map_compact::get_submodel_bbox(map, submodel);
	let bbox_vertices = [
		Vec3f::new(bbox.min.x, bbox.min.y, bbox.min.z),
		Vec3f::new(bbox.min.x, bbox.min.y, bbox.max.z),
		Vec3f::new(bbox.min.x, bbox.max.y, bbox.min.z),
		Vec3f::new(bbox.min.x, bbox.max.y, bbox.max.z),
		Vec3f::new(bbox.max.x, bbox.min.y, bbox.min.z),
		Vec3f::new(bbox.max.x, bbox.min.y, bbox.max.z),
		Vec3f::new(bbox.max.x, bbox.max.y, bbox.min.z),
		Vec3f::new(bbox.max.x, bbox.max.y, bbox.max.z),
	];

	let mut submodel_leafs_list = Vec::new();
	let root_node = (map.nodes.len() - 1) as u32;
	collect_submodel_leafs_r(map, &bbox_vertices, root_node, &mut submodel_leafs_list);

	// Know which leafs are visible for submodel's leafs.
	let mut submodel_visible_leafs_bit_set = vec![false; map.leafs.len()];
	for &leaf_index in &submodel_leafs_list
	{
		let visibility_matrix_row = &visibility_matrix
			[(leaf_index as usize) * map.leafs.len() .. ((leaf_index as usize) + 1) * map.leafs.len()];
		for (src, dst) in visibility_matrix_row
			.iter()
			.zip(submodel_visible_leafs_bit_set.iter_mut())
		{
			*dst |= src;
		}
	}

	let mut visible_leafs_list = Vec::new();
	for other_leaf_index in 0 .. map.leafs.len()
	{
		if submodel_visible_leafs_bit_set[other_leaf_index]
		{
			visible_leafs_list.push(other_leaf_index as u32);
		}
	}
	visible_leafs_list
}

fn collect_submodel_leafs_r(
	map: &bsp_map_compact::BSPMap,
	bbox_vertices: &[Vec3f; 8],
	node_index: u32,
	out_leafs: &mut Vec<u32>,
)
{
	if node_index >= bsp_map_compact::FIRST_LEAF_INDEX
	{
		let leaf_index = node_index - bsp_map_compact::FIRST_LEAF_INDEX;
		out_leafs.push(leaf_index);
	}
	else
	{
		let node = map.nodes[node_index as usize];

		let mut vertices_front = 0;
		for &vertex in bbox_vertices
		{
			if node.plane.vec.dot(vertex) > node.plane.dist
			{
				vertices_front += 1;
			}
		}

		if vertices_front > 0
		{
			collect_submodel_leafs_r(map, bbox_vertices, node.children[0], out_leafs);
		}
		if vertices_front < bbox_vertices.len()
		{
			collect_submodel_leafs_r(map, bbox_vertices, node.children[1], out_leafs);
		}
	}
}

fn build_polygon_secondary_lightmap(
	lights: &[SecondaryLightSource],
	polygon_index: usize,
	map: &bsp_map_compact::BSPMap,
	visible_leafs: &[u32], // Leafs visible for this polygon.
	lightmaps_data: &mut [bsp_map_compact::LightmapElement],
)
{
	let polygon = &map.polygons[polygon_index];
	let lightmap_size = get_polygon_lightmap_size(polygon);

	let plane_normal_normalized = polygon.plane.vec / polygon.plane.vec.magnitude();

	let polygon_center = get_polygon_center(map, polygon) + TEXEL_NORMAL_SHIFT * plane_normal_normalized;

	let lightmap_basis = calculate_lightmap_basis(polygon);

	// Shift pos slightly towards direction of normal to avoid self-shadowing artifacts.
	let start_pos = lightmap_basis.pos + plane_normal_normalized * TEXEL_NORMAL_SHIFT;

	for v in 0 .. lightmap_size[1]
	{
		let start_pos_v = start_pos + (v as f32) * lightmap_basis.v_vec;
		let line_dst_start = polygon.lightmap_data_offset + v * lightmap_size[0];
		for u in 0 .. lightmap_size[0]
		{
			let mut total_light = [0.0, 0.0, 0.0];
			let pos_initial = start_pos_v + (u as f32) * lightmap_basis.u_vec;

			let pos_sihfted_towards_center = pre_correct_secondary_light_sample_position(&pos_initial, &polygon_center);
			let pos = correct_sample_position(map, &pos_sihfted_towards_center, &lightmap_basis, &polygon_center);

			// Calculate light only from polygons in visible leafs.
			for &leaf_index in visible_leafs
			{
				let leaf = &map.leafs[leaf_index as usize];
				for light_source_polygon_index in
					leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize
				{
					if light_source_polygon_index == polygon_index
					{
						// Ignore lights from this polygon.
						continue;
					}

					let light = &lights[light_source_polygon_index];
					if light.samples.is_empty()
					{
						continue;
					}

					// Compute LOD.
					let light_source_lod = get_light_source_lod(&pos, light);
					let min_dist2 =
						get_secondary_light_source_sample_min_square_distance(light.sample_size, light_source_lod);

					// Iterate over all samples of this LOD.
					for sample in &light.samples[light_source_lod]
					{
						let vec_to_light = sample.pos - pos;
						let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);
						let vec_to_light_normalized = vec_to_light / vec_to_light_len2.sqrt();
						let angle_cos = plane_normal_normalized.dot(vec_to_light_normalized);

						if angle_cos <= 0.0
						{
							// Do not determine visibility for light behind polygon plane.
							continue;
						}

						let angle_cos_src = -(light.normal.dot(vec_to_light_normalized));
						if angle_cos_src <= 0.0
						{
							// Do not determine visibility for texels behind light source plane.
							continue;
						}

						if !can_see(&sample.pos, &pos, map)
						{
							// In shadow.
							continue;
						}

						let light_scale = angle_cos * angle_cos_src / vec_to_light_len2.max(min_dist2);
						let color_scaled = [
							sample.color[0] * light_scale,
							sample.color[1] * light_scale,
							sample.color[2] * light_scale,
						];

						total_light[0] += color_scaled[0];
						total_light[1] += color_scaled[1];
						total_light[2] += color_scaled[2];
					} // for light samples.
				} // for leaf polygons.
			} // for leafs.

			lightmaps_data[(u + line_dst_start) as usize] = total_light;
		}
	}
}

fn build_directional_lightmaps(
	sample_grid_size: u32,
	primary_lights: &LightsByLeaf,
	secondary_lights: &[SecondaryLightSource],
	emissive_lights: &[SecondaryLightSource],
	map: &bsp_map_compact::BSPMap,
	visibility_matrix: &pvs::VisibilityMatrix,
	lightmaps_data: &mut [bsp_map_compact::DirectionalLightmapElement],
)
{
	let progress_tracker = ProgressTracker::new(map, lightmaps_data.len(), "Building directional lightmaps");

	// It is safe to share lightmaps data across threads since each polygons uses its own region.
	let lightmaps_data_shared = SharedMutSlice::new(lightmaps_data);

	map.leafs.par_iter().enumerate().for_each(|(leaf_index, leaf)| {
		let lightmaps_data_unshared = unsafe { lightmaps_data_shared.get() };

		let visible_leafs_list = get_visible_leafs(map, leaf_index, visibility_matrix);
		let visible_lights_list = get_visible_lights(primary_lights, &visible_leafs_list);

		for polygon_index in leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize
		{
			if map.polygons[polygon_index].lightmap_data_offset == 0
			{
				// No lightmap for this polygon.
				continue;
			}
			build_polygon_diretional_lightmap(
				sample_grid_size,
				&visible_lights_list,
				secondary_lights,
				emissive_lights,
				polygon_index,
				map,
				&visible_leafs_list,
				lightmaps_data_unshared,
			);

			progress_tracker.process_polygon(&map.polygons[polygon_index]);
		} // for leaf polygons.
	});

	map.submodels.par_iter().for_each(|submodel| {
		let lightmaps_data_unshared = unsafe { lightmaps_data_shared.get() };

		let visible_leafs_list = get_visible_leafs_for_submodel(map, submodel, visibility_matrix);
		let visible_lights_list = get_visible_lights(primary_lights, &visible_leafs_list);

		for polygon_index in
			submodel.first_polygon as usize .. (submodel.first_polygon + submodel.num_polygons) as usize
		{
			if map.polygons[polygon_index].lightmap_data_offset == 0
			{
				// No lightmap for this polygon.
				continue;
			}
			build_polygon_diretional_lightmap(
				sample_grid_size,
				&visible_lights_list,
				secondary_lights,
				emissive_lights,
				polygon_index,
				map,
				&visible_leafs_list,
				lightmaps_data_unshared,
			);

			progress_tracker.process_polygon(&map.polygons[polygon_index]);
		} // for submodel polygons.
	}); // for submodels
}

fn build_polygon_diretional_lightmap(
	sample_grid_size: u32,
	primary_lights: &[PointLight],
	secondary_lights: &[SecondaryLightSource],
	emissive_lights: &[SecondaryLightSource],
	polygon_index: usize,
	map: &bsp_map_compact::BSPMap,
	visible_leafs: &[u32], // Leafs visible for this polygon.
	lightmaps_data: &mut [bsp_map_compact::DirectionalLightmapElement],
)
{
	let polygon = &map.polygons[polygon_index];
	let lightmap_size = get_polygon_lightmap_size(polygon);

	let plane_normal_normalized = polygon.plane.vec / polygon.plane.vec.magnitude();

	let polygon_center = get_polygon_center(map, polygon) + TEXEL_NORMAL_SHIFT * plane_normal_normalized;

	let lightmap_basis = calculate_lightmap_basis(polygon);

	// Shift pos slightly towards direction of normal to avoid self-shadowing artifacts.
	let start_pos = lightmap_basis.pos + plane_normal_normalized * TEXEL_NORMAL_SHIFT;

	// Prepare sample grid shifts.
	let mut sample_grid = [Vec3f::zero(); (MAX_SAMPLE_GRID_SIZE * MAX_SAMPLE_GRID_SIZE) as usize];
	if sample_grid_size > 1
	{
		let grid_size_f = sample_grid_size as f32;
		let u_step = lightmap_basis.u_vec / grid_size_f;
		let v_step = lightmap_basis.v_vec / grid_size_f;
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

	// Use normalized u/v vecs in order to transform direction to light into texel space.
	let u_vec_normalized = lightmap_basis.u_vec / (lightmap_basis.u_vec.magnitude().max(MIN_POSITIVE_VALUE));
	let v_vec_normalized = lightmap_basis.v_vec / (lightmap_basis.v_vec.magnitude().max(MIN_POSITIVE_VALUE));

	for v in 0 .. lightmap_size[1]
	{
		let start_pos_v = start_pos + (v as f32) * lightmap_basis.v_vec;
		let line_dst_start = polygon.lightmap_data_offset + v * lightmap_size[0];
		for u in 0 .. lightmap_size[0]
		{
			let mut light_hemisphere = LightHemisphere::new();

			let texel_pos = start_pos_v + (u as f32) * lightmap_basis.u_vec;
			// Calculate light for multiple samples withing current texel, than use average value.
			// This allow us to get (reltively) soft shadows.
			for &sample_shift in &sample_grid[.. num_sample_grid_samples]
			{
				let pos = correct_sample_position(map, &(texel_pos + sample_shift), &lightmap_basis, &polygon_center);

				for primay_light in primary_lights
				{
					let vec_to_light = primay_light.pos - pos;
					let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);

					if plane_normal_normalized.dot(vec_to_light) <= 0.0
					{
						// Do not determine visibility for light behind polygon plane.
						continue;
					}

					if !can_see(&primay_light.pos, &pos, map)
					{
						// In shadow.
						continue;
					}

					// Do not use agle cos because we add light into light hemisphere.
					let light_scale = multi_sampling_scale / vec_to_light_len2;
					let color_scaled = [
						primay_light.color[0] * light_scale,
						primay_light.color[1] * light_scale,
						primay_light.color[2] * light_scale,
					];

					// Use same direction to light for all grid samples to avoid bluring point lights (make specular sharp).
					let vec_to_light_from_texel_center = primay_light.pos - texel_pos;
					let vec_to_light_transformed = Vec3f::new(
						vec_to_light_from_texel_center.dot(u_vec_normalized),
						vec_to_light_from_texel_center.dot(v_vec_normalized),
						vec_to_light_from_texel_center.dot(plane_normal_normalized),
					);
					light_hemisphere.add_point_light(&vec_to_light_transformed, &color_scaled);
				}
			} // For primary light sample shifts.

			let pos_sihfted_towards_center = pre_correct_secondary_light_sample_position(&texel_pos, &polygon_center);
			let pos = correct_sample_position(map, &pos_sihfted_towards_center, &lightmap_basis, &polygon_center);

			// Calculate light only from polygons in visible leafs.
			for light_set in [secondary_lights, emissive_lights]
			{
				if light_set.is_empty()
				{
					continue;
				}

				for &leaf_index in visible_leafs
				{
					let leaf = &map.leafs[leaf_index as usize];
					for light_source_polygon_index in
						leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize
					{
						if light_source_polygon_index == polygon_index
						{
							// Ignore lights from this polygon.
							continue;
						}

						let light = &light_set[light_source_polygon_index];
						if light.samples.is_empty()
						{
							continue;
						}

						// Compute LOD.
						let light_source_lod = get_light_source_lod(&pos, light);
						let min_dist2 =
							get_secondary_light_source_sample_min_square_distance(light.sample_size, light_source_lod);
						let current_sample_size = ((1 << light_source_lod) as f32) * light.sample_size;

						// Iterate over all samples of this LOD.
						for sample in &light.samples[light_source_lod]
						{
							let vec_to_light = sample.pos - pos;
							let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);
							let vec_to_light_normalized = vec_to_light / vec_to_light_len2.sqrt();
							let normal_dot = plane_normal_normalized.dot(vec_to_light);

							if normal_dot <= 0.0
							{
								// Do not determine visibility for light behind polygon plane.
								continue;
							}

							let angle_cos_src = -(light.normal.dot(vec_to_light_normalized));
							if angle_cos_src <= 0.0
							{
								// Do not determine visibility for texels behind light source plane.
								continue;
							}

							if !can_see(&sample.pos, &pos, map)
							{
								// In shadow.
								continue;
							}

							// Do not use agle cos because we add light into light hemisphere.

							let vec_to_light_len2_clamped = vec_to_light_len2.max(min_dist2);

							let light_scale = angle_cos_src / vec_to_light_len2_clamped;
							let color_scaled = [
								sample.color[0] * light_scale,
								sample.color[1] * light_scale,
								sample.color[2] * light_scale,
							];

							let vec_to_light_transformed = Vec3f::new(
								vec_to_light.dot(u_vec_normalized),
								vec_to_light.dot(v_vec_normalized),
								normal_dot,
							);
							light_hemisphere.add_sized_light(
								&vec_to_light_transformed,
								&color_scaled,
								current_sample_size / vec_to_light_len2_clamped.sqrt(),
							);
						} // for light samples.
					} // for leaf polygons.
				} // for leafs.
			} // For light sets.

			let ambient_light = light_hemisphere.extract_ambient_light();
			let directional_light_params = light_hemisphere.calculate_light_direction();

			let dst_lightmap_element = &mut lightmaps_data[(u + line_dst_start) as usize];
			dst_lightmap_element.ambient_light = ambient_light;
			dst_lightmap_element.directional_light_color = directional_light_params.color;
			dst_lightmap_element.light_direction_vector_scaled = directional_light_params.direction_vector_scaled;
			dst_lightmap_element.directional_light_deviation = directional_light_params.deviation;
		}
	}
}

pub fn get_light_source_lod(point: &Vec3f, light_source: &SecondaryLightSource) -> usize
{
	// Calculate light source lod.
	// Try to achieve target ratio between sample size and distance to closest point of light source (approaximated as circle).
	let closest_distance_to_light = calculate_dinstance_between_point_and_circle(
		&point,
		&light_source.center,
		&light_source.normal,
		light_source.radius,
	)
	.max(MIN_POSITIVE_VALUE);
	let mut sample_lod = 0;
	loop
	{
		let ratio = ((1 << sample_lod) as f32) * light_source.sample_size / closest_distance_to_light;
		if ratio >= MAX_ALLOVED_SAMPLE_SIZE_TO_DISTANCE_RATIO
		{
			break;
		}
		if sample_lod + 1 < light_source.samples.len()
		{
			sample_lod += 1;
		}
		else
		{
			break;
		}
	}

	sample_lod
}

const MIN_POSITIVE_VALUE: f32 = 1.0 / ((1 << 30) as f32);
const MAP_LIGHTS_SCALE: f32 = 32.0; // TODO - tune this.
const MAX_SAMPLE_GRID_SIZE: u32 = 8;
const TEXEL_NORMAL_SHIFT: f32 = 1.0 / 16.0;

// This constant affects light source lod selection.
// It should be less than sin(90/8 deg).
const MAX_ALLOVED_SAMPLE_SIZE_TO_DISTANCE_RATIO: f32 = 0.125;

// Multiply all secondary light sources by this constant in order to achieve light energy saving.
// TODO - check if this is valid constant.
const LIGHT_INTEGRATION_NORMALIZATION_CONSTANT: f32 = 1.0 / std::f32::consts::PI;

// Use pretty dark albedo if texture was not found.
pub const DEFAULT_ALBEDO: MaterialAlbedo = [0.25, 0.25, 0.25];

fn get_polygon_center(map: &bsp_map_compact::BSPMap, polygon: &bsp_map_compact::Polygon) -> Vec3f
{
	// TODO - improve this.
	// Calculate real center (center of mass?), not just average values of all vertices.
	let mut polygon_vertices_average = Vec3f::new(0.0, 0.0, 0.0);
	for &v in &map.vertices[polygon.first_vertex as usize .. (polygon.first_vertex + polygon.num_vertices) as usize]
	{
		polygon_vertices_average += v;
	}

	polygon_vertices_average / (polygon.num_vertices as f32)
}

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

pub type SecondaryLightSources = Vec<SecondaryLightSource>;

pub struct SecondaryLightSource
{
	// Set of sample grid lods.
	pub samples: Vec<Vec<SecondaryLightSourceSample>>,
	pub sample_size: f32, // Linear size of sample.
	pub normal: Vec3f,    // Normalized.
	// Approximate polygon as 2d circle.
	pub center: Vec3f,
	pub radius: f32,
}

pub struct SecondaryLightSourceSample
{
	pub pos: Vec3f,
	pub color: [f32; 3], // Color scaled by intensity.
}

// Secondary light sources are mapped 1 to 1 to source polygons.
// Light sources for polygons withoult lightmap have zero intensity.
pub fn create_secondary_light_sources(
	materials_albedo: &[MaterialAlbedo],
	map: &bsp_map_compact::BSPMap,
	primary_lightmaps_data: &LightmapsData,
) -> SecondaryLightSources
{
	let mut result = Vec::with_capacity(map.polygons.len());
	let mut sample_raster_data = Vec::new();
	for polygon in &map.polygons
	{
		let source = if polygon.lightmap_data_offset == 0
		{
			SecondaryLightSource {
				samples: Vec::new(),
				normal: Vec3f::unit_z(),
				sample_size: 1.0,
				center: Vec3f::zero(),
				radius: 0.0,
			}
		}
		else
		{
			create_secondary_light_source(
				materials_albedo,
				map,
				primary_lightmaps_data,
				polygon,
				&mut sample_raster_data,
			)
		};

		result.push(source);
	}

	result
}

fn create_emissive_surfaces_light_sources(
	materials_emissive_light: &[[f32; 3]],
	map: &bsp_map_compact::BSPMap,
	lightmap_data_size: usize,
) -> SecondaryLightSources
{
	// Use same function for emissive surfaces lights creation as for secondary lights.
	// In order to do this create pseudo-lightmap with all one values and modulate it by emissive light power.
	let all_ones_lightmap = vec![[1.0, 1.0, 1.0]; lightmap_data_size];

	let mut result = Vec::with_capacity(map.polygons.len());
	let mut sample_raster_data = Vec::new();
	for polygon in &map.polygons
	{
		let light = materials_emissive_light[polygon.texture as usize];
		let source = if light[0] <= 0.0 && light[1] <= 0.0 && light[2] <= 0.0
		{
			SecondaryLightSource {
				samples: Vec::new(),
				normal: Vec3f::unit_z(),
				sample_size: 1.0,
				center: Vec3f::zero(),
				radius: 0.0,
			}
		}
		else
		{
			create_secondary_light_source(
				materials_emissive_light,
				map,
				&all_ones_lightmap,
				polygon,
				&mut sample_raster_data,
			)
		};

		result.push(source);
	}

	result
}

type SampleRasterData = Vec<[f32; 3]>;

fn create_secondary_light_source(
	materials_albedo: &[MaterialAlbedo],
	map: &bsp_map_compact::BSPMap,
	primary_lightmaps_data: &LightmapsData,
	polygon: &bsp_map_compact::Polygon,
	sample_raster_data: &mut SampleRasterData,
) -> SecondaryLightSource
{
	let plane_normal_normalized = polygon.plane.vec / polygon.plane.vec.magnitude();

	let lightmap_size = get_polygon_lightmap_size(polygon);
	let lightmap_basis = calculate_lightmap_basis(polygon);

	// Shift pos slightly towards direction of normal to avoid self-shadowing artifacts.
	let start_pos = lightmap_basis.pos + plane_normal_normalized * TEXEL_NORMAL_SHIFT;

	let polygon_vertices =
		&map.vertices[polygon.first_vertex as usize .. (polygon.first_vertex + polygon.num_vertices) as usize];

	// Use constant albedo for whole polygon.
	// TODO - maybe perform per-pixel albedo fetch?
	let polygon_albedo = materials_albedo[polygon.texture as usize];

	const SAMPLE_RASTER_SHIFT: u32 = 3;
	const SAMPLE_RASTER_SIZE: u32 = 1 << SAMPLE_RASTER_SHIFT;
	const SAMPLE_RASTER_MASK: u32 = SAMPLE_RASTER_SIZE - 1;
	const INV_SAMPLE_RASTER_SIZE: f32 = 1.0 / (SAMPLE_RASTER_SIZE as f32);

	let sample_grid_size = [lightmap_size[0] - 1, lightmap_size[1] - 1];
	let sample_raster_size = [
		sample_grid_size[0] << SAMPLE_RASTER_SHIFT,
		sample_grid_size[1] << SAMPLE_RASTER_SHIFT,
	];

	// Prepare sample raster.
	{
		sample_raster_data.resize(
			(sample_raster_size[0] * sample_raster_size[1]) as usize,
			[0.0, 0.0, 0.0],
		);

		let lightmap_data = &primary_lightmaps_data[polygon.lightmap_data_offset as usize ..
			(polygon.lightmap_data_offset + lightmap_size[0] * lightmap_size[1]) as usize];

		let raster_u_vec = lightmap_basis.u_vec * INV_SAMPLE_RASTER_SIZE;
		let raster_v_vec = lightmap_basis.v_vec * INV_SAMPLE_RASTER_SIZE;
		let raster_start_pos = start_pos + 0.5 * (raster_u_vec + raster_v_vec);
		for v in 0 .. sample_raster_size[1]
		{
			let start_pos_v = raster_start_pos + (v as f32) * raster_v_vec;
			for u in 0 .. sample_raster_size[0]
			{
				let pos = start_pos_v + (u as f32) * raster_u_vec;

				// Check if sample is inside polygon. Ignore samples outside polygons.
				let mut inside_polygon = true;
				for i in 0 .. polygon.num_vertices
				{
					let v0 = polygon_vertices[i as usize];
					let v1 = polygon_vertices[((i + 1) % polygon.num_vertices) as usize];
					let edge_vec = v0 - v1;
					let vec = pos - v0;
					let cross = edge_vec.cross(vec);
					let normal_dot = plane_normal_normalized.dot(cross);
					if normal_dot < 0.0
					{
						inside_polygon = false;
						break;
					}
				}
				let dst = &mut sample_raster_data[(u + v * sample_raster_size[0]) as usize];
				if inside_polygon
				{
					// Perform interpolated lightmap fetch.
					let lightmap_u = u >> SAMPLE_RASTER_SHIFT;
					let lightmap_v = v >> SAMPLE_RASTER_SHIFT;
					let lightmap_u_plus_one = lightmap_u + 1;
					let lightmap_v_plus_one = lightmap_v + 1;
					debug_assert!(lightmap_u_plus_one < lightmap_size[0]);
					debug_assert!(lightmap_v_plus_one < lightmap_size[1]);

					let lightmap00 = lightmap_data[(lightmap_u + lightmap_v * lightmap_size[0]) as usize];
					let lightmap01 = lightmap_data[(lightmap_u + lightmap_v_plus_one * lightmap_size[0]) as usize];
					let lightmap10 = lightmap_data[(lightmap_u_plus_one + lightmap_v * lightmap_size[0]) as usize];
					let lightmap11 =
						lightmap_data[(lightmap_u_plus_one + lightmap_v_plus_one * lightmap_size[0]) as usize];

					let k_u = (0.5 + ((u & SAMPLE_RASTER_MASK) as f32)) * INV_SAMPLE_RASTER_SIZE;
					let k_v = (0.5 + ((v & SAMPLE_RASTER_MASK) as f32)) * INV_SAMPLE_RASTER_SIZE;
					let one_minus_k_u = 1.0 - k_u;
					let one_minus_k_v = 1.0 - k_v;

					let mut result = [0.0, 0.0, 0.0];
					for i in 0 .. 3
					{
						let light0 = lightmap00[i] * one_minus_k_v + lightmap01[i] * k_v;
						let light1 = lightmap10[i] * one_minus_k_v + lightmap11[i] * k_v;
						result[i] = (light0 * one_minus_k_u + light1 * k_u) * polygon_albedo[i];
					}
					*dst = result;
				}
				else
				{
					*dst = [0.0, 0.0, 0.0];
				}
			} // for u
		} // for v
	}

	let polygon_center = get_polygon_center(map, polygon);
	let polygon_center_normal_shifted = polygon_center + plane_normal_normalized * TEXEL_NORMAL_SHIFT;

	let texel_area = lightmap_basis.u_vec.cross(lightmap_basis.v_vec).magnitude();

	// Resample raster, make sample grid lods.
	let color_scale =
		(INV_SAMPLE_RASTER_SIZE * INV_SAMPLE_RASTER_SIZE * LIGHT_INTEGRATION_NORMALIZATION_CONSTANT) * texel_area;
	let mut cur_sample_grid_size = sample_grid_size;
	let mut cur_sample_raster_shift = SAMPLE_RASTER_SHIFT;
	let mut samples_lods = Vec::new();
	loop
	{
		let cur_sample_raster_size = 1 << cur_sample_raster_shift;
		let cur_basis_vecs_scale = (1 << (cur_sample_raster_shift - SAMPLE_RASTER_SHIFT)) as f32;
		let cur_u_vec = lightmap_basis.u_vec * cur_basis_vecs_scale;
		let cur_v_vec = lightmap_basis.v_vec * cur_basis_vecs_scale;

		let mut samples = Vec::new();
		for v in 0 .. cur_sample_grid_size[1]
		{
			let start_pos_v = start_pos + ((v as f32) + 0.5) * cur_v_vec;
			for u in 0 .. cur_sample_grid_size[0]
			{
				let mut color = [0.0, 0.0, 0.0];
				let pixel_start_u = u << cur_sample_raster_shift;
				let pixel_start_v = v << cur_sample_raster_shift;
				for dv in 0 .. cur_sample_raster_size
				{
					for du in 0 .. cur_sample_raster_size
					{
						let src_u = pixel_start_u + du;
						let src_v = pixel_start_v + dv;
						if src_u < sample_raster_size[0] && src_v < sample_raster_size[1]
						{
							let pixel = sample_raster_data[(src_u + src_v * sample_raster_size[0]) as usize];
							for i in 0 .. 3
							{
								color[i] += pixel[i];
							}
						}
					} // for du
				} // for dv
				if color[0] <= 0.0 && color[1] <= 0.0 && color[2] <= 0.0
				{
					continue;
				}
				for i in 0 .. 3
				{
					color[i] *= color_scale;
				}

				let pos = start_pos_v + ((u as f32) + 0.5) * cur_u_vec;
				let pos_corrected = correct_sample_position(map, &pos, &lightmap_basis, &polygon_center_normal_shifted);

				samples.push(SecondaryLightSourceSample {
					pos: pos_corrected,
					color,
				});
			} // for u
		} // for v

		samples_lods.push(samples);
		if cur_sample_grid_size[0] == 1 && cur_sample_grid_size[1] == 1
		{
			break;
		}
		cur_sample_grid_size[0] = (cur_sample_grid_size[0] + 1) >> 1;
		cur_sample_grid_size[1] = (cur_sample_grid_size[1] + 1) >> 1;
		cur_sample_raster_shift += 1;
	} // For sample grid lods.

	// Length of lightmap texel diagonal.
	let sample_size = (lightmap_basis.u_vec + lightmap_basis.v_vec).magnitude();

	// Calculate approximation circle params.
	let mut square_radius = 0.0;
	for v in polygon_vertices
	{
		let square_dist = (v - polygon_center).magnitude2();
		if square_dist > square_radius
		{
			square_radius = square_dist;
		}
	}

	SecondaryLightSource {
		samples: samples_lods,
		sample_size,
		normal: plane_normal_normalized,
		center: polygon_center,
		radius: square_radius.sqrt(),
	}
}

fn prepare_light_grid(map: &mut bsp_map_compact::BSPMap, settings: &LightmappingSettings)
{
	let map_bbox = bsp_map_compact::get_map_bbox(map);

	let light_grid_header = &mut map.light_grid_header;

	let width_clamped = settings.light_grid_cell_width.max(16.0).min(512.0);
	let height_clamped = settings.light_grid_cell_height.max(16.0).min(512.0);
	light_grid_header.grid_cell_size = [width_clamped, width_clamped, height_clamped];

	// Align grid cells properly.
	for i in 0 .. 3
	{
		let start = (map_bbox.min[i] / light_grid_header.grid_cell_size[i]).floor();
		let end = (map_bbox.max[i] / light_grid_header.grid_cell_size[i]).ceil();

		light_grid_header.grid_start[i] = start * light_grid_header.grid_cell_size[i];
		light_grid_header.grid_size[i] = (end - start) as u32 + 1;
	}

	println!(
		"Light grid size: {} x {} x {} ({} elements)",
		light_grid_header.grid_size[0],
		light_grid_header.grid_size[1],
		light_grid_header.grid_size[2],
		light_grid_header.grid_size[0] * light_grid_header.grid_size[1] * light_grid_header.grid_size[2]
	);
}

type LightGridUncompressed = Vec<bsp_map_compact::LightGridElement>;

fn calculate_light_grid(
	sample_grid_size: u32,
	primary_lights: &LightsByLeaf,
	secondary_lights: &[SecondaryLightSource],
	emissive_lights: &[SecondaryLightSource],
	map: &bsp_map_compact::BSPMap,
	visibility_matrix: &pvs::VisibilityMatrix,
) -> LightGridUncompressed
{
	let light_grid_header = &map.light_grid_header;

	println!("Calculating light grid");

	// Prepare sample grid shifts.
	// We need to calculate light for multiple samples in order to make it smoother.
	let mut sample_shifts_grid =
		[Vec3f::zero(); (MAX_SAMPLE_GRID_SIZE * MAX_SAMPLE_GRID_SIZE * MAX_SAMPLE_GRID_SIZE) as usize];
	if sample_grid_size > 1
	{
		let grid_size_f = sample_grid_size as f32;
		let step = Vec3f::from(light_grid_header.grid_cell_size) / grid_size_f;
		let grid_start = (-0.5 * (grid_size_f - 1.0)) * step;
		for x in 0 .. sample_grid_size
		{
			for y in 0 .. sample_grid_size
			{
				for z in 0 .. sample_grid_size
				{
					sample_shifts_grid[(x + y * sample_grid_size + z * sample_grid_size * sample_grid_size) as usize] =
						grid_start + Vec3f::new(x as f32, y as f32, z as f32).mul_element_wise(step);
				}
			}
		}
	}
	let num_sample_grid_shifts = (sample_grid_size * sample_grid_size * sample_grid_size) as usize;

	// Limit distance to light to avoid almost infinity light for samples too close to light sources.
	let min_light_dist = light_grid_header.grid_cell_size[0]
		.min(light_grid_header.grid_cell_size[1])
		.min(light_grid_header.grid_cell_size[2]) *
		0.5;
	let min_light_square_dist = min_light_dist * min_light_dist;

	let samples_complete = atomic::AtomicU32::new(0);
	let samples_total =
		light_grid_header.grid_size[0] * light_grid_header.grid_size[1] * light_grid_header.grid_size[2];

	let mut light_grid = vec![[0.0, 0.0, 0.0]; samples_total as usize];

	light_grid
		.par_iter_mut()
		.enumerate()
		.for_each(|(sample_address, dst_light)| {
			let (x, y, z) = get_light_grid_coord_for_address(light_grid_header, sample_address);
			let pos = Vec3f::from(light_grid_header.grid_start) +
				Vec3f::new(x as f32, y as f32, z as f32)
					.mul_element_wise(Vec3f::from(light_grid_header.grid_cell_size));

			let mut total_light = [0.0, 0.0, 0.0];
			let mut num_valid_shift_points = 0;
			for shift in &sample_shifts_grid[0 .. num_sample_grid_shifts]
			{
				if let Some(pos_corrected) = correct_light_grid_sample_position(map, &(pos + shift))
				{
					let light = calculate_light_for_grid_point(
						&pos_corrected,
						primary_lights,
						secondary_lights,
						emissive_lights,
						map,
						visibility_matrix,
						min_light_square_dist,
					);

					num_valid_shift_points += 1;
					total_light[0] += light[0];
					total_light[1] += light[1];
					total_light[2] += light[2];
				}
			} // for multisample shifts
			if num_valid_shift_points > 0
			{
				// Scale light properly, since we perform integration over whole sphere.
				let scale = 1.0 / ((num_valid_shift_points as f32) * std::f32::consts::TAU);
				total_light[0] *= scale;
				total_light[1] *= scale;
				total_light[2] *= scale;
			}

			*dst_light = total_light;

			// Track progress.
			let samples_complete_before = samples_complete.fetch_add(1, atomic::Ordering::SeqCst);
			let samples_complete_after = samples_complete_before + 1;

			let ratio_before = samples_complete_before * 256 / samples_total;
			let ratio_after = samples_complete_after * 256 / samples_total;
			if ratio_after > ratio_before
			{
				print!(
					"\r{:03.2}% complete ({} of {} samples)",
					(samples_complete_after as f32) * 100.0 / (samples_total as f32),
					samples_complete_after,
					samples_total,
				);
				let _ignore_errors = std::io::stdout().flush();
			}
		});

	println!("\nDone!");
	light_grid
}

fn get_light_grid_sample_address(header: &bsp_map_compact::LightGridHeader, x: u32, y: u32, z: u32) -> usize
{
	// Store columns first
	(z + (x + y * header.grid_size[0]) * header.grid_size[2]) as usize
}

fn get_light_grid_coord_for_address(header: &bsp_map_compact::LightGridHeader, address: usize) -> (u32, u32, u32)
{
	// Store columns first
	let address_u32 = address as u32;
	let z = address_u32 % header.grid_size[2];
	let layer = address_u32 / header.grid_size[2];
	let x = layer % header.grid_size[0];
	let y = layer / header.grid_size[0];

	(x, y, z)
}

fn compress_light_grid(
	map: &bsp_map_compact::BSPMap,
	light_grid: &LightGridUncompressed,
) -> (
	Vec<bsp_map_compact::LightGridColumn>,
	Vec<bsp_map_compact::LightGridElement>,
)
{
	let light_grid_header = &map.light_grid_header;

	println!("Compressing light grid");

	let mut light_grid_columns = vec![
		bsp_map_compact::LightGridColumn::default();
		(light_grid_header.grid_size[0] * light_grid_header.grid_size[1]) as usize
	];
	let mut light_grid_samples = Vec::new();

	let get_sample = |x, y, z| light_grid[get_light_grid_sample_address(light_grid_header, x, y, z)];

	for x in 0 .. light_grid_header.grid_size[0]
	{
		for y in 0 .. light_grid_header.grid_size[1]
		{
			let mut min_non_zero_light_z = 0;
			while min_non_zero_light_z < light_grid_header.grid_size[2]
			{
				let light = get_sample(x, y, min_non_zero_light_z);
				if light[0] > 0.0 || light[1] > 0.0 || light[2] > 0.0
				{
					break;
				}
				min_non_zero_light_z += 1;
			}

			let mut max_non_zero_light_z = light_grid_header.grid_size[2] - 1;
			while max_non_zero_light_z > min_non_zero_light_z
			{
				let light = get_sample(x, y, max_non_zero_light_z);
				if light[0] > 0.0 || light[1] > 0.0 || light[2] > 0.0
				{
					break;
				}
				max_non_zero_light_z -= 1;
			} // for z

			light_grid_columns[(x + y * light_grid_header.grid_size[0]) as usize] = bsp_map_compact::LightGridColumn {
				first_sample: light_grid_samples.len() as u32,
				num_samples: max_non_zero_light_z + 1 - min_non_zero_light_z,
				start_z: min_non_zero_light_z,
			};

			for z in min_non_zero_light_z ..= max_non_zero_light_z
			{
				light_grid_samples.push(get_sample(x, y, z));
			}
		} // for y
	} // for x

	println!("Non-empty light grid samples: {}", light_grid_samples.len());

	(light_grid_columns, light_grid_samples)
}

fn calculate_light_for_grid_point(
	pos: &Vec3f,
	primary_lights: &LightsByLeaf,
	secondary_lights: &[SecondaryLightSource],
	emissive_lights: &[SecondaryLightSource],
	map: &bsp_map_compact::BSPMap,
	visibility_matrix: &pvs::VisibilityMatrix,
	min_light_square_dist: f32,
) -> [f32; 3]
{
	let point_leaf_index = bsp_map_compact::get_leaf_for_point(map, pos) as usize;
	let visibility_matrix_row =
		&visibility_matrix[point_leaf_index * map.leafs.len() .. (point_leaf_index + 1) * map.leafs.len()];

	let mut total_light = [0.0, 0.0, 0.0];
	for light_source_leaf_index in 0 .. map.leafs.len()
	{
		if !visibility_matrix_row[light_source_leaf_index]
		{
			// This leaf is not visible. Ignore point ligths and secondary lights of this leaf.
			continue;
		}

		for primay_light in &primary_lights[light_source_leaf_index]
		{
			let vec_to_light = primay_light.pos - pos;
			let vec_to_light_len2 = vec_to_light.magnitude2().max(min_light_square_dist);

			if !can_see(&primay_light.pos, &pos, map)
			{
				// In shadow.
				continue;
			}

			// Do not use agle cos because we add light into light sphere.
			let light_scale = 1.0 / vec_to_light_len2;
			total_light[0] += primay_light.color[0] * light_scale;
			total_light[1] += primay_light.color[1] * light_scale;
			total_light[2] += primay_light.color[2] * light_scale;
		} // for primary lights

		let light_source_leaf = &map.leafs[light_source_leaf_index as usize];
		let light_source_leaf_polygons_range = light_source_leaf.first_polygon as usize ..
			(light_source_leaf.first_polygon + light_source_leaf.num_polygons) as usize;

		for light_set in [secondary_lights, emissive_lights]
		{
			if light_set.is_empty()
			{
				continue;
			}

			for light_source_polygon_index in light_source_leaf_polygons_range.clone()
			{
				let light = &light_set[light_source_polygon_index];
				if light.samples.is_empty()
				{
					continue;
				}

				// Compute LOD.
				let light_source_lod = get_light_source_lod(&pos, light);
				let min_dist2 =
					get_secondary_light_source_sample_min_square_distance(light.sample_size, light_source_lod)
						.max(min_light_square_dist);

				// Iterate over all samples of this LOD.
				for sample in &light.samples[light_source_lod]
				{
					let vec_to_light = sample.pos - pos;
					let vec_to_light_len2 = vec_to_light.magnitude2().max(MIN_POSITIVE_VALUE);
					let vec_to_light_normalized = vec_to_light / vec_to_light_len2.sqrt();

					let angle_cos_src = -(light.normal.dot(vec_to_light_normalized));
					if angle_cos_src <= 0.0
					{
						// Do not determine visibility for texels behind light source plane.
						continue;
					}

					if !can_see(&sample.pos, &pos, map)
					{
						// In shadow.
						continue;
					}

					// Do not use agle cos because we add light into light sphere.
					let vec_to_light_len2_clamped = vec_to_light_len2.max(min_dist2);

					let light_scale = angle_cos_src / vec_to_light_len2_clamped;
					total_light[0] += sample.color[0] * light_scale;
					total_light[1] += sample.color[1] * light_scale;
					total_light[2] += sample.color[2] * light_scale;
				} // for light samples.
			} // for visible leaf polygons.
		} // for light sets.
	} // for BSP leafs.

	total_light
}

fn correct_light_grid_sample_position(map: &bsp_map_compact::BSPMap, pos: &Vec3f) -> Option<Vec3f>
{
	if is_point_inside_leaf_volume(map, pos)
	{
		return Some(*pos);
	}

	let grid_cell_size = Vec3f::from(map.light_grid_header.grid_cell_size);

	// Shifts for cube.
	// First try side shifts, than edge shifts, lastly vertices shifts.
	const SHIFTS: [[f32; 3]; 26] = [
		// Cube side shifts.
		[1.0, 0.0, 0.0],
		[-1.0, 0.0, 0.0],
		[0.0, 1.0, 0.0],
		[0.0, -1.0, 0.0],
		[0.0, 0.0, 1.0],
		[0.0, 0.0, -1.0],
		// Cube edge shifts.
		[1.0, 1.0, 0.0],
		[-1.0, 1.0, 0.0],
		[1.0, -1.0, 0.0],
		[-1.0, -1.0, 0.0],
		[1.0, 0.0, 1.0],
		[-1.0, 0.0, 1.0],
		[1.0, 0.0, -1.0],
		[-1.0, 0.0, -1.0],
		[0.0, 1.0, 1.0],
		[0.0, -1.0, 1.0],
		[0.0, 1.0, -1.0],
		[0.0, -1.0, -1.0],
		// Cube vertices shifts.
		[1.0, 1.0, 1.0],
		[-1.0, 1.0, 1.0],
		[1.0, -1.0, 1.0],
		[-1.0, -1.0, 1.0],
		[1.0, 1.0, -1.0],
		[-1.0, 1.0, -1.0],
		[1.0, -1.0, -1.0],
		[-1.0, -1.0, -1.0],
	];
	// Try to shift sample position in order to move it into leaf bounds.
	// Iterate over steps, for each step try all possible shifts.
	let max_step = 3;
	for step in 1 ..= max_step
	{
		let step_f = (step as f32) / (max_step as f32);
		let step_vec = grid_cell_size * step_f;
		for shift in SHIFTS
		{
			let delta = Vec3f::from(shift).mul_element_wise(step_vec);
			let pos_shifted = pos + delta;
			if is_point_inside_leaf_volume(map, &pos_shifted)
			{
				return Some(pos_shifted);
			}
		}
	}

	None
}

fn is_point_inside_leaf_volume(map: &bsp_map_compact::BSPMap, point: &Vec3f) -> bool
{
	let leaf_index = bsp_map_compact::get_leaf_for_point(map, point);
	let leaf = &map.leafs[leaf_index as usize];
	for polygon in &map.polygons[leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize]
	{
		if point.dot(polygon.plane.vec) <= polygon.plane.dist
		{
			return false;
		}
	}

	true
}

fn calculate_dinstance_between_point_and_circle(
	point: &Vec3f,
	circle_center: &Vec3f,
	circle_normal: &Vec3f,
	circle_radius: f32,
) -> f32
{
	// See https://www.geometrictools.com/Documentation/DistanceToCircle3.pdf.

	let vec_from_center_to_point = point - circle_center;
	let signed_dinstance_to_circle_plane = vec_from_center_to_point.dot(*circle_normal);

	let perpendicular_vec = signed_dinstance_to_circle_plane * circle_normal;
	let vec_from_center_to_point_projection = vec_from_center_to_point - perpendicular_vec;
	let vec_from_center_to_point_projection_square_len = vec_from_center_to_point_projection.magnitude2();
	if vec_from_center_to_point_projection_square_len <= circle_radius * circle_radius
	{
		return signed_dinstance_to_circle_plane.abs();
	}

	let dist_from_projection_point_to_circle = vec_from_center_to_point_projection_square_len.sqrt() - circle_radius;
	let square_len = dist_from_projection_point_to_circle * dist_from_projection_point_to_circle +
		signed_dinstance_to_circle_plane * signed_dinstance_to_circle_plane;
	square_len.sqrt()
}

fn get_secondary_light_source_sample_min_square_distance(sample_size: f32, light_source_lod: usize) -> f32
{
	// Limit inv square distance - do not allow almost infinite light in case if light sample is too close.
	let current_sample_size = ((1 << light_source_lod) as f32) * sample_size;
	0.25 * current_sample_size * current_sample_size
}

fn pre_correct_secondary_light_sample_position(pos_initial: &Vec3f, polygon_center: &Vec3f) -> Vec3f
{
	// HACK! Shift sample position slightly towards polygon center to avoid completely black outlines in corners.
	pos_initial * (63.0 / 64.0) + polygon_center * (1.0 / 64.0)
}

fn correct_sample_position(
	map: &bsp_map_compact::BSPMap,
	pos: &Vec3f,
	lightmap_basis: &LightmapBasis,
	polygon_center: &Vec3f,
) -> Vec3f
{
	// Can see from sample point to polygon center - return initial sample point.
	if can_see(pos, polygon_center, map)
	{
		return *pos;
	}

	// Try to perform fixed adjustments.
	// Use steps along lightmap basis with length 0.5, than with length 1.0, than diagonal steps.
	const SHIFT_VECS: [[f32; 2]; 12] = [
		[0.5, 0.0],
		[-0.5, 0.0],
		[0.0, 0.5],
		[0.0, -0.5],
		[1.0, 0.0],
		[-1.0, 0.0],
		[0.0, 1.0],
		[0.0, -1.0],
		[1.0, 1.0],
		[1.0, -1.0],
		[-1.0, 1.0],
		[-1.0, -1.0],
	];

	for shift in SHIFT_VECS
	{
		let pos_corrected = pos + lightmap_basis.u_vec * shift[0] + lightmap_basis.v_vec * shift[1];
		if can_see(&pos_corrected, polygon_center, map)
		{
			return pos_corrected;
		}
	}

	// Hard situation. Try to move sample to polygon center via iterative steps.
	let max_basis_vec_len = lightmap_basis.u_vec.magnitude().max(lightmap_basis.v_vec.magnitude());
	let mut pos_corrected = *pos;
	for _i in 0 .. 16
	{
		let vec_to_center = polygon_center - pos_corrected;
		let vec_to_center_len = vec_to_center.magnitude().max(MIN_POSITIVE_VALUE);
		let vec_to_center_normalized = vec_to_center / vec_to_center_len;
		pos_corrected += vec_to_center_normalized * max_basis_vec_len.min(vec_to_center_len);
		if can_see(&pos_corrected, polygon_center, map)
		{
			return pos_corrected;
		}
	}

	// In worst case just return polygon center.
	return *polygon_center;
}
