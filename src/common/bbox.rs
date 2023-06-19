use super::math_types::*;

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct BBox
{
	pub min: Vec3f,
	pub max: Vec3f,
}

impl BBox
{
	pub fn zero() -> Self
	{
		Self {
			min: Vec3f::zero(),
			max: Vec3f::zero(),
		}
	}

	pub fn from_min_max(min: Vec3f, max: Vec3f) -> Self
	{
		Self { min, max }
	}

	pub fn from_point(point: &Vec3f) -> Self
	{
		Self {
			min: *point,
			max: *point,
		}
	}

	pub fn is_empty_or_invalid(&self) -> bool
	{
		!self.is_valid_and_non_empty()
	}

	pub fn is_valid_and_non_empty(&self) -> bool
	{
		self.min.x < self.max.x && self.min.y < self.max.y && self.min.z < self.max.z
	}

	pub fn contains(&self, other: &BBox) -> bool
	{
		self.min.x <= other.min.x &&
			self.max.x >= other.max.x &&
			self.min.y <= other.min.y &&
			self.max.y >= other.max.y &&
			self.min.z <= other.min.z &&
			self.max.z >= other.max.z
	}

	pub fn contains_point(&self, point: &Vec3f) -> bool
	{
		point.x >= self.min.x &&
			point.x <= self.max.x &&
			point.y >= self.min.y &&
			point.y <= self.max.y &&
			point.z >= self.min.z &&
			point.z <= self.max.z
	}

	pub fn touches_or_intersects(&self, other: &BBox) -> bool
	{
		!(self.max.x < other.min.x ||
			self.min.x > other.max.x ||
			self.max.y < other.min.y ||
			self.min.y > other.max.y ||
			self.max.z < other.min.z ||
			self.min.z > other.max.z)
	}

	pub fn get_center(&self) -> Vec3f
	{
		(self.min + self.max) * 0.5
	}

	pub fn get_size(&self) -> Vec3f
	{
		self.max - self.min
	}

	pub fn get_corners_vertices(&self) -> [Vec3f; 8]
	{
		[
			Vec3f::new(self.min.x, self.min.y, self.min.z),
			Vec3f::new(self.min.x, self.min.y, self.max.z),
			Vec3f::new(self.min.x, self.max.y, self.min.z),
			Vec3f::new(self.min.x, self.max.y, self.max.z),
			Vec3f::new(self.max.x, self.min.y, self.min.z),
			Vec3f::new(self.max.x, self.min.y, self.max.z),
			Vec3f::new(self.max.x, self.max.y, self.min.z),
			Vec3f::new(self.max.x, self.max.y, self.max.z),
		]
	}

	pub fn extend(&mut self, other: &BBox)
	{
		if other.min.x < self.min.x
		{
			self.min.x = other.min.x;
		}
		if other.max.x > self.max.x
		{
			self.max.x = other.max.x;
		}
		if other.min.y < self.min.y
		{
			self.min.y = other.min.y;
		}
		if other.max.y > self.max.y
		{
			self.max.y = other.max.y;
		}
		if other.min.z < self.min.z
		{
			self.min.z = other.min.z;
		}
		if other.max.z > self.max.z
		{
			self.max.z = other.max.z;
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
