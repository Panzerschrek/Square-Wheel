use super::{
	commands_processor, commands_queue, config, console, debug_stats_printer::*, frame_upscaler::*, game_interface::*,
	host_config::*, performance_counter::*, postprocessor::*, renderer, resources_manager::*, text_printer,
	ticks_counter::*,
};
use crate::common::{color::*, screenshot::*, system_window};
use sdl2::{event::Event, keyboard::Keycode};
use std::time::Duration;

pub struct Host
{
	config_file_path: std::path::PathBuf,
	app_config: config::ConfigSharedPtr,
	config: HostConfig,

	commands_queue: commands_queue::CommandsQueuePtr<Host>,
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	window: system_window::SystemWindow,
	postprocessor: Postprocessor,
	frame_upscaler: FrameUpscaler,
	resources_manager: ResourcesManagerSharedPtr,
	game_creation_function: GameCreationFunction,
	active_map: Option<ActiveMap>,
	prev_time: std::time::Instant,
	prev_frame_end_time: std::time::Instant,
	fps_counter: TicksCounter,
	frame_duration_counter: PerformanceCounter,
	quit_requested: bool,
	screenshot_requested: bool,
}

struct ActiveMap
{
	game: GameInterfacePtr,
	renderer: renderer::Renderer,
	debug_stats_printer: DebugStatsPrinter,
}

impl Host
{
	pub fn new(
		config_file_path: std::path::PathBuf,
		startup_commands: Vec<String>,
		game_creation_function: GameCreationFunction,
	) -> Self
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
				host_config.num_threads = num_threads_max as u32;
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
			("screenshot", Host::command_screenshot),
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
			commands_queue,
			commands_processor,
			console: console.clone(),
			window: system_window::SystemWindow::new(),
			postprocessor: Postprocessor::new(app_config.clone()),
			frame_upscaler: FrameUpscaler::new(),
			resources_manager: ResourcesManager::new(app_config, console),
			game_creation_function,
			active_map: None,
			prev_time: cur_time,
			prev_frame_end_time: cur_time,
			fps_counter: TicksCounter::new(),
			frame_duration_counter: PerformanceCounter::new(200),
			quit_requested: false,
			screenshot_requested: false,
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

	fn process_events(&mut self, events: &[sdl2::event::Event])
	{
		// Remember if ` was pressed to avoid using it as input for console.
		let mut has_backquote = false;
		let mut console = self.console.lock().unwrap();
		for event in events
		{
			match event
			{
				Event::Quit { .. } =>
				{
					self.quit_requested = true;
				},
				Event::KeyDown { keycode, .. } =>
				{
					if *keycode == Some(Keycode::Escape)
					{
						if console.is_active()
						{
							console.toggle();
						}
						else
						{
							self.quit_requested = true;
						}
					}
					if *keycode == Some(Keycode::F12)
					{
						self.screenshot_requested = true;
					}
					if *keycode == Some(Keycode::Backquote)
					{
						has_backquote = true;
						console.toggle();
					}
					if console.is_active()
					{
						if let Some(k) = *keycode
						{
							console.process_key_press(k);
						}
					}
				},
				Event::TextInput { text, .. } =>
				{
					if console.is_active() && !has_backquote
					{
						console.process_text_input(&text);
					}
				},
				_ =>
				{},
			}
		}
	}

	fn update_window_state(&mut self)
	{
		if self.config.fullscreen_mode == 0
		{
			self.window.set_windowed();
		}
		else if self.config.fullscreen_mode == 1
		{
			self.window.set_fullscreen_desktop();
		}
		else if self.config.fullscreen_mode == 2
		{
			self.window.set_fullscreen();
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

		self.config = HostConfig::from_app_config(&self.app_config);

		// Make sure that config values are reasonable.
		let mut config_is_dirty = false;
		if self.config.max_fps < 0.0
		{
			self.config.max_fps = 0.0;
			config_is_dirty = true;
		}
		if self.config.fullscreen_mode > 2
		{
			self.config.fullscreen_mode = 0;
			config_is_dirty = true;
		}

		if self.config.frame_scale > MAX_FRAME_SCALE as u32
		{
			self.config.frame_scale = MAX_FRAME_SCALE as u32;
			config_is_dirty = true;
		}
		if self.config.frame_scale < 1
		{
			self.config.frame_scale = 1;
			config_is_dirty = true;
		}

		if config_is_dirty
		{
			self.config.update_app_config(&self.app_config);
		}
	}

	// Returns true if need to continue.
	pub fn process_frame(&mut self) -> bool
	{
		let mut events = self.window.get_events();
		let mut keyboard_state = self.window.get_keyboard_state();
		self.process_events(&events);

		let console_is_active = self.console.lock().unwrap().is_active();
		{
			let game_grab_mouse_input = if let Some(active_map) = &self.active_map
			{
				active_map.game.grab_mouse_input()
			}
			else
			{
				false
			};
			self.window
				.set_relative_mouse(game_grab_mouse_input && !console_is_active);
		}
		if console_is_active
		{
			// Do not pass any events to game code if console is active.
			events.clear();
			keyboard_state = system_window::KeyboardState::default();
		}

		self.process_commands();
		self.synchronize_config();
		self.update_window_state();

		// Limit time delta if engine works very slow (in debug mode).
		const MAX_TIME_DELTA: f32 = 0.1;

		let cur_time = std::time::Instant::now();
		let time_delta_s = (cur_time - self.prev_time).as_secs_f32().min(MAX_TIME_DELTA);
		self.prev_time = cur_time;

		self.frame_duration_counter.add_value(time_delta_s);

		let parallel_swap_buffers = self.config.parallel_swap_buffers;

		let window = &mut self.window;
		let postprocessor = &mut self.postprocessor;
		let frame_upscaler = &mut self.frame_upscaler;
		let active_map = &mut self.active_map;
		let console = self.console.clone();
		let fps_counter = &mut self.fps_counter;
		let max_fps = self.config.max_fps;
		let frame_scale = self.config.frame_scale;
		let frame_resize_interpolate = self.config.frame_resize_interpolate;
		let frame_duration_counter = &self.frame_duration_counter;
		let prev_frame_end_time = &mut self.prev_frame_end_time;
		let screenshot_requested = &mut self.screenshot_requested;

		let mut frame_info = None;

		// First, only prepare frame without accessing surface pixels.
		let surface_info_initial = window.get_window_surface_info();
		let mut prepare_frame_func = || {
			if let Some(active_map) = active_map
			{
				// Process game logic.
				active_map.game.update(&keyboard_state, &events, time_delta_s);

				// Perform rendering frame preparation.
				if frame_scale > 1
				{
					// In case of scaled buffer perform frame preparation, rendering and postprocessing now.

					let (pixels_scaled, surface_info_scaled) =
						frame_upscaler.get_draw_buffer(&surface_info_initial, frame_scale as usize);

					// Get frame info from game code.
					frame_info = Some(active_map.game.get_frame_info(&surface_info_scaled));
					let frame_info_ref = frame_info.as_ref().unwrap();

					if postprocessor.use_hdr_rendering()
					{
						let hdr_buffer_size = [surface_info_scaled.width, surface_info_scaled.height];
						let hdr_buffer = postprocessor.get_hdr_buffer(hdr_buffer_size);

						let hdr_surface_info = system_window::SurfaceInfo {
							width: hdr_buffer_size[0],
							height: hdr_buffer_size[1],
							pitch: hdr_buffer_size[0],
						};

						active_map
							.renderer
							.prepare_frame::<Color64>(&hdr_surface_info, frame_info_ref);

						active_map.renderer.draw_frame(
							hdr_buffer,
							&hdr_surface_info,
							frame_info_ref,
							&mut active_map.debug_stats_printer,
						);

						postprocessor.perform_postprocessing(
							pixels_scaled,
							&surface_info_scaled,
							time_delta_s,
							&mut active_map.debug_stats_printer,
						);
					}
					else
					{
						active_map
							.renderer
							.prepare_frame::<Color32>(&surface_info_scaled, frame_info_ref);

						active_map.renderer.draw_frame(
							pixels_scaled,
							&surface_info_scaled,
							frame_info_ref,
							&mut active_map.debug_stats_printer,
						);
					}

					active_map.game.draw_frame_overlay(pixels_scaled, &surface_info_scaled);
				}
				else
				{
					// Get frame info from game code.
					frame_info = Some(active_map.game.get_frame_info(&surface_info_initial));
					let frame_info_ref = frame_info.as_ref().unwrap();

					if postprocessor.use_hdr_rendering()
					{
						// Postprocessing is enabled - perform frame preparation and rendering.

						let hdr_buffer_size = [surface_info_initial.width, surface_info_initial.height];
						let hdr_buffer = postprocessor.get_hdr_buffer(hdr_buffer_size);

						let hdr_surface_info = system_window::SurfaceInfo {
							width: hdr_buffer_size[0],
							height: hdr_buffer_size[1],
							pitch: hdr_buffer_size[0],
						};

						active_map
							.renderer
							.prepare_frame::<Color64>(&hdr_surface_info, frame_info_ref);

						active_map.renderer.draw_frame(
							hdr_buffer,
							&hdr_surface_info,
							frame_info_ref,
							&mut active_map.debug_stats_printer,
						);
					}
					else
					{
						// No postprocessing and no intermediate buffer - only prepare frame.
						active_map
							.renderer
							.prepare_frame::<Color32>(&surface_info_initial, frame_info_ref);
					}
				}
			}
		};

		let mut limit_fps_func = || {
			if max_fps > 0.0
			{
				let now = std::time::Instant::now();
				let frame_duration_s = (now - *prev_frame_end_time).as_secs_f32();
				let min_frame_time = 1.0 / max_fps;
				if frame_duration_s < min_frame_time
				{
					std::thread::sleep(Duration::from_secs_f32(
						((min_frame_time - frame_duration_s) * 1000.0).floor() / 1000.0,
					));
				}
			}
			*prev_frame_end_time = std::time::Instant::now();
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
				if frame_scale > 1
				{
					// In case of scaled buffer just perform upscaling into screen buffer.

					frame_upscaler.perform_upscale(
						pixels,
						surface_info,
						frame_scale as usize,
						frame_resize_interpolate,
					);
				}
				else
				{
					if postprocessor.use_hdr_rendering()
					{
						// Perform postprocessing into screen buffer.
						postprocessor.perform_postprocessing(
							pixels,
							surface_info,
							time_delta_s,
							&mut active_map.debug_stats_printer,
						);
					}
					else
					{
						// No intermediate buffer or postprocessing - perform rendering directly into screen buffer.
						let frame_info_ref = frame_info.as_ref().unwrap();
						active_map.renderer.draw_frame(
							pixels,
							surface_info,
							frame_info_ref,
							&mut active_map.debug_stats_printer,
						);
					}

					active_map.game.draw_frame_overlay(pixels, surface_info);
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

			if *screenshot_requested
			{
				*screenshot_requested = false;
				save_screenshot(pixels, surface_info);
			}
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

		let map_name = &args[0];

		let loading_text = format!("Loading map \"{}\"", map_name);

		let map_loading_start_time = std::time::Instant::now();
		self.console.lock().unwrap().add_text(loading_text.clone());

		// Draw single frame with loading text.
		self.window.update_window_surface(|pixels, surface_info| {
			for pixel in pixels.iter_mut()
			{
				*pixel = pixel.get_half_dark();
			}

			text_printer::print(
				pixels,
				surface_info,
				&loading_text,
				((surface_info.width / 2) as i32) - ((loading_text.len() * text_printer::GLYPH_WIDTH / 2) as i32),
				((surface_info.height / 2) as i32) - ((text_printer::GLYPH_HEIGHT / 2) as i32),
				Color32::from_rgb(255, 255, 255),
			);
		});
		self.window.swap_buffers();

		// Perform actual map loading.
		let map_opt = self.resources_manager.lock().unwrap().get_map(&args[0]);
		if let Some(map) = map_opt
		{
			let game_creation_function = self.game_creation_function;
			self.active_map = Some(ActiveMap {
				game: game_creation_function(
					self.app_config.clone(),
					self.commands_processor.clone(),
					self.console.clone(),
					self.resources_manager.clone(),
					map.clone(),
				),
				renderer: renderer::Renderer::new(self.resources_manager.clone(), self.app_config.clone(), map.clone()),
				debug_stats_printer: DebugStatsPrinter::new(self.config.show_debug_stats),
			});

			// Clear unused resources from previous map.
			self.resources_manager.lock().unwrap().clear_cache();

			let map_loading_end_time = std::time::Instant::now();
			let loading_duration = (map_loading_end_time - map_loading_start_time).as_secs_f32();
			self.console.lock().unwrap().add_text(format!(
				"Loading map \"{}\" finished in {} seconds",
				map_name, loading_duration
			));
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

	fn command_screenshot(&mut self, _args: commands_queue::CommandArgs)
	{
		self.screenshot_requested = true;
		self.console.lock().unwrap().add_text("Making screenshot".to_string());
	}
}

impl Drop for Host
{
	fn drop(&mut self)
	{
		config::save(&self.app_config.lock().unwrap(), &self.config_file_path);
	}
}
