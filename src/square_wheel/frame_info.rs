use super::{resources_manager::*, textures::*, triangle_model::*};
use crate::common::{bbox::*, material::BlendingMode, math_types::*, matrix::*, plane::*};
use serde::{Deserialize, Serialize};

// This file contains structs, used to describe world state for Renderer in order to draw a frame properly.

pub struct FrameInfo
{
	// Root view info - for image, rendered on screen.
	pub view: FrameViewInfo,
	pub world: FrameWorldInfo,
}

// View-associated frame info.
pub struct FrameViewInfo
{
	pub camera_matrices: CameraMatrices,

	// How to modulate output image color. Use 1 for normal cases.
	// Use values greater than 1 to overexpose image.
	// Use dark colored value in order to draw fullscreen blood or underwater effect.
	// Use values only greater than zero!
	pub color_modulate: [f32; 3],

	pub is_third_person_view: bool,
}

// Frame info, independent on view point.
// Game code should avoid building this depending on view point, for example, removing objects far from camera,
// since it is possible to render view from another point via portals/mirrors.
pub struct FrameWorldInfo
{
	pub game_time_s: f32,
	pub skybox_rotation: QuaternionF,
	// submodels mapped 1 to 1 to initial submodels.
	pub submodel_entities: Vec<SubmodelEntityOpt>,
	pub model_entities: Vec<ModelEntity>,
	pub decals: Vec<Decal>,
	pub sprites: Vec<Sprite>,
	pub lights: Vec<DynamicLight>,
	pub portals: Vec<ViewPortal>,
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
	pub blending_mode: BlendingMode,
	pub lighting: ModelLighting,
	pub flags: ModelEntityDrawFlags,

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

bitflags::bitflags! {
#[derive(Serialize, Deserialize)]
pub struct ModelEntityDrawFlags: u8
{
	// Weapon or thing in player's hands.
	// Draw it always and after any other models.
	const VIEW_MODEL = 1;
	// Draw only in portals/mirrors, but not from intial view point.
	const ONLY_THIRD_PERSON_VIEW = 2;
}
}

#[derive(Clone)]
pub struct Decal
{
	// Decal primitive is cube with half-size = 1.
	pub position: Vec3f,
	pub rotation: QuaternionF,
	pub scale: Vec3f,
	pub texture: SharedResourcePtr<TextureLiteWithMips>,
	pub blending_mode: BlendingMode,
	pub lightmap_light_scale: f32,
	pub light_add: [f32; 3],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sprite
{
	pub position: Vec3f,
	pub angle: f32, // Rotation around axis, perpendicular to sprite plane.
	pub radius: f32,
	pub texture: SharedResourcePtr<TextureLiteWithMips>,
	pub blending_mode: BlendingMode,
	// Describe only the way to obtain result vertices of the sprite, instead of describing vertices itself.
	// This is needed in order to draw sprites properly in portals/mirrors.
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

#[derive(Clone)]
pub struct ViewPortal
{
	pub view: PortalView,
	pub plane: Plane,
	pub tex_coord_equation: [Plane; 2],
	pub vertices: Vec<Vec3f>,
	pub blending_mode: BlendingMode,
	pub texture: Option<ViewPortalTexture>,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PortalView
{
	CameraAtPosition
	{
		position: Vec3f,
		rotation: QuaternionF,
		fov: RadiansF,
	},
	Mirror {},
	ParallaxPortal
	{
		// Matrix that transforms camera position into camera position of view point of this portal.
		transform_matrix: Mat4f,
	},
}

#[derive(Clone)]
pub struct ViewPortalTexture
{
	pub blending_mode: BlendingMode,
	pub texture: SharedResourcePtr<TextureLiteWithMips>,
	pub light_scale: f32,
	pub light_add: [f32; 3],
}
