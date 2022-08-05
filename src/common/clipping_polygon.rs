use super::math_types::*;

// 2d Clipping polygon. Has small number of fixed sizes.
#[derive(Copy, Clone, Default)]
pub struct ClippingPolygon
{
	// Now it is just an axis-aligned octagon.
	x: ClipAxis,
	y: ClipAxis,
	x_plus_y: ClipAxis,
	x_minus_y: ClipAxis,
}

pub type ClippingPolygonPlanes = [Vec3f; 8];
pub type ClippingPolygonBoxPlanes = [Vec3f; 4];

impl ClippingPolygon
{
	pub fn from_box(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self
	{
		Self {
			x: ClipAxis { min: min_x, max: max_x },
			y: ClipAxis { min: min_y, max: max_y },
			x_plus_y: ClipAxis {
				min: min_x + min_y,
				max: max_x + max_y,
			},
			x_minus_y: ClipAxis {
				min: min_x - max_y,
				max: max_x - min_y,
			},
		}
	}

	pub fn from_point(point: &Vec2f) -> Self
	{
		Self {
			x: ClipAxis {
				min: point.x,
				max: point.x,
			},
			y: ClipAxis {
				min: point.y,
				max: point.y,
			},
			x_plus_y: ClipAxis {
				min: point.x + point.y,
				max: point.x + point.y,
			},
			x_minus_y: ClipAxis {
				min: point.x - point.y,
				max: point.x - point.y,
			},
		}
	}

	pub fn is_empty_or_invalid(&self) -> bool
	{
		!self.is_valid_and_non_empty()
	}

	pub fn is_valid_and_non_empty(&self) -> bool
	{
		self.x.is_valid_and_non_empty() &&
			self.y.is_valid_and_non_empty() &&
			self.x_plus_y.is_valid_and_non_empty() &&
			self.x_minus_y.is_valid_and_non_empty()
	}

	pub fn contains(&self, other: &ClippingPolygon) -> bool
	{
		self.x.contains(&other.x) &&
			self.y.contains(&other.y) &&
			self.x_plus_y.contains(&other.x_plus_y) &&
			self.x_minus_y.contains(&other.x_minus_y)
	}

	// Result polygon will contain both "self" and "other".
	pub fn extend(&mut self, other: &ClippingPolygon)
	{
		self.x.extend(&other.x);
		self.y.extend(&other.y);
		self.x_plus_y.extend(&other.x_plus_y);
		self.x_minus_y.extend(&other.x_minus_y);
	}

	pub fn extend_with_polygon(&mut self, polygon_points: &[Vec2f])
	{
		for point in polygon_points
		{
			self.extend_with_point(point);
		}
	}

	pub fn extend_with_point(&mut self, point: &Vec2f)
	{
		self.x.extend_with_point(point.x);
		self.y.extend_with_point(point.y);
		self.x_plus_y.extend_with_point(point.x + point.y);
		self.x_minus_y.extend_with_point(point.x - point.y);
	}

	// Both "self" and "other" will contain result polygon.
	pub fn intersect(&mut self, other: &ClippingPolygon)
	{
		self.x.intersect(&other.x);
		self.y.intersect(&other.y);
		self.x_plus_y.intersect(&other.x_plus_y);
		self.x_minus_y.intersect(&other.x_minus_y);
	}

	// Input polygon must be non-empty.
	pub fn intersect_with_polygon(&mut self, polygon_points: &[Vec2f])
	{
		let mut points_bound = Self::from_point(&polygon_points[0]);
		for point in &polygon_points[1 ..]
		{
			points_bound.extend_with_point(point);
		}

		self.intersect(&points_bound);
	}

	pub fn scale_relative_center(&mut self, scale: f32)
	{
		self.x.scale_relative_center(scale);
		self.y.scale_relative_center(scale);
		self.x_plus_y.scale_relative_center(scale);
		self.x_minus_y.scale_relative_center(scale);
	}

	pub fn increase(&mut self, delta: f32)
	{
		self.x.increase(delta);
		self.y.increase(delta);
		self.x_plus_y.increase(delta);
		self.x_minus_y.increase(delta);
	}

	pub fn get_clip_planes(&self) -> ClippingPolygonPlanes
	{
		[
			Vec3f::new(-1.0, 0.0, -self.x.max),
			Vec3f::new(1.0, 0.0, self.x.min),
			Vec3f::new(0.0, -1.0, -self.y.max),
			Vec3f::new(0.0, 1.0, self.y.min),
			Vec3f::new(-1.0, -1.0, -self.x_plus_y.max),
			Vec3f::new(1.0, 1.0, self.x_plus_y.min),
			Vec3f::new(-1.0, 1.0, -self.x_minus_y.max),
			Vec3f::new(1.0, -1.0, self.x_minus_y.min),
		]
	}

	pub fn get_box_clip_planes(&self) -> ClippingPolygonBoxPlanes
	{
		[
			Vec3f::new(-1.0, 0.0, -self.x.max),
			Vec3f::new(1.0, 0.0, self.x.min),
			Vec3f::new(0.0, -1.0, -self.y.max),
			Vec3f::new(0.0, 1.0, self.y.min),
		]
	}
}

#[derive(Copy, Clone, Default)]
struct ClipAxis
{
	min: f32,
	max: f32,
}

#[allow(dead_code)]
impl ClipAxis
{
	pub fn is_empty_or_invalid(&self) -> bool
	{
		!self.is_valid_and_non_empty()
	}

	pub fn is_valid_and_non_empty(&self) -> bool
	{
		self.min < self.max
	}

	fn contains(&self, other: &ClipAxis) -> bool
	{
		other.min >= self.min && other.max <= self.max
	}

	fn extend(&mut self, other: &ClipAxis)
	{
		if other.min < self.min
		{
			self.min = other.min;
		}
		if other.max > self.max
		{
			self.max = other.max;
		}
	}

	fn extend_with_point(&mut self, point: f32)
	{
		if point < self.min
		{
			self.min = point;
		}
		if point > self.max
		{
			self.max = point;
		}
	}

	fn intersect(&mut self, other: &ClipAxis)
	{
		if other.min > self.min
		{
			self.min = other.min;
		}
		if other.max < self.max
		{
			self.max = other.max;
		}
	}

	fn scale_relative_center(&mut self, scale: f32)
	{
		let center = (self.min + self.max) * 0.5;
		let half_size_scaled = (self.max - self.min) * 0.5 * scale;
		self.min = center - half_size_scaled;
		self.max = center + half_size_scaled;
	}

	fn increase(&mut self, delta: f32)
	{
		self.min -= delta;
		self.max += delta;
	}
}
