use super::{config, host_config::*, renderer};
use common::{bsp_map_save_load, camera_controller, color::*, system_window, ticks_counter::*};
use sdl2::{event::Event, keyboard::Keycode};
use std::{cell::RefCell, rc::Rc, time::Duration};

pub struct Host
{
	config: HostConfig,
	window: Rc<RefCell<system_window::SystemWindow>>,
	camera: camera_controller::CameraController,
	renderer: renderer::Renderer,
	prev_time: std::time::Instant,
	fps_counter: TicksCounter,
}

impl Host
{
	pub fn new(map_path: &std::path::Path) -> Self
	{
		let config_file_path = "config.json";
		let config_json = config::load(std::path::Path::new(config_file_path)).unwrap_or_default();

		let map = bsp_map_save_load::load_map(map_path).unwrap().unwrap();

		Host {
			config: HostConfig::from_app_config(&config_json),
			window: Rc::new(RefCell::new(system_window::SystemWindow::new())),
			camera: camera_controller::CameraController::new(),
			renderer: renderer::Renderer::new(&config_json, map),
			prev_time: std::time::Instant::now(),
			fps_counter: TicksCounter::new(),
		}
	}

	// Returns true if need to continue.
	pub fn process_frame(&mut self) -> bool
	{
		for event in self.window.borrow_mut().get_events()
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

		self.camera
			.update(&self.window.borrow_mut().get_keyboard_state(), time_delta_s);

		let witndow_ptr_clone = self.window.clone();

		witndow_ptr_clone.borrow_mut().end_frame(|pixels, surface_info| {
			self.draw_frame(pixels, surface_info);
		});

		if self.config.max_fps > 0.0
		{
			let frame_end_time = std::time::Instant::now();
			let frame_time_s = (frame_end_time - self.prev_time).as_secs_f32();
			let min_frame_time = 1.0 / self.config.max_fps;
			if frame_time_s < min_frame_time
			{
				std::thread::sleep(Duration::from_secs_f32(min_frame_time - frame_time_s));
			}
		}

		self.fps_counter.tick();

		true
	}

	fn draw_frame(&mut self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		let view_matrix = &self
			.camera
			.build_view_matrix(surface_info.width as f32, surface_info.height as f32);

		self.renderer.draw_frame(pixels, surface_info, view_matrix);

		common::text_printer::print(
			pixels,
			surface_info,
			&format!("fps {:04.2}", self.fps_counter.get_frequency()),
			(surface_info.width - 96) as i32,
			1,
			Color32::from_rgb(255, 255, 255),
		);
		common::text_printer::print(
			pixels,
			surface_info,
			&format!("{:04.2} ms", 1000.0 / self.fps_counter.get_frequency()),
			(surface_info.width - 96) as i32,
			19,
			Color32::from_rgb(255, 255, 255),
		);
	}
}
