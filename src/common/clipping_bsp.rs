use super::{clipping, map_polygonizer, math_types::*, plane::*};

use map_polygonizer::Polygon;

#[derive(PartialEq, Eq)]
pub enum PolygonPositionRelativePlane
{
	Front,
	Back,
	CoplanarFront,
	CoplanarBack,
	Splitted,
}

pub fn get_polygon_position_relative_plane(polygon: &Polygon, plane: &Plane) -> PolygonPositionRelativePlane
{
	let mut vertices_front = 0;
	let mut vertices_back = 0;
	for v in &polygon.vertices
	{
		match get_point_position_relative_plane(v, plane)
		{
			PointPositionRelativePlane::Front =>
			{
				vertices_front += 1;
			},
			PointPositionRelativePlane::Back =>
			{
				vertices_back += 1;
			},
			PointPositionRelativePlane::OnPlane =>
			{},
		};
	}

	if vertices_front != 0 && vertices_back != 0
	{
		PolygonPositionRelativePlane::Splitted
	}
	else if vertices_front != 0
	{
		PolygonPositionRelativePlane::Front
	}
	else if vertices_back != 0
	{
		PolygonPositionRelativePlane::Back
	}
	else if polygon.plane.vec.dot(plane.vec) >= 0.0
	{
		PolygonPositionRelativePlane::CoplanarFront
	}
	else
	{
		PolygonPositionRelativePlane::CoplanarBack
	}
}

#[derive(PartialEq, Eq)]
pub enum PointPositionRelativePlane
{
	Front,
	Back,
	OnPlane,
}

pub const POINT_POSITION_EPS: f32 = 1.0 / 16.0;

pub fn get_point_position_relative_plane(point: &Vec3f, plane: &Plane) -> PointPositionRelativePlane
{
	// Polygon vector is unnormalized. So, scale epsilon to length of this vector.
	let dist_scaled = point.dot(plane.vec) - plane.dist;
	let eps_scaled = POINT_POSITION_EPS * plane.vec.magnitude();
	if dist_scaled > eps_scaled
	{
		PointPositionRelativePlane::Front
	}
	else if dist_scaled < -eps_scaled
	{
		PointPositionRelativePlane::Back
	}
	else
	{
		PointPositionRelativePlane::OnPlane
	}
}

// Returns pair of front and back polygons.
pub fn split_polygon(in_polygon: &Polygon, plane: &Plane) -> (Polygon, Polygon)
{
	let mut polygon_front = Polygon {
		plane: in_polygon.plane,
		texture_info: in_polygon.texture_info.clone(),
		vertices: Vec::new(),
	};
	let mut polygon_back = Polygon {
		plane: in_polygon.plane,
		texture_info: in_polygon.texture_info.clone(),
		vertices: Vec::new(),
	};

	let mut prev_vert = in_polygon.vertices.last().unwrap();
	let mut prev_vert_pos = get_point_position_relative_plane(prev_vert, plane);
	for vert in &in_polygon.vertices
	{
		let vert_pos = get_point_position_relative_plane(vert, plane);

		match vert_pos
		{
			PointPositionRelativePlane::Front =>
			{
				if prev_vert_pos == PointPositionRelativePlane::Back
				{
					let intersection = clipping::get_line_plane_intersection(prev_vert, vert, plane);
					polygon_back.vertices.push(intersection);
					polygon_front.vertices.push(intersection);
				}
				polygon_front.vertices.push(*vert);
			},
			PointPositionRelativePlane::Back =>
			{
				if prev_vert_pos == PointPositionRelativePlane::Front
				{
					let intersection = clipping::get_line_plane_intersection(prev_vert, vert, plane);
					polygon_front.vertices.push(intersection);
					polygon_back.vertices.push(intersection);
				}
				polygon_back.vertices.push(*vert);
			},
			PointPositionRelativePlane::OnPlane =>
			{
				polygon_front.vertices.push(*vert);
				polygon_back.vertices.push(*vert);
			},
		};

		prev_vert = vert;
		prev_vert_pos = vert_pos;
	}

	(polygon_front, polygon_back)
}
