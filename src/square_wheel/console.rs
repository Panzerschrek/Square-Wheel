use super::commands_queue;
use common::{color::*, system_window, text_printer};

pub struct Console
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
	is_active: bool,
	start_time: std::time::Instant,
}

impl Console
{
	pub fn new() -> Self
	{
		Console {
			commands_queues: Vec::new(),
			is_active: false,
			start_time: std::time::Instant::now(),
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
