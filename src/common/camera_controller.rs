use super::math_types::*;
use sdl2::keyboard::Scancode;

pub struct CameraController
{
	pos: Vec3f,
	azimuth: RadiansF,
	elevation: RadiansF,
}

impl CameraController
{
	pub fn new() -> Self
	{
		CameraController {
			pos: Vec3f::new(0.0, 0.0, 0.0),
			azimuth: RadiansF::zero(),
			elevation: RadiansF::zero(),
		}
	}

	pub fn update(&mut self, keyboard_state: &sdl2::keyboard::KeyboardState, time_delta_s: f32)
	{
		const SPEED: f32 = 512.0;
		const JUMP_SPEED: f32 = 0.8 * SPEED;
		const ANGLE_SPEED: RadiansF = Rad(1.0);
		const PI: RadiansF = Rad(std::f32::consts::PI);
		let half_pi = PI / 2.0;
		let two_pi = PI * 2.0;

		let forward_vector = Vec3f::new(-(self.azimuth.sin()), self.azimuth.cos(), 0.0);
		let left_vector = Vec3f::new(self.azimuth.cos(), self.azimuth.sin(), 0.0);
		let mut move_vector = Vec3f::new(0.0, 0.0, 0.0);

		if keyboard_state.is_scancode_pressed(Scancode::W)
		{
			move_vector += forward_vector;
		}
		if keyboard_state.is_scancode_pressed(Scancode::S)
		{
			move_vector -= forward_vector;
		}
		if keyboard_state.is_scancode_pressed(Scancode::D)
		{
			move_vector += left_vector;
		}
		if keyboard_state.is_scancode_pressed(Scancode::A)
		{
			move_vector -= left_vector;
		}

		let move_vector_length = move_vector.magnitude();
		if move_vector_length > 0.0
		{
			self.pos += move_vector * (time_delta_s * SPEED / move_vector_length);
		}

		if keyboard_state.is_scancode_pressed(Scancode::Space)
		{
			self.pos.z += time_delta_s * JUMP_SPEED;
		}
		if keyboard_state.is_scancode_pressed(Scancode::C)
		{
			self.pos.z -= time_delta_s * JUMP_SPEED;
		}

		if keyboard_state.is_scancode_pressed(Scancode::Left)
		{
			self.azimuth += ANGLE_SPEED * time_delta_s;
		}
		if keyboard_state.is_scancode_pressed(Scancode::Right)
		{
			self.azimuth -= ANGLE_SPEED * time_delta_s;
		}

		if keyboard_state.is_scancode_pressed(Scancode::Up)
		{
			self.elevation += ANGLE_SPEED * time_delta_s;
		}
		if keyboard_state.is_scancode_pressed(Scancode::Down)
		{
			self.elevation -= ANGLE_SPEED * time_delta_s;
		}

		while self.azimuth > PI
		{
			self.azimuth -= two_pi;
		}
		while self.azimuth < -PI
		{
			self.azimuth += two_pi;
		}

		if self.elevation > half_pi
		{
			self.elevation = half_pi;
		}
		if self.elevation < -half_pi
		{
			self.elevation = -half_pi;
		}
	}

	pub fn build_view_matrix(&self, viewport_width: f32, viewport_height: f32) -> Mat4f
	{
		// TODO - tune this?
		let fov = Rad(std::f32::consts::PI * 0.375);
		let aspect = viewport_width / viewport_height;
		let z_near = 1.0;
		let z_far = 64.0 * 256.0;

		let translate = Mat4f::from_translation(-self.pos);
		let rotate_z = Mat4f::from_angle_z(-self.azimuth);
		let rotate_x = Mat4f::from_angle_x(-self.elevation);

		let mut basis_change = Mat4f::identity();
		basis_change.y.y = 0.0;
		basis_change.z.y = -1.0;
		basis_change.y.z = -1.0;
		basis_change.z.z = 0.0;

		//let perspective = cgmath::perspective(fov, aspect, z_near, z_far);
		let perspective = Mat4f::from_nonuniform_scale(1.0 / 256.0, 1.0 / 256.0, 1.0);
		let resize_to_viewport = Mat4f::from_nonuniform_scale(viewport_width * 0.5, viewport_height * 0.5, 1.0);
		let shift_to_viewport_center =
			Mat4f::from_translation(Vec3f::new(viewport_width * 0.5, viewport_height * 0.5, 0.0));

		// Perform transformations in reverse order in order to perform transformation via "matrix * vector".
		// TODO - perform calculations in "double" for better pericision?
		shift_to_viewport_center * resize_to_viewport * perspective * basis_change * rotate_x * rotate_z * translate
	}
}
