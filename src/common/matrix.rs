use super::math_types::*;

#[derive(Copy, Clone)]
pub struct CameraMatrices
{
	pub position: Vec3f,
	// Matrix used for vertices projection. Viewport size scale and shift applied.
	pub view_matrix: Mat4f,
	// Matrix used for transformation of plane equations. Viewport center shift is not applied.
	pub planes_matrix: Mat4f,
}

pub fn build_view_matrix(
	position: Vec3f,
	azimuth: RadiansF,
	elevation: RadiansF,
	fov: f32,
	viewport_width: f32,
	viewport_height: f32,
) -> CameraMatrices
{
	build_view_matrix_with_full_rotation(
		position,
		QuaternionF::from_angle_z(azimuth + Rad(std::f32::consts::PI * 0.5)) * QuaternionF::from_angle_y(-elevation),
		fov,
		viewport_width,
		viewport_height,
	)
}

pub fn build_view_matrix_with_full_rotation(
	position: Vec3f,
	rotation: QuaternionF,
	fov: f32,
	viewport_width: f32,
	viewport_height: f32,
) -> CameraMatrices
{
	let rotate = Mat4f::from(rotation.conjugate());

	let mut basis_change = Mat4f::identity();
	basis_change.x.x = 0.0;
	basis_change.y.y = 0.0;
	basis_change.z.z = 0.0;
	basis_change.x.z = 1.0;
	basis_change.y.x = -1.0;
	basis_change.z.y = -1.0;

	complete_view_matrix(position, &(basis_change * rotate), fov, viewport_width, viewport_height)
}

pub fn complete_view_matrix(
	position: Vec3f,
	rotation_matrix: &Mat4f,
	fov: f32,
	viewport_width: f32,
	viewport_height: f32,
) -> CameraMatrices
{
	let translate = Mat4f::from_translation(-position);

	let inv_half_fov_tan = 1.0 / ((fov * 0.5).tan());
	let aspect = viewport_width / viewport_height;
	let perspective = Mat4f::from_nonuniform_scale(inv_half_fov_tan / aspect, inv_half_fov_tan, 1.0);

	let resize_to_viewport = Mat4f::from_nonuniform_scale(viewport_width * 0.5, viewport_height * 0.5, 1.0);

	let mut shift_to_viewport_center = Mat4f::identity();
	shift_to_viewport_center.z.x = viewport_width * 0.5;
	shift_to_viewport_center.z.y = viewport_height * 0.5;

	// Perform transformations in reverse order in order to perform transformation via "matrix * vector".
	// TODO - perform calculations in "double" for better pericision?
	let base_view_matrix = shift_to_viewport_center * resize_to_viewport * perspective * rotation_matrix * translate;
	CameraMatrices {
		position,
		view_matrix: base_view_matrix,
		// TODO - maybe avoid calculation of inverse matrix and perform direct matrix calculation?
		planes_matrix: base_view_matrix.transpose().invert().unwrap(),
	}
}

pub fn get_object_matrix(position: Vec3f, rotation: QuaternionF) -> Mat4f
{
	let rotate = Mat4f::from(rotation);

	let translate = Mat4f::from_translation(position);
	translate * rotate
}

pub fn get_object_matrix_with_scale(position: Vec3f, rotation: QuaternionF, scale: Vec3f) -> Mat4f
{
	get_object_matrix(position, rotation) * Mat4f::from_nonuniform_scale(scale.x, scale.y, scale.z)
}

// Transform vertex into screen, using CameraMatrices::view_matrix, that was previously prepared via "complete_view_matrix" function.
pub fn view_matrix_transform_vertex(view_matrix: &Mat4f, vertex: &Vec3f) -> Vec3f
{
	(view_matrix * vertex.extend(1.0)).truncate()
}
