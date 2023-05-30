use super::{
	commands_processor, config, console, frame_info::*, game_interface::*, resources_manager::*, text_printer,
};
use square_wheel_lib::common::{bsp_map_compact::*, color::*, system_window};
use std::sync::Arc;

use super::game_map::*;

pub struct Game
{
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	resources_manager: ResourcesManagerSharedPtr,
	game_map: Option<GameMap>,
}

impl Game
{
	pub fn new(
		_config: config::ConfigSharedPtr,
		commands_processor: commands_processor::CommandsProcessorPtr,
		console: console::ConsoleSharedPtr,
		resources_manager: ResourcesManagerSharedPtr,
	) -> Self
	{
		Self {
			commands_processor,
			console,
			resources_manager,
			game_map: None,
		}
	}
}

impl GameInterface for Game
{
	fn update(
		&mut self,
		keyboard_state: &system_window::KeyboardState,
		events: &[sdl2::event::Event],
		time_delta_s: f32,
	)
	{
		for event in events
		{
			if let sdl2::event::Event::KeyDown {
				keycode: Some(sdl2::keyboard::Keycode::Escape),
				..
			} = event
			{
				self.commands_processor.lock().unwrap().process_command("quit");
			}
		}

		if let Some(game_map) = &mut self.game_map
		{
			game_map.update(keyboard_state, events, time_delta_s);
		}

		// Process here menu, for example.
	}

	fn grab_mouse_input(&self) -> bool
	{
		if let Some(game_map) = &self.game_map
		{
			game_map.grab_mouse_input()
		}
		else
		{
			false
		}
	}

	fn set_map(&mut self, map: Option<Arc<BSPMap>>)
	{
		self.game_map = None;
		if let Some(m) = map
		{
			self.game_map = Some(GameMap::new(
				self.commands_processor.clone(),
				self.console.clone(),
				self.resources_manager.clone(),
				m,
			));
		}
	}

	fn get_frame_info(&self, surface_info: &system_window::SurfaceInfo) -> Option<FrameInfo>
	{
		self.game_map.as_ref().map(|m| m.get_frame_info(surface_info))
	}

	fn draw_frame_overlay(&self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		if let Some(game_map) = &self.game_map
		{
			game_map.draw_frame_overlay(pixels, surface_info)
		}
		else
		{
			// Can draw here menu, for example.
			for pixel in pixels.iter_mut()
			{
				*pixel = Color32::black();
			}
		}
	}

	fn get_draw_loading_screen_function(&self) -> LoadingStringDrawFunction
	{
		draw_loading_screen
	}
}

fn draw_loading_screen(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	map_name: &str,
	laoding_step: usize,
)
{
	let progress_bar_cells = [
		[0, 0],
		[1, 0],
		[2, 0],
		[3, 0],
		[3, 1],
		[3, 2],
		[3, 3],
		[2, 3],
		[1, 3],
		[0, 3],
		[0, 2],
		[0, 1],
	];

	let cell_size = 16;
	let padding = 1;
	let current_step = laoding_step % progress_bar_cells.len();

	for (cell_index, cell) in progress_bar_cells.iter().enumerate()
	{
		let start_x = surface_info.width / 2 - cell_size * 2;
		let start_y = surface_info.height / 2 + cell_size * 2;

		let dist = (progress_bar_cells.len() + cell_index - current_step) % progress_bar_cells.len();
		let brightness = (255 * dist / progress_bar_cells.len()) as u8;
		let color = Color32::from_rgb(brightness, brightness, brightness);

		for y in start_y + cell[1] * cell_size + padding .. start_y + (cell[1] + 1) * cell_size - padding
		{
			let dst_line = &mut pixels[y * surface_info.pitch .. (y + 1) * surface_info.pitch];
			for pixel in
				&mut dst_line[start_x + cell[0] * cell_size + padding .. start_x + (cell[0] + 1) * cell_size - padding]
			{
				*pixel = color;
			}
		}
	}

	let loading_text = format!("Entering map \"{}\"", map_name);

	text_printer::print(
		pixels,
		surface_info,
		&loading_text,
		((surface_info.width / 2) as i32) - ((loading_text.len() * text_printer::GLYPH_WIDTH / 2) as i32),
		((surface_info.height / 2) as i32) - ((text_printer::GLYPH_HEIGHT / 2) as i32),
		Color32::from_rgb(255, 255, 255),
	);
}
