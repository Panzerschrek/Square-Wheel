use crate::common::{bbox::*, math_types::*, matrix::*, plane::*};

// Class for sorting objects inside a leaf.
// uses internal storage and has limit for number of objects.
// Allocate it once per frame and reuse for sorting of objects in leafs.
pub struct LeafObjectsSorter
{
	objects: [BBoxForDrawOrdering; MAX_OBJECTS],
	num_objects: usize,
}

impl LeafObjectsSorter
{
	pub fn new() -> Self
	{
		Self {
			objects: [BBoxForDrawOrdering::default(); MAX_OBJECTS],
			num_objects: 0,
		}
	}

	pub fn add_object(&mut self, object: BBoxForDrawOrdering)
	{
		if self.num_objects >= MAX_OBJECTS
		{
			return;
		}
		self.objects[self.num_objects] = object;
		self.num_objects += 1;
	}

	pub fn clear(&mut self)
	{
		self.num_objects = 0;
	}

	pub fn sort_objects(&mut self)
	{
		order_bboxes(&mut self.objects[.. self.num_objects]);
	}

	pub fn get_objects(&self) -> &[BBoxForDrawOrdering]
	{
		&self.objects[.. self.num_objects]
	}
}

const MAX_OBJECTS: usize = 48;

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

fn order_bboxes(bboxes: &mut [BBoxForDrawOrdering])
{
	if bboxes.len() <= 1
	{
		return;
	}

	// Use simple bubble sorting.
	// We can't use library sorting because we can't implement reliable comparator.
	// It is fine to use quadratic algorithm here since number of models is relatively small.

	// TODO - maybe run sorting multiple times to resolve complex situations?

	for i in 0 .. bboxes.len() - 1
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
	// Of each pair of bbox planes select plane facing towards camera (if some).
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
	bbox.get_corners_vertices()
		.map(|pos| view_matrix_transform_vertex(view_matrix, &pos))
}

fn compare_projected_bboxes(l: &ProjectedBBox, r: &ProjectedBBox) -> bool
{
	// If one of bboxes lies totally at front of one of another's bbox planes, we can determine proper order.
	for l_plane in &l.planes
	{
		if is_bbox_at_front_of_plane(l_plane, &r.vertices)
		{
			return false;
		}
	}

	for r_plane in &r.planes
	{
		if is_bbox_at_front_of_plane(r_plane, &l.vertices)
		{
			return true;
		}
	}

	// Hard case - just compare closest points.
	get_projected_bbox_min_z(l) < get_projected_bbox_min_z(r)
}

fn is_bbox_at_front_of_plane(plane: &Plane, bbox_vertices: &BBoxVerticesProjected) -> bool
{
	if plane.dist >= 0.0
	{
		// This plane facing away from camera, ignore it.
		return false;
	}

	let mut vertices_front = 0;
	for v in bbox_vertices
	{
		if plane.vec.dot(*v) >= plane.dist
		{
			vertices_front += 1;
		}
	}

	vertices_front == bbox_vertices.len()
}

fn get_projected_bbox_min_z(b: &ProjectedBBox) -> f32
{
	let mut r = b.vertices[0].z;
	for v in b.vertices
	{
		r = r.min(v.z);
	}
	r
}
