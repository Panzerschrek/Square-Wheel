use common::{bbox::*, math_types::*};

pub type ModelForDrawOrdering = (u32, BBox);

pub fn order_models(models: &mut [ModelForDrawOrdering], camera_position: &Vec3f)
{
	if models.len() <= 1
	{
		return;
	}

	// Use simple bubble sorting.
	// We can't use library sorting because we can't implement reliable comparator.
	// It is fine to use quadratic algorithm here since number of models is relatively small.

	// TODO - maybe run sorting multiple times to resolve complex situations?

	for i in 0 .. models.len()
	{
		for j in i + 1 .. models.len()
		{
			if compare_models(&models[i].1, &models[j].1, camera_position)
			{
				models.swap(i, j);
			}
		}
	}
}

fn compare_models(l: &BBox, r: &BBox, camera_position: &Vec3f) -> bool
{
	// Try to determine if one bbox is closer than another bbox checking 3 axis.

	let l_min: &[f32; 3] = l.min.as_ref();
	let l_max: &[f32; 3] = l.max.as_ref();
	let r_min: &[f32; 3] = r.min.as_ref();
	let r_max: &[f32; 3] = r.max.as_ref();
	let cam_pos: &[f32; 3] = camera_position.as_ref();

	for i in 0 .. 3
	{
		let dist_l = get_point_range_dist(l_min[i], l_max[i], cam_pos[i]);
		let dist_r = get_point_range_dist(r_min[i], r_max[i], cam_pos[i]);

		// TODO - check this properly.
		if dist_l < dist_r
		{
			return false;
		}
		if dist_r > dist_l
		{
			return true;
		}
	}

	// TODO - try to perform additional checks here.

	false
}

// Get distance (non-negative) from point to range.
// Returns 0 if point is inside range.
fn get_point_range_dist(range_min: f32, range_max: f32, point: f32) -> f32
{
	if point < range_min
	{
		range_min - point
	}
	else if point > range_max
	{
		point - range_max
	}
	else
	{
		0.0
	}
}
