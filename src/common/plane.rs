use super::math_types::*;

#[repr(C)] // Require C representation in order to get stable fileds order for binary serialization.
#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Plane
{
	pub vec: Vec3f, // Unnormalized direction
	pub dist: f32,  // for point on plane dot(vec, point) = dist
}

impl Plane
{
	pub fn get_inverted(self) -> Self
	{
		Plane {
			vec: -self.vec,
			dist: -self.dist,
		}
	}
}
