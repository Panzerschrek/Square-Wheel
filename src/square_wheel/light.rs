use common::math_types::*;

pub struct PointLight
{
	pub pos: Vec3f,
	pub color: [f32; 3], // Color scaled by intencity.
}
