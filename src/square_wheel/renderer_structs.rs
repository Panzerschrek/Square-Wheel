use super::{
	dynamic_objects_index::*, equations::*, frame_info::*, frame_number::*, inline_models_index::*, light::*,
	map_materials_processor::*, partial_renderer::PartialRenderer, performance_counter::*, surfaces::*,
};
use crate::common::{bsp_map_compact, clipping_polygon::*, math_types::*, matrix::*, plane::*};

pub struct RendererPerformanceCounters
{
	pub materials_update: PerformanceCounter,
	pub visible_leafs_search: PerformanceCounter,
	pub triangle_models_preparation: PerformanceCounter,
	pub surfaces_preparation: PerformanceCounter,
	pub shadow_maps_building: PerformanceCounter,
	pub background_fill: PerformanceCounter,
	pub portals_rendering: PerformanceCounter,
	pub rasterization: PerformanceCounter,
}

impl RendererPerformanceCounters
{
	pub fn new() -> Self
	{
		let window_size = 100;
		Self {
			materials_update: PerformanceCounter::new(window_size),
			visible_leafs_search: PerformanceCounter::new(window_size),
			triangle_models_preparation: PerformanceCounter::new(window_size),
			surfaces_preparation: PerformanceCounter::new(window_size),
			shadow_maps_building: PerformanceCounter::new(window_size),
			background_fill: PerformanceCounter::new(window_size),
			portals_rendering: PerformanceCounter::new(window_size),
			rasterization: PerformanceCounter::new(window_size),
		}
	}
}

#[derive(Default)]
pub struct RendererDebugStats
{
	pub num_visible_leafs: usize,
	pub num_visible_submodels_parts: usize,
	pub num_visible_meshes: usize,
	pub num_visible_meshes_parts: usize,
	pub num_triangles: usize,
	pub num_triangle_vertices: usize,
	pub num_decals: usize,
	pub num_decals_leafs_parts: usize,
	pub num_sprites: usize,
	pub num_sprites_leafs_parts: usize,
	pub num_visible_lights: usize,
	pub num_visible_lights_with_shadow: usize,
	pub num_visible_portals: usize,
	pub num_visible_polygons: usize,
	pub num_surfaces_pixels: usize,
}

// Data shared across multiple PartialRenderer instances (independent on view point).
pub struct RenderersCommonData
{
	pub materials_processor: MapMaterialsProcessor,
	pub inline_models_index: InlineModelsIndex,
	pub dynamic_models_index: DynamicObjectsIndex,
	pub decals_index: DynamicObjectsIndex,
	pub sprites_index: DynamicObjectsIndex,
	pub dynamic_lights_index: DynamicObjectsIndex,
	// Index of drawable portals polygons (not view point polygons).
	pub portals_index: DynamicObjectsIndex,
	// Store precalculated list of clip planes for each leaf in order to clip dynamic objects with these planes.
	pub leafs_planes: Vec<LeafClipPlanes>,
}

pub type LeafClipPlanes = Vec<Plane>;

// Mutable data associated with map polygon.
#[derive(Copy, Clone)]
pub struct DrawPolygonData
{
	// Leaf index where this polygon is located or submodel index.
	pub parent: DrawPolygonParent,
	// Precalculaed basis vecs for mip 0
	pub basis_vecs: PolygonBasisVecs,
	// Frame last time this polygon was visible.
	pub visible_frame: FrameNumber,
	// Projected equations for current frame.
	pub depth_equation: DepthEquation,
	pub tex_coord_equation: TexCoordEquation,
	pub surface_pixels_offset: usize,
	pub surface_size: [u32; 2],
	pub mip: u32,
	pub surface_tc_min: [i32; 2],
}

#[derive(Copy, Clone)]
pub enum DrawPolygonParent
{
	Leaf(u32),
	Submodel(u32),
}

// Calculate matrices once for frame and use them during polygons preparation, sorting and polygons ordering.
#[derive(Copy, Clone)]
pub struct VisibleSubmodelMatrices
{
	// Planes matrix for transformation of submodel planes into current position of submodel within the world.
	pub world_planes_matrix: Mat4f,
	pub camera_matrices: CameraMatrices,
}

#[derive(Default, Clone)]
pub struct VisibleSubmodelInfo
{
	pub matrices: Option<VisibleSubmodelMatrices>,
	// Dynamic lights that affects this submodel.
	pub dynamic_lights: Vec<DynamicObjectId>,
}

pub struct VisibleDynamicMeshInfo
{
	pub entity_index: u32,
	pub mesh_index: u32,
	pub vertices_offset: usize,
	pub triangles_offset: usize,
	pub num_visible_triangles: usize,
	pub bbox_vertices_transformed: [Vec3f; 8],
	pub clipping_polygon: ClippingPolygon,
	pub model_matrix: Mat4f,
	pub camera_matrices: CameraMatrices,
	pub mip: u32,
}

#[derive(Default, Copy, Clone)]
pub struct DynamicModelInfo
{
	pub first_visible_mesh: u32,
	pub num_visible_meshes: u32,
}

#[derive(Default, Copy, Clone)]
pub struct DynamicLightInfo
{
	// Light is not visible if all leafs where it is located are not visible.
	pub visible: bool,
	pub shadow_map_data_offset: usize,
	// Cubemap side size or projector shadowmap size.
	pub shadow_map_size: u32,
}

#[derive(Copy, Clone)]
pub struct DecalInfo
{
	pub camera_planes_matrix: Mat4f,
	pub dynamic_light: bsp_map_compact::LightGridElement,
}

#[derive(Copy, Clone)]
pub struct SpriteInfo
{
	pub vertices_projected: [Vec3f; 4],
	pub light: [f32; 3],
	pub mip: u32,
	pub tesselation_level: u32,
}

pub const MAX_SPRITE_TESSELATION_LEVEL: u32 = 4;

// Get list of unique clip planes for given map leaf.
pub fn get_leaf_clip_planes(map: &bsp_map_compact::BSPMap, leaf_index: u32) -> LeafClipPlanes
{
	let leaf = &map.leafs[leaf_index as usize];

	let mut planes = Vec::<Plane>::new();

	let mut add_and_deduplicate_plane = |plane: Plane| {
		// We need to use planes with normalized vector in order to compare distances properly.
		let normal_length = plane.vec.magnitude();
		if normal_length < 0.00000000001
		{
			return;
		}
		let plane_normalized = Plane {
			vec: plane.vec / normal_length,
			dist: plane.dist / normal_length,
		};

		// Perform dedupliction - iterate over previous planes.
		// We have quadratic complexity here, but it is not a problem since number of planes are usually small (6 for cube-shaped leaf).
		for prev_plane in &mut planes
		{
			// Dot product is angle cos since vectors are normalized.
			let dot = plane_normalized.vec.dot(prev_plane.vec);
			if dot >= 1.0 - 1.0 / 256.0
			{
				// Planes are (almost) parallel.
				// Select plane with greater distance to clip more.
				prev_plane.dist = prev_plane.dist.max(plane_normalized.dist);
				return;
			}
		}

		planes.push(plane_normalized);
	};

	// Use planes of all portals.
	for &portal_index in
		&map.leafs_portals[leaf.first_leaf_portal as usize .. (leaf.first_leaf_portal + leaf.num_leaf_portals) as usize]
	{
		let portal = &map.portals[portal_index as usize];
		let clip_plane = if portal.leafs[0] == leaf_index
		{
			portal.plane
		}
		else
		{
			portal.plane.get_inverted()
		};
		add_and_deduplicate_plane(clip_plane);
	}

	// Use planes of all polygons.
	for polygon in &map.polygons[leaf.first_polygon as usize .. (leaf.first_polygon + leaf.num_polygons) as usize]
	{
		add_and_deduplicate_plane(polygon.plane);
	}

	planes
}

pub fn create_dynamic_light_with_shadow<'a>(
	light: &DynamicLight,
	light_info: &DynamicLightInfo,
	shadow_maps_data: &'a [ShadowMapElement],
) -> DynamicLightWithShadow<'a>
{
	DynamicLightWithShadow {
		position: light.position,
		radius: light.radius,
		inv_square_radius: 1.0 / (light.radius * light.radius),
		color: light.color,
		shadow_map: match &light.shadow_type
		{
			DynamicLightShadowType::None => ShadowMap::None,
			DynamicLightShadowType::Cubemap => ShadowMap::Cube(
				if light_info.visible
				{
					create_dynamic_light_cube_shadow_map(light_info, shadow_maps_data)
				}
				else
				{
					create_dynamic_light_cube_shadow_map_dummy()
				},
			),
			DynamicLightShadowType::Projector { rotation, fov } => ShadowMap::Projector(
				if light_info.visible
				{
					create_dynamic_light_projector_shadow_map(rotation, *fov, light_info, shadow_maps_data)
				}
				else
				{
					create_dynamic_light_projector_shadow_map_dummy()
				},
			),
		},
	}
}

pub fn create_dynamic_light_cube_shadow_map<'a>(
	light_info: &DynamicLightInfo,
	shadow_maps_data: &'a [ShadowMapElement],
) -> CubeShadowMap<'a>
{
	let side_data_size = (light_info.shadow_map_size * light_info.shadow_map_size) as usize;
	let shadow_map_data =
		&shadow_maps_data[light_info.shadow_map_data_offset .. light_info.shadow_map_data_offset + side_data_size * 6];

	CubeShadowMap {
		size: light_info.shadow_map_size,
		sides: [
			&shadow_map_data[0 * side_data_size .. 1 * side_data_size],
			&shadow_map_data[1 * side_data_size .. 2 * side_data_size],
			&shadow_map_data[2 * side_data_size .. 3 * side_data_size],
			&shadow_map_data[3 * side_data_size .. 4 * side_data_size],
			&shadow_map_data[4 * side_data_size .. 5 * side_data_size],
			&shadow_map_data[5 * side_data_size .. 6 * side_data_size],
		],
	}
}

pub fn create_dynamic_light_projector_shadow_map<'a>(
	rotation: &QuaternionF,
	fov: RadiansF,
	light_info: &DynamicLightInfo,
	shadow_maps_data: &'a [ShadowMapElement],
) -> ProjectorShadowMap<'a>
{
	let data_size = (light_info.shadow_map_size * light_info.shadow_map_size) as usize;
	let shadow_map_data =
		&shadow_maps_data[light_info.shadow_map_data_offset .. light_info.shadow_map_data_offset + data_size];

	let inv_half_fov_tan = 1.0 / (fov * 0.5).tan();

	ProjectorShadowMap {
		size: light_info.shadow_map_size,
		data: shadow_map_data,
		basis_x: rotation.rotate_vector(Vec3f::unit_y()) * inv_half_fov_tan,
		basis_y: rotation.rotate_vector(Vec3f::unit_z()) * inv_half_fov_tan,
		basis_z: rotation.rotate_vector(-Vec3f::unit_x()),
	}
}

pub struct PortalsRenderingData
{
	pub renderer: PartialRenderer,
	// Index of drawable portals polygons (not view point polygons).
	pub portals_info: Vec<PortalInfo>,
	// Store textures pixels as raw array.
	// Use specific color while preparing surfaces or performing rasterization.
	// TODO - make sure alignment is correct.
	pub textures_pixels: Vec<u8>,
	pub num_textures_pixels: usize,
}

#[derive(Copy, Clone, Default)]
pub struct PortalInfo
{
	pub resolution: [u32; 2], // zero if invisible
	pub texture_pixels_offset: usize,
	// TODO - add mip-level here.

	// Projected equations for current frame.
	pub depth_equation: DepthEquation,
	pub tex_coord_equation: TexCoordEquation,
	pub tc_min: [i32; 2],
}
