use super::{bsp_map_compact, material::*, math_types::*};

pub type MaterialsOpacityTable = Vec<f32>;

pub fn build_materials_opacity_table(map: &bsp_map_compact::BSPMap, materials: &MaterialsMap) -> MaterialsOpacityTable
{
	map.textures
		.iter()
		.map(|texture_name| get_texture_opacity(texture_name, materials))
		.collect()
}

fn get_texture_opacity(texture_name: &bsp_map_compact::Texture, materials: &MaterialsMap) -> f32
{
	if let Some(material) = materials.get(bsp_map_compact::get_texture_string(texture_name))
	{
		if material.skybox.is_some()
		{
			// For now just assume skyboxes passing all light
			// It is needed because right now we pass sun light using just gaps in world geometry.
			1.0
		}
		else if material.shadow
		{
			match material.blending_mode
			{
				BlendingMode::None => 0.0,
				BlendingMode::Average => 0.5,
				BlendingMode::Additive => 1.0,
				// TODO - maybe calculate average opacity in case of alpha-dependent blending?
				BlendingMode::AlphaTest => 0.5,
				BlendingMode::AlphaBlend => 0.5,
			}
		}
		else
		{
			1.0
		}
	}
	else
	{
		0.0
	}
}

pub fn can_see(from: &Vec3f, to: &Vec3f, map: &bsp_map_compact::BSPMap, opacity_table: &MaterialsOpacityTable) -> bool
{
	get_shadow_factor(from, to, map, opacity_table) > 0.0
}

pub fn get_shadow_factor(
	from: &Vec3f,
	to: &Vec3f,
	map: &bsp_map_compact::BSPMap,
	opacity_table: &MaterialsOpacityTable,
) -> f32
{
	let root_node = bsp_map_compact::get_root_node_index(map);
	get_shadow_factor_r(from, to, root_node, map, opacity_table)
	// TODO - check intersection with submodel polygons?
}

// Speed-up intersection calculation - recursively determine loction of check edge withing BSP tree.
// Than check only leafs where edge is actually located.
fn get_shadow_factor_r(
	v0: &Vec3f,
	v1: &Vec3f,
	current_index: u32,
	map: &bsp_map_compact::BSPMap,
	opacity_table: &MaterialsOpacityTable,
) -> f32
{
	if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
	{
		let leaf_index = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
		let leaf = &map.leafs[leaf_index as usize];
		let mut shadow_factor_accumulated = 1.0;
		for polygon in &map.polygons[leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize]
		{
			let opacity = opacity_table[polygon.texture as usize];
			if opacity >= 1.0
			{
				continue;
			}
			if edge_intersects_with_polygon(v0, v1, polygon, map)
			{
				if opacity <= 0.0
				{
					return 0.0;
				}
				shadow_factor_accumulated *= opacity;
			}
		}
		return shadow_factor_accumulated;
	}
	else
	{
		let node = &map.nodes[current_index as usize];
		let dist0 = v0.dot(node.plane.vec) - node.plane.dist;
		let dist1 = v1.dot(node.plane.vec) - node.plane.dist;
		if dist0 >= 0.0 && dist1 >= 0.0
		{
			return get_shadow_factor_r(v0, v1, node.children[0], map, opacity_table);
		}
		if dist0 <= 0.0 && dist1 <= 0.0
		{
			return get_shadow_factor_r(v0, v1, node.children[1], map, opacity_table);
		}

		// Split edge using BSP node plane.

		let dist_sum = dist1 - dist0;
		if dist_sum.abs() < MIN_POSITIVE_VALUE
		{
			// Edge is almost on polygon plane.
			return 1.0;
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

		let shadow_factor_front =
			get_shadow_factor_r(v_front, &intersection_pos_front, node.children[0], map, opacity_table);
		if shadow_factor_front <= 0.0
		{
			return 0.0;
		}

		let shadow_factor_back =
			get_shadow_factor_r(&intersection_pos_back, v_back, node.children[1], map, opacity_table);
		if shadow_factor_back <= 0.0
		{
			return 0.0;
		}

		// Absorb light by bots sub-paths - front and back.
		return shadow_factor_front * shadow_factor_back;
	}
}

pub fn get_sun_shadow_factor(
	from: &Vec3f,
	dir: &Vec3f,
	map: &bsp_map_compact::BSPMap,
	opacity_table: &MaterialsOpacityTable,
) -> f32
{
	// For now just trace into direction of the sun.
	// This works since skybox brushes does not block light and we have no geometry behind sky brushes.
	// TODO - find intersections with nearest sky brush towards sun direction instead.
	get_shadow_factor(from, &(from + dir), map, opacity_table)
}

fn edge_intersects_with_polygon(
	v0: &Vec3f,
	v1: &Vec3f,
	polygon: &bsp_map_compact::Polygon,
	map: &bsp_map_compact::BSPMap,
) -> bool
{
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

const MIN_POSITIVE_VALUE: f32 = 1.0 / ((1 << 30) as f32);
