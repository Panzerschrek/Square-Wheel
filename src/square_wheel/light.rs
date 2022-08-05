use crate::common::math_types::*;

#[derive(Copy, Clone)]
pub struct PointLight
{
	pub pos: Vec3f,
	pub color: [f32; 3], // Color scaled by intencity.
}
