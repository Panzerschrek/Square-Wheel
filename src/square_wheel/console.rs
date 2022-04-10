use super::commands_queue;
use common::{color::*, system_window, text_printer};
use sdl2::keyboard::Keycode;

pub struct Console
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
	is_active: bool,
	start_time: std::time::Instant,
	lines: std::collections::VecDeque<String>,
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
			lines: std::collections::VecDeque::with_capacity(LINES_BUFFER_LEN),
			input_line: String::new(),
		}
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}

	pub fn add_text(&mut self, text: String)
	{
		if self.lines.len() >= LINES_BUFFER_LEN
		{
			self.lines.pop_front();
		}
		self.lines.push_back(text);
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
			self.process_enter();
		}
		if key_code == Keycode::Backspace
		{
			self.input_line.pop();
		}
		// Todo - implement completion.
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

		let mut y = (console_pos - text_printer::GLYPH_HEIGHT) as i32;
		text_printer::print(pixels, surface_info, &text, 0, y, color);

		for line in self.lines.iter().rev()
		{
			y -= text_printer::GLYPH_HEIGHT as i32;
			if y <= -(text_printer::GLYPH_HEIGHT as i32)
			{
				break;
			}

			text_printer::print(pixels, surface_info, &line, 0, y, color);
		}
	}

	fn process_enter(&mut self)
	{
		let mut command = None;
		let mut args = Vec::<String>::new();
		for token in self.input_line.split_ascii_whitespace()
		{
			if command.is_none()
			{
				command = Some(token.to_string());
			}
			else
			{
				args.push(token.to_string());
			}
		}

		self.add_text(self.input_line.clone());
		self.input_line.clear();

		if let Some(c) = command
		{
			for queue in &self.commands_queues
			{
				if queue.borrow().has_handler(&c)
				{
					queue.borrow_mut().add_invocation(&c, args);
					return;
				}
			}

			// TODO - try to modify config here if command name is valid config variable.
			self.add_text(format!("{}: not found", c));
		}
	}
}

const LINES_BUFFER_LEN: usize = 128;
