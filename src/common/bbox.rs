use super::math_types::*;

#[derive(Copy, Clone)]
pub struct BBox
{
	pub min: Vec3f,
	pub max: Vec3f,
}

impl BBox
{
	pub fn from_point(point: &Vec3f) -> Self
	{
		Self {
			min: *point,
			max: *point,
		}
	}

	pub fn extend_with_point(&mut self, point: &Vec3f)
	{
		if point.x < self.min.x
		{
			self.min.x = point.x;
		}
		if point.x > self.max.x
		{
			self.max.x = point.x;
		}
		if point.y < self.min.y
		{
			self.min.y = point.y;
		}
		if point.y > self.max.y
		{
			self.max.y = point.y;
		}
		if point.z < self.min.z
		{
			self.min.z = point.z;
		}
		if point.z > self.max.z
		{
			self.max.z = point.z;
		}
	}
}
