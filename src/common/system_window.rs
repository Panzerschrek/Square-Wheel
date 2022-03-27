use super::color::*;

pub struct SystemWindow
{
	sdl2_window: sdl2::video::Window,
	sdl2_event_pump: sdl2::EventPump,
}

pub struct SurfaceInfo
{
	pub width: usize,
	pub height: usize,
	pub pitch: usize,
}

impl SystemWindow
{
	pub fn new() -> Self
	{
		let context = sdl2::init().unwrap();

		let mut window = context
			.video()
			.unwrap()
			.window("Square Wheel", 800, 600)
			.resizable()
			.position_centered()
			.build()
			.unwrap();

		window.set_minimum_size(320, 200).unwrap();

		let event_pump = context.event_pump().unwrap();
		SystemWindow {
			sdl2_window: window,
			sdl2_event_pump: event_pump,
		}
	}

	pub fn get_events(&mut self) -> Vec<sdl2::event::Event>
	{
		self.sdl2_event_pump.poll_iter().collect()
	}

	pub fn get_keyboard_state(&mut self) -> sdl2::keyboard::KeyboardState
	{
		self.sdl2_event_pump.keyboard_state()
	}

	pub fn end_frame<F: FnOnce(&mut [Color32], &SurfaceInfo)>(&mut self, draw_fn: F)
	{
		let mut surface = self.sdl2_window.surface(&self.sdl2_event_pump).unwrap();

		let surface_info = SurfaceInfo {
			width: surface.width() as usize,
			height: surface.height() as usize,
			pitch: surface.pitch() as usize / 4,
		};

		surface.with_lock_mut(|pixels| {
			// Pixels must be 4-byte aligned.
			let pixels_32 = unsafe { pixels.align_to_mut::<Color32>().1 };
			draw_fn(pixels_32, &surface_info)
		});

		let _ = surface.update_window();
	}
}
