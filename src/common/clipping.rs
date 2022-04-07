use super::{math_types::*};

pub fn clip_3d_polygon_by_z_plane(polygon: &[Vec3f], z_dist: f32, out_polygon: &mut [Vec3f]) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut out_vertex_count = 0;
	for v in polygon
	{
		if v.z > z_dist
		{
			if prev_v.z < z_dist
			{
				out_polygon[out_vertex_count] = get_line_z_intersection(prev_v, v, z_dist);
				out_vertex_count += 1;
				if out_vertex_count == out_polygon.len()
				{
					break;
				}
			}
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if v.z == z_dist
		{
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if prev_v.z > z_dist
		{
			out_polygon[out_vertex_count] = get_line_z_intersection(prev_v, v, z_dist);
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}

		prev_v = v;
	}

	out_vertex_count
}

pub fn get_line_z_intersection(v0: &Vec3f, v1: &Vec3f, z: f32) -> Vec3f
{
	let dist0 = v0.z - z;
	let dist1 = v1.z - z;
	let dist_sum = v1.z - v0.z;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
}

pub fn clip_2d_polygon(polygon: &[Vec2f], clip_line_equation: &Vec3f, out_polygon: &mut [Vec2f]) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut prev_dist = prev_v.dot(clip_line_equation.truncate()) - clip_line_equation.z;
	let mut out_vertex_count = 0;
	for v in polygon
	{
		let dist = v.dot(clip_line_equation.truncate()) - clip_line_equation.z;
		if dist > 0.0
		{
			if prev_dist < 0.0
			{
				out_polygon[out_vertex_count] = get_line_line_intersection(prev_v, v, clip_line_equation);
				out_vertex_count += 1;
				if out_vertex_count == out_polygon.len()
				{
					break;
				}
			}
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if dist == 0.0
		{
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if prev_dist > 0.0
		{
			out_polygon[out_vertex_count] = get_line_line_intersection(prev_v, v, clip_line_equation);
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}

		prev_v = v;
		prev_dist = dist;
	}

	out_vertex_count
}

pub fn get_line_line_intersection(v0: &Vec2f, v1: &Vec2f, line: &Vec3f) -> Vec2f
{
	let dist0 = v0.dot(line.truncate()) - line.z;
	let dist1 = v1.dot(line.truncate()) - line.z;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
}
