use super::{
	commands_processor, commands_queue, config, console, debug_stats_printer::*, host_config::*, inline_models_index,
	performance_counter::*, postprocessor::*, renderer, resources_manager::*, test_game, text_printer,
	ticks_counter::*,
};
use common::{color::*, system_window};
use sdl2::{event::Event, keyboard::Keycode};
use std::time::Duration;

pub struct Host
{
	config_file_path: std::path::PathBuf,
	app_config: config::ConfigSharedPtr,
	config: HostConfig,
	config_is_durty: bool,

	commands_queue: commands_queue::CommandsQueuePtr<Host>,
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	window: system_window::SystemWindow,
	postprocessor: Postprocessor,
	resources_manager: ResourcesManagerSharedPtr,
	active_map: Option<ActiveMap>,
	prev_time: std::time::Instant,
	fps_counter: TicksCounter,
	frame_duration_counter: PerformanceCounter,
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
		console.lock().unwrap().add_text("Innitializing host".to_string());

		let commands_queue = commands_queue::CommandsQueue::new(vec![
			("map", Host::command_map),
			("quit", Host::command_quit),
			("resize_window", Host::command_resize_window),
		]);

		commands_processor
			.lock()
			.unwrap()
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
			console: console.clone(),
			window: system_window::SystemWindow::new(),
			postprocessor: Postprocessor::new(app_config.clone()),
			resources_manager: ResourcesManager::new(app_config, console),
			active_map: None,
			prev_time: cur_time,
			fps_counter: TicksCounter::new(),
			frame_duration_counter: PerformanceCounter::new(200),
			quit_requested: false,
		};

		// Process startup commands one by one.
		// This is needed to handle properly commands of subsystems that are constructed by previous commands.
		for command_line in &startup_commands
		{
			host.console
				.lock()
				.unwrap()
				.add_text(format!("Executing \"{}\"", command_line));
			host.commands_processor.lock().unwrap().process_command(&command_line);
			host.process_commands();
		}

		host
	}

	fn process_events(&mut self)
	{
		// Remember if ` was pressed to avoid using it as input for console.
		let mut has_backquote = false;
		for event in self.window.get_events()
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
						if self.console.lock().unwrap().is_active()
						{
							self.console.lock().unwrap().toggle();
						}
						else
						{
							self.quit_requested = true;
						}
					}
					if keycode == Some(Keycode::Backquote)
					{
						has_backquote = true;
						self.console.lock().unwrap().toggle();
					}
					if self.console.lock().unwrap().is_active()
					{
						if let Some(k) = keycode
						{
							self.console.lock().unwrap().process_key_press(k);
						}
					}
				},
				Event::TextInput { text, .. } =>
				{
					if self.console.lock().unwrap().is_active() && !has_backquote
					{
						self.console.lock().unwrap().process_text_input(&text);
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
		queue_ptr_copy.lock().unwrap().process_commands(self);
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

	// Returns true if need to continue.
	pub fn process_frame(&mut self) -> bool
	{
		self.process_events();
		self.process_commands();
		self.synchronize_config();

		if self.config.fullscreen_mode == 0.0
		{
			self.window.set_windowed();
		}
		else if self.config.fullscreen_mode == 1.0
		{
			self.window.set_fullscreen_desktop();
		}
		else if self.config.fullscreen_mode == 2.0
		{
			self.window.set_fullscreen();
		}
		else
		{
			self.config.fullscreen_mode = 0.0;
		}

		let cur_time = std::time::Instant::now();
		let time_delta_s = (cur_time - self.prev_time).as_secs_f32();
		self.prev_time = cur_time;

		self.frame_duration_counter.add_value(time_delta_s);

		let parallel_swap_buffers = self.config.parallel_swap_buffers;

		let window = &mut self.window;
		let keyboard_state = window.get_keyboard_state();

		let postprocessor = &mut self.postprocessor;
		let active_map = &mut self.active_map;
		let console = self.console.clone();
		let fps_counter = &mut self.fps_counter;
		let max_fps = self.config.max_fps;
		let frame_duration_counter = &self.frame_duration_counter;

		let mut frame_info = None;

		// First, only prepare frame without accessing surface pixels.
		let surface_info_initial = window.get_window_surface_info();
		let mut prepare_frame_func = || {
			if let Some(active_map) = active_map
			{
				// Process game logic.
				if !console.lock().unwrap().is_active()
				{
					active_map.game.process_input(&keyboard_state, time_delta_s);
				}
				active_map.game.update(time_delta_s);

				// Get frame info from game code.
				frame_info = Some(active_map.game.get_frame_info(&surface_info_initial));
				let frame_info_ref = frame_info.as_ref().unwrap();

				// Perform rendering frame preparation.
				if postprocessor.use_hdr_rendering()
				{
					let hdr_buffer_size = [surface_info_initial.width, surface_info_initial.height];
					let hdr_buffer = postprocessor.get_hdr_buffer(hdr_buffer_size);

					let hdr_surface_info = system_window::SurfaceInfo {
						width: hdr_buffer_size[0],
						height: hdr_buffer_size[1],
						pitch: hdr_buffer_size[0],
					};

					active_map.renderer.prepare_frame::<Color64>(
						&hdr_surface_info,
						frame_info_ref,
						&active_map.inline_models_index,
					);

					active_map.renderer.draw_frame(
						hdr_buffer,
						&hdr_surface_info,
						frame_info_ref,
						&active_map.inline_models_index,
						&mut active_map.debug_stats_printer,
					);
				}
				else
				{
					active_map.renderer.prepare_frame::<Color32>(
						&surface_info_initial,
						frame_info_ref,
						&active_map.inline_models_index,
					);
				}
			}
		};

		let limit_fps_func = || {
			if max_fps > 0.0
			{
				let min_frame_time = 1.0 / max_fps;
				if time_delta_s < min_frame_time
				{
					std::thread::sleep(Duration::from_secs_f32(
						((min_frame_time - time_delta_s) * 1000.0).floor() / 1000.0,
					));
				}
			}
		};

		if parallel_swap_buffers
		{
			rayon::in_place_scope(|s| {
				// Start CURRENT frame preparation.
				s.spawn(|_s| {
					prepare_frame_func();
				});
				// Swap buffers for PREVIOUS frame.
				window.swap_buffers();
				limit_fps_func();
			});
		}
		else
		{
			// Prepare CURRENT frame.
			prepare_frame_func();
		}

		// Than update surface pixels.
		window.update_window_surface(|pixels, surface_info| {
			if *surface_info != surface_info_initial
			{
				// Skip this frame because surface size was changed.
				return;
			}

			if let Some(active_map) = active_map
			{
				if postprocessor.use_hdr_rendering()
				{
					postprocessor.perform_postprocessing(
						pixels,
						surface_info,
						time_delta_s,
						&mut active_map.debug_stats_printer,
					);
				}
				else
				{
					let frame_info_ref = frame_info.as_ref().unwrap();
					active_map.renderer.draw_frame(
						pixels,
						surface_info,
						frame_info_ref,
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

			console.lock().unwrap().draw(pixels, surface_info);

			text_printer::print(
				pixels,
				surface_info,
				&format!(
					"fps {:04.2}\n{:04.2}ms",
					fps_counter.get_frequency(),
					frame_duration_counter.get_average_value() * 1000.0
				),
				(surface_info.width - 96) as i32,
				1,
				Color32::from_rgb(255, 255, 255),
			);
		});

		if !parallel_swap_buffers
		{
			// Finally, swap buffers for CURRENT frame.
			window.swap_buffers();
			limit_fps_func();
		}

		fps_counter.tick();

		!self.quit_requested
	}

	fn command_map(&mut self, args: commands_queue::CommandArgs)
	{
		if args.is_empty()
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Expected map file name".to_string());
			return;
		}
		self.active_map = None;

		let map_opt = self.resources_manager.lock().unwrap().get_map(&args[0]);
		if let Some(map) = map_opt
		{
			self.active_map = Some(ActiveMap {
				game: test_game::Game::new(
					self.commands_processor.clone(),
					self.console.clone(),
					self.resources_manager.clone(),
				),
				renderer: renderer::Renderer::new(self.resources_manager.clone(), self.app_config.clone(), map.clone()),
				inline_models_index: inline_models_index::InlineModelsIndex::new(map),
				debug_stats_printer: DebugStatsPrinter::new(self.config.show_debug_stats),
			});

			// Clear unused resources from previous map.
			self.resources_manager.lock().unwrap().clear_cache();
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
			self.console.lock().unwrap().add_text("Expected two args".to_string());
			return;
		}

		if let (Ok(width), Ok(height)) = (args[0].parse::<u32>(), args[1].parse::<u32>())
		{
			self.window.resize(width, height);
		}
		else
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Failed to parse args".to_string());
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
