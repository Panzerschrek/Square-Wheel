use super::math_types::*;

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
	let rotate_z = Mat4f::from_angle_z(-azimuth);
	let rotate_x = Mat4f::from_angle_x(-elevation);

	let mut basis_change = Mat4f::identity();
	basis_change.y.y = 0.0;
	basis_change.z.y = -1.0;
	basis_change.y.z = 1.0;
	basis_change.z.z = 0.0;

	complete_view_matrix(
		position,
		&(basis_change * rotate_x * rotate_z),
		fov,
		viewport_width,
		viewport_height,
	)
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
	// Perform Z and W manipulations only for view matrix, but not for planes equation matrix.
	let mut perspective_finalization = Mat4f::identity();
	perspective_finalization.w.z = 1.0;
	perspective_finalization.z.z = 0.0;
	perspective_finalization.z.w = 1.0;
	perspective_finalization.w.w = 0.0;

	let resize_to_viewport = Mat4f::from_nonuniform_scale(viewport_width * 0.5, viewport_height * 0.5, 1.0);
	let shift_to_viewport_center =
		Mat4f::from_translation(Vec3f::new(viewport_width * 0.5, viewport_height * 0.5, 0.0));

	let mut planes_shift_to_viewport_center = Mat4f::identity();
	planes_shift_to_viewport_center.x.z = -viewport_width * 0.5;
	planes_shift_to_viewport_center.y.z = -viewport_height * 0.5;

	// Perform transformations in reverse order in order to perform transformation via "matrix * vector".
	// TODO - perform calculations in "double" for better pericision?
	let base_view_matrix = resize_to_viewport * perspective * rotation_matrix * translate;
	// TODO - maybe avoid clculation of inverse matrix and perform direct matrix calculation?
	let planes_matrix = base_view_matrix.transpose().invert().unwrap();
	CameraMatrices {
		position,
		view_matrix: shift_to_viewport_center * perspective_finalization * base_view_matrix,
		planes_matrix: planes_shift_to_viewport_center * planes_matrix,
	}
}
