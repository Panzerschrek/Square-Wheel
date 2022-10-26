use super::{
	dynamic_objects_index::*, equations::*, frame_info::*, frame_number::*, light::*, performance_counter::*,
	surfaces::*,
};
use crate::common::{bsp_map_compact, clipping_polygon::*, math_types::*, matrix::*};

pub struct RendererPerformanceCounters
{
	pub materials_update: PerformanceCounter,
	pub visible_leafs_search: PerformanceCounter,
	pub triangle_models_preparation: PerformanceCounter,
	pub surfaces_preparation: PerformanceCounter,
	pub shadow_maps_building: PerformanceCounter,
	pub background_fill: PerformanceCounter,
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
			rasterization: PerformanceCounter::new(window_size),
		}
	}
}

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
	pub lightmap_tc_shift: [u32; 2],
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
