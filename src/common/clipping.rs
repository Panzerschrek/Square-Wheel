#![allow(clippy::float_cmp)]
use super::{math_types::*, plane::*};

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

pub fn clip_3d_polygon_by_plane(polygon: &[Vec3f], plane: &Plane, out_polygon: &mut [Vec3f]) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut prev_dist = plane.vec.dot(*prev_v);
	let mut out_vertex_count = 0;
	for v in polygon
	{
		let dist = plane.vec.dot(*v);
		if dist > plane.dist
		{
			if prev_dist < plane.dist
			{
				out_polygon[out_vertex_count] = get_line_plane_intersection(prev_v, v, plane);
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
		else if dist == plane.dist
		{
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if prev_dist > plane.dist
		{
			out_polygon[out_vertex_count] = get_line_plane_intersection(prev_v, v, plane);
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

fn get_line_line_intersection(v0: &Vec2f, v1: &Vec2f, line: &Vec3f) -> Vec2f
{
	let dist0 = v0.dot(line.truncate()) - line.z;
	let dist1 = v1.dot(line.truncate()) - line.z;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
}

pub fn get_line_plane_intersection(v0: &Vec3f, v1: &Vec3f, plane: &Plane) -> Vec3f
{
	let dist0 = v0.dot(plane.vec) - plane.dist;
	let dist1 = v1.dot(plane.vec) - plane.dist;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
}

#[derive(Copy, Clone)]
pub struct ModelVertex2d
{
	pub pos: Vec2f,
	pub tc: Vec2f,
}

#[derive(Copy, Clone)]
pub struct ModelVertex3d
{
	pub pos: Vec3f,
	pub tc: Vec2f,
}

pub fn clip_2d_model_polygon(
	polygon: &[ModelVertex2d],
	clip_line_equation: &Vec3f,
	out_polygon: &mut [ModelVertex2d],
) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut prev_dist = prev_v.pos.dot(clip_line_equation.truncate()) - clip_line_equation.z;
	let mut out_vertex_count = 0;
	for v in polygon
	{
		let dist = v.pos.dot(clip_line_equation.truncate()) - clip_line_equation.z;
		if dist > 0.0
		{
			if prev_dist < 0.0
			{
				out_polygon[out_vertex_count] = get_model_line_line_intersection(prev_v, v, clip_line_equation);
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
			out_polygon[out_vertex_count] = get_model_line_line_intersection(prev_v, v, clip_line_equation);
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

pub fn clip_3d_model_polygon_by_plane(
	polygon: &[ModelVertex3d],
	plane: &Plane,
	out_polygon: &mut [ModelVertex3d],
) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut prev_dist = plane.vec.dot(prev_v.pos);
	let mut out_vertex_count = 0;
	for v in polygon
	{
		let dist = plane.vec.dot(v.pos);
		if dist > plane.dist
		{
			if prev_dist < plane.dist
			{
				out_polygon[out_vertex_count] = get_model_line_plane_intersection(prev_v, v, plane);
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
		else if dist == plane.dist
		{
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if prev_dist > plane.dist
		{
			out_polygon[out_vertex_count] = get_model_line_plane_intersection(prev_v, v, plane);
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

fn get_model_line_line_intersection(v0: &ModelVertex2d, v1: &ModelVertex2d, line: &Vec3f) -> ModelVertex2d
{
	let dist0 = v0.pos.dot(line.truncate()) - line.z;
	let dist1 = v1.pos.dot(line.truncate()) - line.z;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	ModelVertex2d {
		pos: v0.pos * k1 - v1.pos * k0,
		tc: v0.tc * k1 - v1.tc * k0,
	}
}

fn get_model_line_plane_intersection(v0: &ModelVertex3d, v1: &ModelVertex3d, plane: &Plane) -> ModelVertex3d
{
	let dist0 = v0.pos.dot(plane.vec) - plane.dist;
	let dist1 = v1.pos.dot(plane.vec) - plane.dist;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	ModelVertex3d {
		pos: v0.pos * k1 - v1.pos * k0,
		tc: v0.tc * k1 - v1.tc * k0,
	}
}
