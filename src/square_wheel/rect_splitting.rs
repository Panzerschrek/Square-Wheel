use crate::common::math_types::*;

#[derive(Copy, Clone)]
pub struct Rect
{
	pub min: Vec2f,
	pub max: Vec2f,
}

impl Default for Rect
{
	fn default() -> Self
	{
		Self {
			min: Vec2f::zero(),
			max: Vec2f::zero(),
		}
	}
}

// Split rect into several subrects.
// Try to preserve equal area and make rects as square as possible.
pub fn split_rect(r: &Rect, num_parts: u32, out_parts: &mut [Rect])
{
	let mut num_out_parts = 0;
	split_rect_r(r, num_parts, out_parts, &mut num_out_parts);
}

fn split_rect_r(r: &Rect, num_parts: u32, out_parts: &mut [Rect], num_out_parts: &mut usize)
{
	if num_parts <= 1
	{
		if *num_out_parts >= out_parts.len()
		{
			return;
		}
		out_parts[*num_out_parts] = *r;
		*num_out_parts += 1;
		return;
	}

	let num_parts0 = num_parts / 2;
	let num_parts1 = num_parts - num_parts0;
	let ratio0 = (num_parts0 as f32) / (num_parts as f32);
	let ratio1 = (num_parts1 as f32) / (num_parts as f32);

	let r0;
	let r1;

	let width = r.max.x - r.min.x;
	let height = r.max.y - r.min.y;
	if width >= height
	{
		let middle = r.max.x * ratio0 + r.min.x * ratio1;
		r0 = Rect {
			min: Vec2f::new(r.min.x, r.min.y),
			max: Vec2f::new(middle, r.max.y),
		};
		r1 = Rect {
			min: Vec2f::new(middle, r.min.y),
			max: Vec2f::new(r.max.x, r.max.y),
		};
	}
	else
	{
		let middle = r.max.y * ratio0 + r.min.y * ratio1;
		r0 = Rect {
			min: Vec2f::new(r.min.x, r.min.y),
			max: Vec2f::new(r.max.x, middle),
		};
		r1 = Rect {
			min: Vec2f::new(r.min.x, middle),
			max: Vec2f::new(r.max.x, r.max.y),
		};
	}
	split_rect_r(&r0, num_parts0, out_parts, num_out_parts);
	split_rect_r(&r1, num_parts1, out_parts, num_out_parts);
}
