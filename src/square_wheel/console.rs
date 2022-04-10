use super::commands_queue;
use common::{color::*, system_window, text_printer};
use sdl2::keyboard::Keycode;

pub struct Console
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
	is_active: bool,
	start_time: std::time::Instant,
	input_line: String,
}

impl Console
{
	pub fn new() -> Self
	{
		Console {
			commands_queues: Vec::new(),
			is_active: false,
			start_time: std::time::Instant::now(),
			input_line: String::new(),
		}
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}

	pub fn toggle(&mut self)
	{
		self.is_active = !self.is_active;
	}

	pub fn process_text_input(&mut self, text: &str)
	{
		self.input_line += text;
	}

	pub fn process_key_press(&mut self, key_code: Keycode)
	{
		if key_code == Keycode::Return
		{
			self.input_line.clear();
		}
		if key_code == Keycode::Backspace
		{
			self.input_line.pop();
		}
	}

	pub fn is_active(&self) -> bool
	{
		self.is_active
	}

	pub fn draw(&self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		if !self.is_active
		{
			return;
		}

		let console_pos = surface_info.height / 2;

		// Make bacground darker.
		for y in 0 .. console_pos
		{
			let dst = &mut pixels[y * surface_info.pitch .. (y + 1) * surface_info.pitch];
			for pix in dst
			{
				*pix = pix.get_half_dark();
			}
		}

		let mut text = "> ".to_string();
		text += &self.input_line;

		// Add blinking cursor.
		if ((self.start_time.elapsed().as_millis() / 500) & 1) != 0
		{
			text += "_";
		}

		let color = Color32::from_rgb(255, 255, 255);

		text_printer::print(
			pixels,
			surface_info,
			&text,
			0,
			(console_pos - text_printer::GLYPH_HEIGHT) as i32,
			color,
		);
	}
}
