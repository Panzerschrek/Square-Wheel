use super::{commands_processor, commands_queue, config, console, host_config::*, inline_models_index, renderer};
use common::{bsp_map_save_load, camera_controller, color::*, math_types::*, system_window, ticks_counter::*};
use sdl2::{event::Event, keyboard::Keycode};
use std::{cell::RefCell, rc::Rc, time::Duration};

pub struct Host
{
	commands_queue: commands_queue::CommandsQueuePtr<Host>,
	console: console::Console,
	config_json: serde_json::Value,
	config: HostConfig,
	window: Rc<RefCell<system_window::SystemWindow>>,
	camera: camera_controller::CameraController,
	active_map: Option<ActiveMap>,
	prev_time: std::time::Instant,
	fps_counter: TicksCounter,
	quit_requested: bool,
}

struct ActiveMap
{
	renderer: renderer::Renderer,
	inline_models_index: inline_models_index::InlineModelsIndex,
}

impl Host
{
	pub fn new(startup_commands: Vec<String>) -> Self
	{
		let commands_processor = commands_processor::CommandsProcessor::new();
		let mut console = console::Console::new(commands_processor.clone());
		console.add_text("Innitializing host".to_string());

		let commands_queue = commands_queue::CommandsQueue::new(vec![
			("get_pos", Host::command_get_pos),
			("set_pos", Host::command_set_pos),
			("get_angles", Host::command_get_angles),
			("set_angles", Host::command_set_angles),
			("map", Host::command_map),
			("quit", Host::command_quit),
		]);

		commands_processor
			.borrow_mut()
			.register_command_queue(commands_queue.clone() as commands_queue::CommandsQueueDynPtr);

		// Process startup commands.
		// Atually such commands will be processed later (commands will be added to queue).
		for command_line in &startup_commands
		{
			console.add_text(format!("Executing \"{}\"", command_line));
			commands_processor.borrow_mut().process_command(&command_line);
		}

		let config_file_path = "config.json";
		console.add_text(format!("Loading config from file \"{}\"", config_file_path));
		let config_json = if let Some(json) = config::load(std::path::Path::new(config_file_path))
		{
			json
		}
		else
		{
			console.add_text("Failed to load config file".to_string());
			serde_json::Value::Object(serde_json::Map::new())
		};
		Host {
			commands_queue,
			console,
			config: HostConfig::from_app_config(&config_json),
			window: Rc::new(RefCell::new(system_window::SystemWindow::new())),
			camera: camera_controller::CameraController::new(),
			active_map: None,
			config_json,
			prev_time: std::time::Instant::now(),
			fps_counter: TicksCounter::new(),
			quit_requested: false,
		}
	}

	// Returns true if need to continue.
	pub fn process_frame(&mut self) -> bool
	{
		self.process_events();
		self.process_commands();

		let cur_time = std::time::Instant::now();
		let time_delta_s = (cur_time - self.prev_time).as_secs_f32();
		self.prev_time = cur_time;

		if !self.console.is_active()
		{
			self.camera
				.update(&self.window.borrow_mut().get_keyboard_state(), time_delta_s);
		}

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

		!self.quit_requested
	}

	fn process_events(&mut self)
	{
		// Remember if ` was pressed to avoid using it as input for console.
		let mut has_backquote = false;
		for event in self.window.borrow_mut().get_events()
		{
			match event
			{
				Event::Quit { .. } =>
				{
					self.quit_requested = true;
				},
				Event::KeyDown { keycode, .. } =>
				{
					if keycode == Some(Keycode::Escape)
					{
						if self.console.is_active()
						{
							self.console.toggle();
						}
						else
						{
							self.quit_requested = true;
						}
					}
					if keycode == Some(Keycode::Backquote)
					{
						has_backquote = true;
						self.console.toggle();
					}
					if self.console.is_active()
					{
						if let Some(k) = keycode
						{
							self.console.process_key_press(k);
						}
					}
				},
				Event::TextInput { text, .. } =>
				{
					if self.console.is_active() && !has_backquote
					{
						self.console.process_text_input(&text);
					}
				},
				_ =>
				{},
			}
		}
	}

	fn process_commands(&mut self)
	{
		let queue_ptr_copy = self.commands_queue.clone();
		queue_ptr_copy.borrow_mut().process_commands(self);
	}

	fn draw_frame(&mut self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		let view_matrix = &self
			.camera
			.build_view_matrix(surface_info.width as f32, surface_info.height as f32);

		if let Some(active_map) = &mut self.active_map
		{
			active_map.renderer.draw_frame(pixels, surface_info, view_matrix);
		}
		self.console.draw(pixels, surface_info);

		common::text_printer::print(
			pixels,
			surface_info,
			&format!("fps {:04.2}", self.fps_counter.get_frequency()),
			(surface_info.width - 96) as i32,
			1,
			Color32::from_rgb(255, 255, 255),
		);
	}

	fn command_set_pos(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 3
		{
			self.console.add_text("Expected 3 args".to_string());
			return;
		}

		if let (Ok(x), Ok(y), Ok(z)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			self.camera.set_pos(&Vec3f::new(x, y, z));
		}
		else
		{
			self.console.add_text("Failed to parse args".to_string());
		}
	}

	fn command_get_angles(&mut self, _args: commands_queue::CommandArgs)
	{
		let angles = self.camera.get_angles();
		self.console.add_text(format!("{} {}", angles.0, angles.1));
	}

	fn command_set_angles(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 2
		{
			self.console.add_text("Expected 2 args".to_string());
			return;
		}

		if let (Ok(azimuth), Ok(elevation)) = (args[0].parse::<f32>(), args[1].parse::<f32>())
		{
			self.camera.set_angles(azimuth, elevation);
		}
		else
		{
			self.console.add_text("Failed to parse args".to_string());
		}
	}

	fn command_get_pos(&mut self, _args: commands_queue::CommandArgs)
	{
		let pos = self.camera.get_pos();
		self.console.add_text(format!("{} {} {}", pos.x, pos.y, pos.z));
	}

	fn command_map(&mut self, args: commands_queue::CommandArgs)
	{
		if args.is_empty()
		{
			self.console.add_text("Expected map file name".to_string());
			return;
		}
		self.active_map = None;

		// TODO - reset camera?

		match bsp_map_save_load::load_map(&std::path::PathBuf::from(args[0].clone()))
		{
			Ok(Some(map)) =>
			{
				let map_rc = Rc::new(map);
				self.active_map = Some(ActiveMap {
					renderer: renderer::Renderer::new(&self.config_json, map_rc.clone()),
					inline_models_index: inline_models_index::InlineModelsIndex::new(map_rc),
				});
			},
			Ok(None) =>
			{
				self.console.add_text("Failed to load map".to_string());
			},
			Err(e) =>
			{
				self.console.add_text(format!("Failed to load map: {}", e));
			},
		}
	}

	fn command_quit(&mut self, _args: commands_queue::CommandArgs)
	{
		self.quit_requested = true;
	}
}
