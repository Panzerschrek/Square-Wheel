use common::{bbox::*, math_types::*, matrix::*, plane::*};

pub type BBoxForDrawOrdering = (u32, ProjectedBBox);

#[derive(Copy, Clone)]
pub struct ProjectedBBox
{
	planes: BBoxPlanesProjected,
	vertices: BBoxVerticesProjected,
}

impl Default for ProjectedBBox
{
	fn default() -> Self
	{
		Self {
			planes: [Plane {
				vec: Vec3f::zero(),
				dist: 0.0,
			}; 3],
			vertices: [Vec3f::zero(); 8],
		}
	}
}

// Store only visible sides of bbox.
pub type BBoxPlanesProjected = [Plane; 3];

pub type BBoxVerticesProjected = [Vec3f; 8];

pub fn project_bbox(bbox: &BBox, camera_matrices: &CameraMatrices) -> ProjectedBBox
{
	ProjectedBBox {
		planes: project_bbox_planes(bbox, &camera_matrices.planes_matrix),
		vertices: project_bbox_vertices(bbox, &camera_matrices.view_matrix),
	}
}

pub fn order_bboxes(bboxes: &mut [BBoxForDrawOrdering])
{
	if bboxes.len() <= 1
	{
		return;
	}

	// Use simple bubble sorting.
	// We can't use library sorting because we can't implement reliable comparator.
	// It is fine to use quadratic algorithm here since number of models is relatively small.

	// TODO - maybe run sorting multiple times to resolve complex situations?

	for i in 0 .. bboxes.len()
	{
		for j in i + 1 .. bboxes.len()
		{
			if compare_projected_bboxes(&bboxes[i].1, &bboxes[j].1)
			{
				bboxes.swap(i, j);
			}
		}
	}
}

fn project_bbox_planes(bbox: &BBox, planes_matrix: &Mat4f) -> BBoxPlanesProjected
{
	// Of each pair of bbox planes select plane facing towards camera.
	[
		[
			Plane {
				vec: Vec3f::unit_x(),
				dist: bbox.max.x,
			},
			Plane {
				vec: -Vec3f::unit_x(),
				dist: -bbox.min.x,
			},
		],
		[
			Plane {
				vec: Vec3f::unit_y(),
				dist: bbox.max.y,
			},
			Plane {
				vec: -Vec3f::unit_y(),
				dist: -bbox.min.y,
			},
		],
		[
			Plane {
				vec: Vec3f::unit_z(),
				dist: bbox.max.z,
			},
			Plane {
				vec: -Vec3f::unit_z(),
				dist: -bbox.min.z,
			},
		],
	]
	.map(|[p0, p1]| {
		let p0_projected = project_plane(&p0, planes_matrix);
		let p1_projected = project_plane(&p1, planes_matrix);
		if p0_projected.dist < 0.0
		{
			p0_projected
		}
		else
		{
			p1_projected
		}
	})
}

fn project_plane(plane: &Plane, planes_matrix: &Mat4f) -> Plane
{
	let plane_transformed_vec4 = planes_matrix * plane.vec.extend(-plane.dist);
	Plane {
		vec: plane_transformed_vec4.truncate(),
		dist: -plane_transformed_vec4.w,
	}
}

fn project_bbox_vertices(bbox: &BBox, view_matrix: &Mat4f) -> BBoxVerticesProjected
{
	[
		Vec3f::new(bbox.min.x, bbox.min.y, bbox.min.z),
		Vec3f::new(bbox.min.x, bbox.min.y, bbox.max.z),
		Vec3f::new(bbox.min.x, bbox.max.y, bbox.min.z),
		Vec3f::new(bbox.min.x, bbox.max.y, bbox.max.z),
		Vec3f::new(bbox.max.x, bbox.min.y, bbox.min.z),
		Vec3f::new(bbox.max.x, bbox.min.y, bbox.max.z),
		Vec3f::new(bbox.max.x, bbox.max.y, bbox.min.z),
		Vec3f::new(bbox.max.x, bbox.max.y, bbox.max.z),
	]
	.map(|pos| {
		let v_tranformed = view_matrix * pos.extend(1.0);
		Vec3f::new(v_tranformed.x, v_tranformed.y, v_tranformed.w)
	})
}

fn compare_projected_bboxes(l: &ProjectedBBox, r: &ProjectedBBox) -> bool
{
	// If one of bboxes lies totally at front of one of another's bbox planes, we can determine proper order.

	for l_plane in &l.planes
	{
		if is_bbox_at_front_of_plane(l_plane, &r.vertices)
		{
			return true;
		}
	}

	for r_plane in &r.planes
	{
		if is_bbox_at_front_of_plane(r_plane, &l.vertices)
		{
			return false;
		}
	}

	// Hard case - can't order. TODO - try to perform some ordering here.
	false
}

fn is_bbox_at_front_of_plane(plane: &Plane, bbox_vertices: &BBoxVerticesProjected) -> bool
{
	let mut vertices_front = 0;
	for v in bbox_vertices
	{
		if plane.vec.dot(*v) >= plane.dist
		{
			vertices_front += 1;
		}
	}

	vertices_front == 8
}
