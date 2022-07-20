use super::{math_types::*, system_window};
use sdl2::keyboard::Scancode;

pub struct CameraController
{
	pos: Vec3f,
	azimuth: RadiansF,
	elevation: RadiansF,
	roll: RadiansF,
}

impl CameraController
{
	pub fn new() -> Self
	{
		CameraController {
			pos: Vec3f::new(0.0, 0.0, 0.0),
			azimuth: RadiansF::zero(),
			elevation: RadiansF::zero(),
			roll: RadiansF::zero(),
		}
	}

	pub fn get_azimuth(&self) -> RadiansF
	{
		self.azimuth
	}

	pub fn get_elevation(&self) -> RadiansF
	{
		self.elevation
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
		(self.azimuth.0, self.elevation.0, self.roll.0)
	}

	pub fn get_euler_angles(&self) -> EulerAnglesF
	{
		EulerAnglesF::new(
			self.roll,
			-self.elevation,
			self.azimuth + Rad(std::f32::consts::PI * 0.5),
		)
	}

	pub fn set_angles(&mut self, azimuth: f32, elevation: f32, roll: f32)
	{
		self.azimuth = Rad(azimuth);
		self.elevation = Rad(elevation);
		self.roll = Rad(roll);
	}

	pub fn update(&mut self, keyboard_state: &system_window::KeyboardState, time_delta_s: f32)
	{
		const SPEED: f32 = 256.0;
		const JUMP_SPEED: f32 = 0.8 * SPEED;
		const ANGLE_SPEED: RadiansF = Rad(2.0);
		const PI: RadiansF = Rad(std::f32::consts::PI);
		const MAX_ROLL: RadiansF = Rad(std::f32::consts::PI / 6.0);
		let half_pi = PI / 2.0;
		let two_pi = PI * 2.0;

		let forward_vector = Vec3f::new(-(self.azimuth.sin()), self.azimuth.cos(), 0.0);
		let left_vector = Vec3f::new(self.azimuth.cos(), self.azimuth.sin(), 0.0);
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

		if keyboard_state.contains(&Scancode::Left)
		{
			self.azimuth += ANGLE_SPEED * time_delta_s;
		}
		if keyboard_state.contains(&Scancode::Right)
		{
			self.azimuth -= ANGLE_SPEED * time_delta_s;
		}

		if keyboard_state.contains(&Scancode::Up)
		{
			self.elevation += ANGLE_SPEED * time_delta_s;
		}
		if keyboard_state.contains(&Scancode::Down)
		{
			self.elevation -= ANGLE_SPEED * time_delta_s;
		}

		if keyboard_state.contains(&Scancode::Q)
		{
			self.roll -= ANGLE_SPEED * time_delta_s;
		}
		if keyboard_state.contains(&Scancode::E)
		{
			self.roll += ANGLE_SPEED * time_delta_s;
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

		if self.roll > MAX_ROLL
		{
			self.roll = MAX_ROLL;
		}
		if self.roll < -MAX_ROLL
		{
			self.roll = -MAX_ROLL;
		}
	}
}
