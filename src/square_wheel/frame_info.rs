use super::{light::*, resources_manager::*, triangle_model::*};
use common::{bbox::*, image::*, math_types::*, matrix::*};

pub struct FrameInfo
{
	pub camera_matrices: CameraMatrices,
	pub game_time_s: f32,
	pub model_entities: Vec<ModelEntity>,
	pub lights: Vec<PointLight>,
}

#[derive(Clone)]
pub struct ModelEntity
{
	pub position: Vec3f,
	pub angles: EulerAnglesF,
	pub animation: AnimationPoint,
	pub model: SharedResourcePtr<TriangleModel>,
	pub texture: SharedResourcePtr<Image>,

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
