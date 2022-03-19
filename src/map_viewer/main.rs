extern crate sdl2;

use common::system_window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

pub fn main()
{
	let mut window = system_window::SystemWindow::new();
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

		window.end_frame(draw_background);

		::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
	}
}

fn draw_background(pixels: &mut [u8], surface_info: &system_window::SurfaceInfo)
{
	for x in 0 .. surface_info.width
	{
		for y in 0 .. surface_info.height
		{
			let index = 4 * x + y * surface_info.pitch;
			pixels[index] = 255;
			pixels[index + 1] = ((x + y * 2) & 255) as u8;
			pixels[index + 2] = 255;
			pixels[index + 3] = 128;
		}
	}
}
