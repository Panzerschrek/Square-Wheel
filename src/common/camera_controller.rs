use super::math_types::*;
use cgmath::InnerSpace;
use sdl2::keyboard::Scancode;
use std::f32::consts::PI;

pub struct CameraController
{
	pos: Vec3f,
	azimuth: f32,
	elevation: f32,
}

impl CameraController
{
	pub fn new() -> Self
	{
		CameraController {
			pos: Vec3f::new(0.0, 0.0, 0.0),
			azimuth: 0.0,
			elevation: 0.0,
		}
	}

	pub fn update(&mut self, keyboard_state: &sdl2::keyboard::KeyboardState, time_delta_s: f32)
	{
		const SPEED: f32 = 1.0;
		const JUMP_SPEED: f32 = 0.8 * SPEED;
		const ANGLE_SPEED: f32 = 1.0;

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
			self.azimuth += time_delta_s * ANGLE_SPEED;
		}
		if keyboard_state.is_scancode_pressed(Scancode::Right)
		{
			self.azimuth -= time_delta_s * ANGLE_SPEED;
		}

		if keyboard_state.is_scancode_pressed(Scancode::Up)
		{
			self.elevation += time_delta_s * ANGLE_SPEED;
		}
		if keyboard_state.is_scancode_pressed(Scancode::Down)
		{
			self.elevation -= time_delta_s * ANGLE_SPEED;
		}

		while self.azimuth > PI
		{
			self.azimuth -= 2.0 * PI;
		}
		while self.azimuth < -PI
		{
			self.azimuth += 2.0 * PI;
		}

		if self.elevation > PI * 0.5
		{
			self.elevation = PI * 0.5;
		}
		if self.elevation < -PI * 0.5
		{
			self.elevation = -PI * 0.5;
		}
	}
}
