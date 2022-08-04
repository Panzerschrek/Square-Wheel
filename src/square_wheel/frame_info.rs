use super::{light::*, resources_manager::*, triangle_model::*};
use common::{bbox::*, image::*, material, math_types::*, matrix::*};

pub struct FrameInfo
{
	pub camera_matrices: CameraMatrices,
	pub game_time_s: f32,
	// submodels mapped 1 to 1 to initial submodels.
	pub submodel_entities: Vec<SubmodelEntityOpt>,
	pub model_entities: Vec<ModelEntity>,
	pub lights: Vec<PointLight>,
	pub skybox_rotation: QuaternionF,
}

pub type SubmodelEntityOpt = Option<SubmodelEntity>;

#[derive(Copy, Clone, PartialEq)]
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
	pub texture: SharedResourcePtr<Image>,
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
#[derive(Clone)]
pub struct AnimationPoint
{
	pub frames: [u32; 2],
	pub lerp: f32,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ModelLighting
{
	// Use just light from grid
	Default,
	// Use just constant light.
	ConstantLight([f32; 3]),
	// Complex case - use combination of light grid and constant color.
	AdvancedLight
	{
		grid_light_scale: f32,
		light_add: [f32; 3],
	},
}
