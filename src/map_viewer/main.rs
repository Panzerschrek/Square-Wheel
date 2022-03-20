extern crate sdl2;

use common::system_window;
use sdl2::{event::Event, keyboard::Keycode};
use std::time::Duration;

pub fn main()
{
	let mut window = system_window::SystemWindow::new();
	let mut camera_controller = common::camera_controller::CameraController::new();

	let frame_duration_s = 1.0 / 30.0;
	'running: loop
	{
		for event in window.get_events()
		{
			match event
			{
				Event::Quit { .. } |
				Event::KeyDown {
					keycode: Some(Keycode::Escape),
					..
				} => break 'running,
				_ =>
				{},
			}
		}

		camera_controller.update(&window.get_keyboard_state(), frame_duration_s);

		window
			.end_frame(|pixels, surface_info| draw_frame(pixels, surface_info, &camera_controller.build_view_matrix()));

		std::thread::sleep(Duration::from_secs_f32(frame_duration_s));
	}
}

fn draw_frame(pixels: &mut [u8], surface_info: &system_window::SurfaceInfo, view_matrix: &common::math_types::Mat4f)
{
	draw_background(pixels, surface_info);
	draw_lines(pixels, surface_info, view_matrix);
}

fn draw_background(pixels: &mut [u8], surface_info: &system_window::SurfaceInfo)
{
	for x in 0 .. surface_info.width
	{
		for y in 0 .. surface_info.height
		{
			let index = 4 * x + y * surface_info.pitch;
			pixels[index] = 0 * 255;
			pixels[index + 1] = ((x + y * 2) & 255) as u8;
			pixels[index + 2] = 0 * 255;
			pixels[index + 3] = 0 * 128;
		}
	}
}

fn draw_lines(pixels: &mut [u8], surface_info: &system_window::SurfaceInfo, view_matrix: &common::math_types::Mat4f)
{
	use common::{debug_renderer::*, fixed_math::*, math_types::*};

	let lines = [
		(Vec3f::new(1.0, 1.0, 1.0), Vec3f::new(2.0, 1.5, 1.0)),
		(Vec3f::new(1.0, 1.0, 1.0), Vec3f::new(1.5, 2.0, 1.0)),
		(Vec3f::new(1.0, 1.0, 1.0), Vec3f::new(1.0, 1.0, 3.0)),
	];

	let mut renderer = DebugRenderer::new(pixels, surface_info);

	let half_width = (surface_info.width as f32) * 0.5;
	let half_height = (surface_info.height as f32) * 0.5;
	let fixed_scale = FIXED16_ONE as f32;
	for line in lines
	{
		let v0 = view_matrix * line.0.extend(1.0);
		let v1 = view_matrix * line.1.extend(1.0);

		// TODO - perform proper clipping
		if v0.w <= 0.1 || v1.w <= 0.1
		{
			continue;
		}
		let v0 = v0.truncate() / v0.w;
		let v1 = v1.truncate() / v1.w;

		if v0.x < -2.0 ||
			v0.x > 2.0 || v0.y < -2.0 ||
			v0.y > 2.0 || v1.x < -2.0 ||
			v1.x > 2.0 || v1.y < -2.0 ||
			v1.y > 2.0
		{
			continue;
		}

		// TODO - perform final transformations via same view matrix
		renderer.draw_line(
			PointProjected {
				x: ((v0.x + 1.0) * half_width * fixed_scale) as Fixed16,
				y: ((v0.y + 1.0) * half_height * fixed_scale) as Fixed16,
			},
			PointProjected {
				x: ((v1.x + 1.0) * half_width * fixed_scale) as Fixed16,
				y: ((v1.y + 1.0) * half_height * fixed_scale) as Fixed16,
			},
		);
	}
}
