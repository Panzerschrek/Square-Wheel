use common::{camera_controller::*, math_types::*};

pub enum CubeMapSide
{
	XPlus,
	XMinus,
	YPlus,
	YMinus,
	ZPlus,
	ZMinus,
}

pub fn calculate_cube_shadow_map_side_matrices(
	position: Vec3f,
	shadow_map_size: f32,
	side: CubeMapSide,
) -> CameraMatrices
{
	let translate = Mat4f::from_translation(-position);
	let mut side_mat = get_cube_map_side_matrix(side);

	// Perform Z and W manipulations only for projection, but not for planes equation matrix.
	let mut perspective_finalization = Mat4f::identity();
	perspective_finalization.w.z = 1.0;
	perspective_finalization.z.z = 0.0;
	perspective_finalization.z.w = 1.0;
	perspective_finalization.w.w = 0.0;

	let resize_to_viewport = Mat4f::from_nonuniform_scale(shadow_map_size * 0.5, shadow_map_size * 0.5, 1.0);
	let shift_to_viewport_center =
		Mat4f::from_translation(Vec3f::new(shadow_map_size * 0.5, shadow_map_size * 0.5, 0.0));

	let base_view_matrix = resize_to_viewport * side_mat * translate;

	let planes_matrix = base_view_matrix.transpose().invert().unwrap();
	CameraMatrices {
		position,
		view_matrix: shift_to_viewport_center * perspective_finalization * base_view_matrix,
		planes_matrix,
	}
}

fn get_cube_map_side_matrix(side: CubeMapSide) -> Mat4f
{
	let mut mat = Mat4f::identity();
	match side
	{
		CubeMapSide::XPlus =>
		{
			mat.x.x = 0.0;
			mat.x.y = 0.0;
			mat.x.z = 1.0;
			mat.y.x = -1.0;
			mat.y.y = 0.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::XMinus =>
		{
			mat.x.x = 0.0;
			mat.x.y = 0.0;
			mat.x.z = -1.0;
			mat.y.x = 1.0;
			mat.y.y = 0.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::YPlus =>
		{
			mat.x.x = 1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = 0.0;
			mat.y.z = 1.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::YMinus =>
		{
			mat.x.x = -1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = 0.0;
			mat.y.z = -1.0;
			mat.z.x = 0.0;
			mat.z.y = -1.0;
			mat.z.z = 0.0;
		},
		CubeMapSide::ZPlus =>
		{
			mat.x.x = 1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = 1.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = 0.0;
			mat.z.z = 1.0;
		},
		CubeMapSide::ZMinus =>
		{
			mat.x.x = 1.0;
			mat.x.y = 0.0;
			mat.x.z = 0.0;
			mat.y.x = 0.0;
			mat.y.y = -1.0;
			mat.y.z = 0.0;
			mat.z.x = 0.0;
			mat.z.y = 0.0;
			mat.z.z = -1.0;
		},
	}
	mat
}
