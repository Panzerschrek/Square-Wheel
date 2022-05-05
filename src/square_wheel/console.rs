use super::commands_processor;
use common::{color::*, system_window, text_printer};
use sdl2::keyboard::Keycode;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

pub struct Console
{
	commands_processor: commands_processor::CommandsProcessorPtr,
	is_active: bool,
	start_time: std::time::Instant,
	lines: VecDeque<String>,
	input_history: VecDeque<String>,
	current_history_index: usize,
	input_line: String,
}

pub type ConsoleSharedPtr = Rc<RefCell<Console>>;

impl Console
{
	pub fn new(commands_processor: commands_processor::CommandsProcessorPtr) -> ConsoleSharedPtr
	{
		Rc::new(RefCell::new(Console {
			commands_processor,
			is_active: false,
			start_time: std::time::Instant::now(),
			lines: VecDeque::with_capacity(LINES_BUFFER_LEN),
			input_history: VecDeque::with_capacity(LINES_BUFFER_LEN),
			current_history_index: 0,
			input_line: String::new(),
		}))
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
		if key_code == Keycode::Up
		{
			if self.current_history_index > 0
			{
				self.current_history_index -= 1;
				if self.current_history_index < self.input_history.len()
				{
					self.input_line = self.input_history[self.current_history_index].clone();
				}
			}
		}
		if key_code == Keycode::Down
		{
			if self.current_history_index < self.input_history.len()
			{
				self.current_history_index += 1;
				if self.current_history_index < self.input_history.len()
				{
					self.input_line = self.input_history[self.current_history_index].clone();
				}
				else
				{
					self.input_line = String::new();
				}
			}
		}
		if key_code == Keycode::Tab
		{
			let mut completion_result = self.commands_processor.borrow().complete_command(&self.input_line);
			if completion_result.len() == 1
			{
				self.input_line = completion_result.pop().unwrap();
			}
			else
			{
				self.add_text("> ".to_string());
				for possible_command in completion_result.drain(..)
				{
					self.add_text(format!(" {}", possible_command));
				}
			}
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
		if self.input_history.len() >= HISTORY_BUFFER_LEN
		{
			self.input_history.pop_front();
		}
		self.input_history.push_back(self.input_line.clone());
		self.current_history_index = self.input_history.len();

		let command_process_text = self.commands_processor.borrow_mut().process_command(&self.input_line);
		self.add_text(self.input_line.clone());
		self.input_line.clear();

		if !command_process_text.is_empty()
		{
			self.add_text(command_process_text);
		}
	}
}

const LINES_BUFFER_LEN: usize = 128;
const HISTORY_BUFFER_LEN: usize = 32;
