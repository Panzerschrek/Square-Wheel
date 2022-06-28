use super::text_printer;
use common::{color::*, system_window};

pub struct DebugStatsPrinter
{
	buffer: String,
	show_debug_stats: bool,
}

impl DebugStatsPrinter
{
	pub fn new(show_debug_stats: bool) -> Self
	{
		Self {
			show_debug_stats,
			buffer: String::new(),
		}
	}

	pub fn show_debug_stats(&self) -> bool
	{
		self.show_debug_stats
	}

	pub fn add_line(&mut self, line: String)
	{
		self.buffer += &line;
		self.buffer.push('\n');
	}

	pub fn flush(&mut self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		text_printer::print(
			pixels,
			surface_info,
			&self.buffer,
			0,
			0,
			Color32::from_rgb(255, 255, 255),
		);
		self.buffer.clear();
	}
}
