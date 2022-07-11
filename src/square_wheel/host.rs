use super::{
	commands_processor, commands_queue, config, console, debug_stats_printer::*, host_config::*, inline_models_index,
	postprocessor::*, renderer, test_game, text_printer, ticks_counter::*,
};
use common::{bsp_map_save_load, color::*, system_window};
use sdl2::{event::Event, keyboard::Keycode};
use std::{cell::RefCell, rc::Rc, time::Duration};

pub struct Host
{
	config_file_path: std::path::PathBuf,
	app_config: config::ConfigSharedPtr,
	config: HostConfig,
	config_is_durty: bool,

	commands_queue: commands_queue::CommandsQueuePtr<Host>,
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	window: Rc<RefCell<system_window::SystemWindow>>,
	postprocessor: Postprocessor,
	active_map: Option<ActiveMap>,
	prev_time: std::time::Instant,
	fps_counter: TicksCounter,
	quit_requested: bool,
}

struct ActiveMap
{
	game: test_game::Game,
	renderer: renderer::Renderer,
	inline_models_index: inline_models_index::InlineModelsIndex,
	debug_stats_printer: DebugStatsPrinter,
}

impl Host
{
	pub fn new(config_file_path: std::path::PathBuf, startup_commands: Vec<String>) -> Self
	{
		println!("Loading config from file \"{:?}\"", config_file_path);
		let config_json = if let Some(json) = config::load(&config_file_path)
		{
			json
		}
		else
		{
			println!("Failed to load config file");
			serde_json::Value::Object(serde_json::Map::new())
		};
		let app_config = config::make_shared(config_json);
		let mut host_config = HostConfig::from_app_config(&app_config);

		// Initialize global thread pool.
		{
			let mut num_threads = host_config.num_threads as usize;
			if num_threads <= 0
			{
				num_threads = num_cpus::get();
				println!("Use number of threads equal to number of CPU cores");
			}
			let num_threads_max = 64;
			if num_threads > num_threads_max
			{
				num_threads = num_threads_max;
				host_config.num_threads = num_threads_max as f32;
			}
			println!("Initialize thread pool with {} threads", num_threads);
			rayon::ThreadPoolBuilder::new()
				.num_threads(num_threads)
				.build_global()
				.unwrap();
		}

		let commands_processor = commands_processor::CommandsProcessor::new(app_config.clone());
		let console = console::Console::new(commands_processor.clone());
		console.borrow_mut().add_text("Innitializing host".to_string());

		let commands_queue = commands_queue::CommandsQueue::new(vec![
			("map", Host::command_map),
			("quit", Host::command_quit),
			("resize_window", Host::command_resize_window),
		]);

		commands_processor
			.borrow_mut()
			.register_command_queue(commands_queue.clone() as commands_queue::CommandsQueueDynPtr);

		let cur_time = std::time::Instant::now();

		host_config.update_app_config(&app_config); // Update JSON with struct fields.

		let mut host = Host {
			config_file_path,
			app_config: app_config.clone(),
			config: host_config,
			config_is_durty: false,
			commands_queue,
			commands_processor,
			console,
			window: Rc::new(RefCell::new(system_window::SystemWindow::new())),
			postprocessor: Postprocessor::new(app_config),
			active_map: None,
			prev_time: cur_time,
			fps_counter: TicksCounter::new(),
			quit_requested: false,
		};

		// Process startup commands one by one.
		// This is needed to handle properly commands of subsystems that are constructed by previous commands.
		for command_line in &startup_commands
		{
			host.console
				.borrow_mut()
				.add_text(format!("Executing \"{}\"", command_line));
			host.commands_processor.borrow_mut().process_command(&command_line);
			host.process_commands();
		}

		host
	}

	// Returns true if need to continue.
	pub fn process_frame(&mut self) -> bool
	{
		self.process_events();
		self.process_commands();
		self.synchronize_config();

		if self.config.fullscreen_mode == 0.0
		{
			self.window.borrow_mut().set_windowed();
		}
		else if self.config.fullscreen_mode == 1.0
		{
			self.window.borrow_mut().set_fullscreen_desktop();
		}
		else if self.config.fullscreen_mode == 2.0
		{
			self.window.borrow_mut().set_fullscreen();
		}
		else
		{
			self.config.fullscreen_mode = 0.0;
		}

		let cur_time = std::time::Instant::now();
		let time_delta_s = (cur_time - self.prev_time).as_secs_f32();
		self.prev_time = cur_time;

		if let Some(active_map) = &mut self.active_map
		{
			if !self.console.borrow().is_active()
			{
				active_map
					.game
					.process_input(&self.window.borrow_mut().get_keyboard_state(), time_delta_s);
			}
			active_map.game.update(time_delta_s);
		}

		self.draw_frame(time_delta_s);

		if self.config.max_fps > 0.0
		{
			let frame_end_time = std::time::Instant::now();
			let frame_time_s = (frame_end_time - self.prev_time).as_secs_f32();
			let min_frame_time = 1.0 / self.config.max_fps;
			if frame_time_s < min_frame_time
			{
				std::thread::sleep(Duration::from_secs_f32(
					((min_frame_time - frame_time_s) * 1000.0).floor() / 1000.0,
				));
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
						if self.console.borrow().is_active()
						{
							self.console.borrow_mut().toggle();
						}
						else
						{
							self.quit_requested = true;
						}
					}
					if keycode == Some(Keycode::Backquote)
					{
						has_backquote = true;
						self.console.borrow_mut().toggle();
					}
					if self.console.borrow().is_active()
					{
						if let Some(k) = keycode
						{
							self.console.borrow_mut().process_key_press(k);
						}
					}
				},
				Event::TextInput { text, .. } =>
				{
					if self.console.borrow().is_active() && !has_backquote
					{
						self.console.borrow_mut().process_text_input(&text);
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

	fn synchronize_config(&mut self)
	{
		self.postprocessor.synchronize_config();

		if self.config_is_durty
		{
			self.config_is_durty = false;
			self.config.update_app_config(&self.app_config);
		}
		else
		{
			self.config = HostConfig::from_app_config(&self.app_config);
		}

		// Make sure that config values are reasonable.
		if self.config.max_fps < 0.0
		{
			self.config.max_fps = 0.0;
			self.config_is_durty = true;
		}
	}

	fn draw_frame(&mut self, time_delta_s: f32)
	{
		let witndow_ptr_clone = self.window.clone();

		// First, only prepare frame without accessing surface pixels.
		let surface_info_initial = witndow_ptr_clone.borrow_mut().get_window_surface_info();
		if let Some(active_map) = &mut self.active_map
		{
			let camera_matrices = active_map.game.get_camera_matrices(&surface_info_initial);

			if self.postprocessor.use_hdr_rendering()
			{
				let hdr_buffer_size = [surface_info_initial.width, surface_info_initial.height];
				let hdr_buffer = self.postprocessor.get_hdr_buffer(hdr_buffer_size);

				active_map.renderer.prepare_frame::<Color64>(
					&system_window::SurfaceInfo {
						width: hdr_buffer_size[0],
						height: hdr_buffer_size[1],
						pitch: hdr_buffer_size[0],
					},
					&camera_matrices,
					&active_map.inline_models_index,
					active_map.game.get_test_lights(),
					active_map.game.get_game_time_s(),
				);

				active_map.renderer.draw_frame(
					hdr_buffer,
					&system_window::SurfaceInfo {
						width: hdr_buffer_size[0],
						height: hdr_buffer_size[1],
						pitch: hdr_buffer_size[0],
					},
					&camera_matrices,
					&active_map.inline_models_index,
					&mut active_map.debug_stats_printer,
				);
			}
			else
			{
				active_map.renderer.prepare_frame::<Color32>(
					&surface_info_initial,
					&camera_matrices,
					&active_map.inline_models_index,
					active_map.game.get_test_lights(),
					active_map.game.get_game_time_s(),
				);
			}
		}

		// Than update surface pixels.
		witndow_ptr_clone
			.borrow_mut()
			.update_window_surface(|pixels, surface_info| {
				if *surface_info != surface_info_initial
				{
					// Skip this frame because surface size was changed.
					return;
				}

				if let Some(active_map) = &mut self.active_map
				{
					if self.postprocessor.use_hdr_rendering()
					{
						self.postprocessor.perform_postprocessing(
							pixels,
							surface_info,
							time_delta_s,
							&mut active_map.debug_stats_printer,
						);
					}
					else
					{
						let camera_matrices = active_map.game.get_camera_matrices(&surface_info_initial);
						active_map.renderer.draw_frame(
							pixels,
							surface_info,
							&camera_matrices,
							&active_map.inline_models_index,
							&mut active_map.debug_stats_printer,
						);
					}

					active_map.debug_stats_printer.flush(pixels, surface_info);
				}
				else
				{
					// Just clear background. TODO - maybe draw some background pattern or image?
					for pixel in pixels.iter_mut()
					{
						*pixel = Color32::black();
					}
				}

				self.console.borrow().draw(pixels, surface_info);

				text_printer::print(
					pixels,
					surface_info,
					&format!("fps {:04.2}", self.fps_counter.get_frequency()),
					(surface_info.width - 96) as i32,
					1,
					Color32::from_rgb(255, 255, 255),
				);
			});

		// Finally, swap buffers.
		witndow_ptr_clone.borrow_mut().swap_buffers();
	}

	fn command_map(&mut self, args: commands_queue::CommandArgs)
	{
		if args.is_empty()
		{
			self.console.borrow_mut().add_text("Expected map file name".to_string());
			return;
		}
		self.active_map = None;

		let mut map_path = std::path::PathBuf::from(self.config.maps_path.clone());
		map_path.push(args[0].clone());

		map_path = bsp_map_save_load::normalize_bsp_map_file_path(map_path);
		match bsp_map_save_load::load_map(&map_path)
		{
			Ok(Some(map)) =>
			{
				let map_rc = std::sync::Arc::new(map);
				self.active_map = Some(ActiveMap {
					game: test_game::Game::new(self.commands_processor.clone(), self.console.clone()),
					renderer: renderer::Renderer::new(self.app_config.clone(), map_rc.clone()),
					inline_models_index: inline_models_index::InlineModelsIndex::new(map_rc),
					debug_stats_printer: DebugStatsPrinter::new(self.config.show_debug_stats),
				});
			},
			Ok(None) =>
			{
				self.console
					.borrow_mut()
					.add_text(format!("Failed to load map {:?}", map_path));
			},
			Err(e) =>
			{
				self.console
					.borrow_mut()
					.add_text(format!("Failed to load map {:?}: {}", map_path, e));
			},
		}
	}

	fn command_quit(&mut self, _args: commands_queue::CommandArgs)
	{
		self.quit_requested = true;
	}

	fn command_resize_window(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 2
		{
			self.console.borrow_mut().add_text("Expected two args".to_string());
			return;
		}

		if let (Ok(width), Ok(height)) = (args[0].parse::<u32>(), args[1].parse::<u32>())
		{
			self.window.borrow_mut().resize(width, height);
		}
		else
		{
			self.console.borrow_mut().add_text("Failed to parse args".to_string());
		}
	}
}

impl Drop for Host
{
	fn drop(&mut self)
	{
		config::save(&self.app_config.lock().unwrap(), &self.config_file_path);
	}
}
