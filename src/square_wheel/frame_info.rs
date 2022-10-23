use super::{resources_manager::*, textures::*, triangle_model::*};
use crate::common::{bbox::*, material, math_types::*, matrix::*};
use serde::{Deserialize, Serialize};

pub struct FrameInfo
{
	pub camera_matrices: CameraMatrices,
	pub game_time_s: f32,
	// submodels mapped 1 to 1 to initial submodels.
	pub submodel_entities: Vec<SubmodelEntityOpt>,
	pub model_entities: Vec<ModelEntity>,
	pub decals: Vec<Decal>,
	pub sprites: Vec<Sprite>,
	pub lights: Vec<DynamicLight>,
	pub skybox_rotation: QuaternionF,
}

pub type SubmodelEntityOpt = Option<SubmodelEntity>;

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmodelEntity
{
	// Position of Bbox center.
	pub position: Vec3f,
	pub rotation: QuaternionF,
}

#[derive(Clone)]
pub struct ModelEntity
{
	pub position: Vec3f,
	pub rotation: QuaternionF,
	pub animation: AnimationPoint,
	pub model: SharedResourcePtr<TriangleModel>,
	pub texture: SharedResourcePtr<TextureLiteWithMips>,
	pub blending_mode: material::BlendingMode,
	pub lighting: ModelLighting,

	// Weapon or thing in player's hands.
	// Draw it always and after any other models.
	pub is_view_model: bool,

	// Use it to override bbox (in object-space) to improve models ordering.
	// For example, use bbox with size reduced relative to true model bbox.
	pub ordering_custom_bbox: Option<BBox>,
}

// Two frames with interpolation fator between them.
// value = frames[0] * lerp + frames[1] * (1.0 - lerp)
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct AnimationPoint
{
	pub frames: [u32; 2],
	pub lerp: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ModelLighting
{
	// Use just light from grid
	Default,
	// Use just constant light.
	ConstantLight([f32; 3]),
	// Complex case - use combination of light grid and constant color.
	AdvancedLight
	{
		// Scale of light, fetched from light grid.
		grid_light_scale: f32,
		// Constant light, added to value, fetched from light grid.
		light_add: [f32; 3],
		// World space position used for light grid fetch.
		// May be different from model position (for various reasons).
		position: Vec3f,
	},
}

#[derive(Clone)]
pub struct Decal
{
	// Decal primitive is cube with half-size = 1.
	pub position: Vec3f,
	pub rotation: QuaternionF,
	pub scale: Vec3f,
	pub texture: SharedResourcePtr<TextureLiteWithMips>,
	pub blending_mode: material::BlendingMode,
	pub lightmap_light_scale: f32,
	pub light_add: [f32; 3],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sprite
{
	pub position: Vec3f,
	pub radius: f32,
	pub texture: SharedResourcePtr<TextureLiteWithMips>,
	pub blending_mode: material::BlendingMode,
	pub orientation: SpriteOrientation,
	pub light_scale: f32,
	pub light_add: [f32; 3],
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum SpriteOrientation
{
	ParallelToCameraPlane,
	FacingTowardsCamera,
	AlignToZAxisParallelToCameraPlane,
	AlignToZAxisFacingTowardsCamera,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct DynamicLight
{
	pub position: Vec3f,
	pub radius: f32,
	pub color: [f32; 3],
	pub shadow_type: DynamicLightShadowType,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum DynamicLightShadowType
{
	None,
	Cubemap,
	Projector
	{
		rotation: QuaternionF,
		fov: RadiansF,
	},
}
