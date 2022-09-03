use super::{math_types::*, system_window};
use sdl2::keyboard::Scancode;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CameraRotationController
{
	azimuth: RadiansF,
	elevation: RadiansF,
	roll: RadiansF,
}

impl CameraRotationController
{
	pub fn new() -> Self
	{
		Self {
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

	pub fn get_angles(&self) -> (f32, f32, f32)
	{
		(self.azimuth.0, self.elevation.0, self.roll.0)
	}

	pub fn get_rotation(&self) -> QuaternionF
	{
		let rotate_x = QuaternionF::from_angle_x(self.roll);
		let rotate_y = QuaternionF::from_angle_y(-self.elevation);
		let rotate_z = QuaternionF::from_angle_z(self.azimuth + Rad(std::f32::consts::PI * 0.5));

		rotate_z * rotate_y * rotate_x
	}

	pub fn set_angles(&mut self, azimuth: f32, elevation: f32, roll: f32)
	{
		self.azimuth = Rad(azimuth);
		self.elevation = Rad(elevation);
		self.roll = Rad(roll);
	}

	pub fn update(
		&mut self,
		keyboard_state: &system_window::KeyboardState,
		events: &[sdl2::event::Event],
		time_delta_s: f32,
	)
	{
		// TODO - make these values configurable.
		const ANGLE_SPEED: RadiansF = Rad(2.0);
		const MOUSE_SENSITIVITY: RadiansF = Rad(0.005);

		const PI: RadiansF = Rad(std::f32::consts::PI);
		const MAX_ROLL: RadiansF = Rad(std::f32::consts::PI / 6.0);
		let half_pi = PI / 2.0;
		let two_pi = PI * 2.0;

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

		for event in events
		{
			match event
			{
				sdl2::event::Event::MouseMotion { xrel, yrel, .. } =>
				{
					self.azimuth -= MOUSE_SENSITIVITY * (*xrel as f32);
					self.elevation -= MOUSE_SENSITIVITY * (*yrel as f32);
				},
				_ =>
				{},
			}
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
