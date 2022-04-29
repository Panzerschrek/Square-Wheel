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
	// Try to determine if one bbox is closer than another bbox checking bbox ranges against camera position for 3 axis.

	let l_min: &[f32; 3] = l.min.as_ref();
	let l_max: &[f32; 3] = l.max.as_ref();
	let r_min: &[f32; 3] = r.min.as_ref();
	let r_max: &[f32; 3] = r.max.as_ref();
	let cam_pos: &[f32; 3] = camera_position.as_ref();

	for i in 0 .. 3
	{
		let ranges_dist = get_ranges_dist(l_min[i], l_max[i], r_min[i], r_max[i]);
		if ranges_dist == 0.0
		{
			// Overlapping ranges - can't determine proper order for this axis.
			continue;
		}

		let dist_l = get_point_range_dist(l_min[i], l_max[i], cam_pos[i]);
		let dist_r = get_point_range_dist(r_min[i], r_max[i], cam_pos[i]);
		if dist_l < dist_r
		{
			return true;
		}
		if dist_r < dist_l
		{
			return false;
		}
	}

	// There is no non-overlapping ranges.
	// Try to reorder models, using simple distance criteria.
	for i in 0 .. 3
	{
		let dist_l = get_point_range_dist(l_min[i], l_max[i], cam_pos[i]);
		let dist_r = get_point_range_dist(r_min[i], r_max[i], cam_pos[i]);
		if dist_l < dist_r
		{
			return true;
		}
		if dist_r < dist_l
		{
			return false;
		}
	}

	// Can't determine order at all.
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

// Get distance (non-negative) between two ranges.
// Returns 0 if ranges overlaps.
fn get_ranges_dist(l_min: f32, l_max: f32, r_min: f32, r_max: f32) -> f32
{
	if l_max < r_min
	{
		r_min - l_max
	}
	else if r_max < l_min
	{
		l_min - r_max
	}
	else
	{
		0.0
	}
}
