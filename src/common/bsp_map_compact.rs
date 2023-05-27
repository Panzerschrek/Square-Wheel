use super::{bbox::*, math_types::*, plane::*};

// This file contains declaration of compact BSP map representation.
// Such representation allows to process BSP map in order to draw it or in order to save or load it.

#[derive(Default)]
pub struct BSPMap
{
	// Last node is tree root.
	pub nodes: Vec<BSPNode>,
	pub leafs: Vec<BSPLeaf>,
	pub polygons: Vec<Polygon>,
	pub portals: Vec<Portal>,
	pub leafs_portals: Vec<u32>,
	// Both polygon and portal vertices.
	pub vertices: Vec<Vec3f>,
	pub textures: Vec<Texture>,
	pub submodels: Vec<Submodel>,
	pub submodels_bsp_nodes: Vec<SubmodelBSPNode>,

	// Data for entities. Entity is a set of string key-value pairs.
	pub entities: Vec<Entity>,
	pub key_value_pairs: Vec<KeyValuePair>,
	// UTF-8 bytes of all strings.
	pub strings_data: Vec<u8>,

	pub lightmaps_data: Vec<LightmapElement>,
	pub directional_lightmaps_data: Vec<DirectionalLightmapElement>,

	pub light_grid_header: LightGridHeader,
	// 2D matrix with size grid_size[0] * grid_size[1]
	pub light_grid_columns: Vec<LightGridColumn>,
	// Combined values of light samples for each column.
	pub light_grid_samples: Vec<LightGridElement>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BSPNode
{
	// If child index is greater or equal than FIRST_LEAF_INDEX - child is leaf.
	pub children: [u32; 2],
	pub plane: Plane,
}

pub const FIRST_LEAF_INDEX: u32 = 1 << 31;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BSPLeaf
{
	pub first_polygon: u32,
	pub num_polygons: u32,
	pub first_leaf_portal: u32,
	pub num_leaf_portals: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Polygon
{
	pub first_vertex: u32,
	pub num_vertices: u32,
	pub plane: Plane,
	pub tex_coord_equation: [Plane; 2],
	// Store precalculated min/max texture coordinates. Min value is rounded down, maximum value is rounded up.
	// Surface size is max - min.
	// Do this because we calculate lightmap position/size based on this values.
	// We can't recalculate this values after map loading since calculation result may be different due to floating-point calculation errors.
	pub tex_coord_min: [i32; 2],
	pub tex_coord_max: [i32; 2],
	// Offset is zero if this polygon hs no lightmap.
	pub lightmap_data_offset: u32,
	pub texture: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Portal
{
	pub leafs: [u32; 2],
	pub plane: Plane,
	pub first_vertex: u32,
	pub num_vertices: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Submodel
{
	pub root_node: u32,
	// Polygons of submodels are stored sequentially.
	// But for proper ordering BSP tree must be used.
	pub first_polygon: u32,
	pub num_polygons: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SubmodelBSPNode
{
	pub plane: Plane,
	pub first_polygon: u32,
	pub num_polygons: u32,
	// Invalid index if child does not exists.
	pub children: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entity
{
	pub first_key_value_pair: u32,
	pub num_key_value_pairs: u32,
	// invalid index if this entity has no submodel.
	pub submodel_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KeyValuePair
{
	pub key: StringRef,
	pub value: StringRef,
}

// Use 16 bits for offset and size.
// This limits total strings data size to 65536 bytes, but this is enought for most cases, since we use strings deduplication.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringRef
{
	pub offset: u16,
	pub size: u16,
}

pub const MAX_TEXTURE_NAME_LEN: usize = 64;
// UTF-8 values of texture (name, path, or some id). Remaining symbols are filled with nulls.
pub type Texture = [u8; MAX_TEXTURE_NAME_LEN];

// Currently it is just a simple diffuse colored light.
pub type LightmapElement = [f32; 3];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirectionalLightmapElement
{
	// Component of light that is constant in any direction.
	pub ambient_light: [f32; 3],
	// Vector towards predominant light direction, scaled by light intensity.
	pub light_direction_vector_scaled: Vec3f,
	// Normalized deviation. 0 - for totally sharp light, 1 - for totally smooth light.
	pub directional_light_deviation: f32,
	// Color for directional light is normalized.
	pub directional_light_color: [f32; 3],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct LightGridHeader
{
	pub grid_cell_size: [f32; 3],
	// Position of first sample.
	pub grid_start: [f32; 3],
	// Number of samples.
	pub grid_size: [u32; 3],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct LightGridColumn
{
	// Relative to whole grid.
	pub start_z: u32,
	pub first_sample: u32,
	pub num_samples: u32,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct LightGridElement
{
	// -x, +x, -y, +y, -z, +z values of ambient light cube
	pub light_cube: [[f32; 3]; 6],
	// Vector towards predominant light direction, scaled by light intensity.
	pub light_direction_vector_scaled: Vec3f,
	// Color for directional light is normalized.
	pub directional_light_color: [f32; 3],
}

impl Default for LightGridElement
{
	fn default() -> Self
	{
		Self {
			light_cube: [[0.0; 3]; 6],
			light_direction_vector_scaled: Vec3f::zero(),
			directional_light_color: [0.0; 3],
		}
	}
}

pub fn get_map_string(s: StringRef, map: &BSPMap) -> &str
{
	std::str::from_utf8(&map.strings_data[(s.offset as usize) .. ((s.offset + s.size) as usize)]).unwrap_or("")
}

pub fn get_texture_string(texture_name: &Texture) -> &str
{
	let null_pos = texture_name
		.iter()
		.position(|x| *x == 0_u8)
		.unwrap_or(texture_name.len());
	std::str::from_utf8(&texture_name[0 .. null_pos]).unwrap_or("")
}

pub fn get_map_bbox(map: &BSPMap) -> BBox
{
	// Calculate map bounding box based on all vertices (polygons, portals, submodels).
	let inf = 1e24;
	let mut bbox = BBox {
		min: Vec3f::new(inf, inf, inf),
		max: Vec3f::new(-inf, -inf, -inf),
	};

	for vertex in &map.vertices
	{
		bbox.extend_with_point(vertex);
	}

	bbox
}

pub fn get_submodel_bbox(map: &BSPMap, submodel: &Submodel) -> BBox
{
	// Calculate model bounding box based on all vertices of all polygons.
	let inf = 1e24;
	let mut bbox = BBox {
		min: Vec3f::new(inf, inf, inf),
		max: Vec3f::new(-inf, -inf, -inf),
	};

	for polygon in
		&map.polygons[submodel.first_polygon as usize .. (submodel.first_polygon + submodel.num_polygons) as usize]
	{
		for vertex in get_polygon_vertices(map, polygon)
		{
			bbox.extend_with_point(vertex);
		}
	}

	bbox
}

pub fn get_polygon_vertices<'a>(map: &'a BSPMap, polygon: &Polygon) -> &'a [Vec3f]
{
	&map.vertices[polygon.first_vertex as usize .. (polygon.first_vertex + polygon.num_vertices) as usize]
}

pub fn get_submodel_polygons<'a>(map: &'a BSPMap, submodel: &Submodel) -> &'a [Polygon]
{
	&map.polygons[submodel.first_polygon as usize .. (submodel.first_polygon + submodel.num_polygons) as usize]
}

pub fn get_leaf_for_point(map: &BSPMap, point: &Vec3f) -> u32
{
	let mut index = get_root_node_index(map);
	loop
	{
		if index >= FIRST_LEAF_INDEX
		{
			return index - FIRST_LEAF_INDEX;
		}

		let node = &map.nodes[index as usize];
		index = if node.plane.vec.dot(*point) > node.plane.dist
		{
			node.children[0]
		}
		else
		{
			node.children[1]
		};
	}
}

pub fn get_root_node_index(map: &BSPMap) -> u32
{
	(map.nodes.len() - 1) as u32
}

pub fn get_convex_hull_bsp_leafs<Collection: std::iter::Extend<u32>>(
	map: &BSPMap,
	vertices: &[Vec3f],
	out_leafs: &mut Collection,
)
{
	get_convex_hull_bsp_leafs_r(map, get_root_node_index(map), vertices, out_leafs)
}

fn get_convex_hull_bsp_leafs_r<Collection: std::iter::Extend<u32>>(
	map: &BSPMap,
	node_index: u32,
	vertices: &[Vec3f],
	out_leafs: &mut Collection,
)
{
	if node_index >= FIRST_LEAF_INDEX
	{
		out_leafs.extend([node_index - FIRST_LEAF_INDEX]);
	}
	else
	{
		let node = &map.nodes[node_index as usize];

		let mut vertices_front = 0;
		for &vertex in vertices
		{
			if node.plane.vec.dot(vertex) > node.plane.dist
			{
				vertices_front += 1;
			}
		}

		let node_children = node.children;

		if vertices_front > 0
		{
			get_convex_hull_bsp_leafs_r(map, node_children[0], vertices, out_leafs);
		}
		if vertices_front < vertices.len()
		{
			get_convex_hull_bsp_leafs_r(map, node_children[1], vertices, out_leafs);
		}
	}
}
