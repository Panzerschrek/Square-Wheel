use super::{light::*, resources_manager::*, triangle_model::*};
use common::{image::*, math_types::*, matrix::*};

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
	pub angle_z: RadiansF,
	pub frame: u32,
	pub model: SharedResourcePtr<TriangleModel>,
	pub texture: SharedResourcePtr<Image>,
}
