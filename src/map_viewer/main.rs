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

		window.end_frame(draw_background);

		std::thread::sleep(Duration::from_secs_f32(frame_duration_s));
	}
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

	use common::{debug_renderer::*, fixed_math::*};

	let mut renderer = DebugRenderer::new(pixels, surface_info);
	renderer.draw_line(
		PointProjected {
			x: int_to_fixed16(145),
			y: int_to_fixed16(77),
		},
		PointProjected {
			x: int_to_fixed16(77),
			y: int_to_fixed16(95),
		},
	);
	renderer.draw_line(
		PointProjected {
			x: int_to_fixed16(3),
			y: int_to_fixed16(5),
		},
		PointProjected {
			x: int_to_fixed16(17),
			y: int_to_fixed16(210),
		},
	);
}
