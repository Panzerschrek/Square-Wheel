use super::{camera_rotation_controller::*, math_types::*, system_window};
use sdl2::keyboard::Scancode;

pub struct CameraController
{
	pos: Vec3f,
	rotation_controller: CameraRotationController,
}

impl CameraController
{
	pub fn new() -> Self
	{
		CameraController {
			pos: Vec3f::new(0.0, 0.0, 0.0),
			rotation_controller: CameraRotationController::new(),
		}
	}

	pub fn get_pos(&self) -> Vec3f
	{
		self.pos
	}

	pub fn set_pos(&mut self, pos: &Vec3f)
	{
		self.pos = *pos;
	}

	pub fn get_angles(&self) -> (f32, f32, f32)
	{
		self.rotation_controller.get_angles()
	}

	pub fn get_rotation(&self) -> QuaternionF
	{
		self.rotation_controller.get_rotation()
	}

	pub fn set_angles(&mut self, azimuth: f32, elevation: f32, roll: f32)
	{
		self.rotation_controller.set_angles(azimuth, elevation, roll)
	}

	pub fn update(&mut self, keyboard_state: &system_window::KeyboardState, time_delta_s: f32)
	{
		self.rotation_controller.update(keyboard_state, time_delta_s);

		const SPEED: f32 = 256.0;
		const JUMP_SPEED: f32 = 0.8 * SPEED;

		let azimuth = self.rotation_controller.get_azimuth();
		let forward_vector = Vec3f::new(-(azimuth.sin()), azimuth.cos(), 0.0);
		let left_vector = Vec3f::new(azimuth.cos(), azimuth.sin(), 0.0);
		let mut move_vector = Vec3f::new(0.0, 0.0, 0.0);

		if keyboard_state.contains(&Scancode::W)
		{
			move_vector += forward_vector;
		}
		if keyboard_state.contains(&Scancode::S)
		{
			move_vector -= forward_vector;
		}
		if keyboard_state.contains(&Scancode::D)
		{
			move_vector += left_vector;
		}
		if keyboard_state.contains(&Scancode::A)
		{
			move_vector -= left_vector;
		}

		let move_vector_length = move_vector.magnitude();
		if move_vector_length > 0.0
		{
			self.pos += move_vector * (time_delta_s * SPEED / move_vector_length);
		}

		if keyboard_state.contains(&Scancode::Space)
		{
			self.pos.z += time_delta_s * JUMP_SPEED;
		}
		if keyboard_state.contains(&Scancode::C)
		{
			self.pos.z -= time_delta_s * JUMP_SPEED;
		}
	}
}
