use super::renderer;
use common::{bsp_map_compact, bsp_map_save_load, camera_controller, color::*, system_window};
use sdl2::{event::Event, keyboard::Keycode};
use std::time::Duration;

pub struct Host
{
	window: system_window::SystemWindow,
	map: bsp_map_compact::BSPMap,
	camera: camera_controller::CameraController,
	prev_time: std::time::Instant,
}

impl Host
{
	pub fn new(map_path: &std::path::Path) -> Self
	{
		Host {
			window: system_window::SystemWindow::new(),
			map: bsp_map_save_load::load_map(map_path).unwrap().unwrap(),
			camera: camera_controller::CameraController::new(),
			prev_time: std::time::Instant::now(),
		}
	}

	// Returns true if need to continue.
	pub fn process_frame(&mut self) -> bool
	{
		for event in self.window.get_events()
		{
			match event
			{
				Event::Quit { .. } |
				Event::KeyDown {
					keycode: Some(Keycode::Escape),
					..
				} => return false,
				_ =>
				{},
			}
		}

		let cur_time = std::time::Instant::now();
		let time_delta_s = (cur_time - self.prev_time).as_secs_f32();
		self.prev_time = cur_time;

		self.camera.update(&self.window.get_keyboard_state(), time_delta_s);

		self.window.end_frame(|pixels, surface_info| {
			renderer::draw_frame(
				pixels,
				surface_info,
				&self
					.camera
					.build_view_matrix(surface_info.width as f32, surface_info.height as f32),
				&self.map,
			);
			common::text_printer::print_scaled(
				pixels,
				surface_info,
				"Square Wheel",
				7,
				3,
				Color32::from_rgb(255, 255, 255),
				2,
			);
		});

		let frame_end_time = std::time::Instant::now();
		let frame_time_s = (frame_end_time - self.prev_time).as_secs_f32();
		let min_frame_time = 0.01;
		if frame_time_s < min_frame_time
		{
			std::thread::sleep(Duration::from_secs_f32(min_frame_time - frame_time_s));
		}

		true
	}
}
