use super::color::*;

pub struct SystemWindow
{
	sdl2_window: sdl2::video::Window,
	sdl2_event_pump: sdl2::EventPump,
	sdl2_mouse: sdl2::mouse::MouseUtil,
}

#[derive(PartialEq)]
pub struct SurfaceInfo
{
	pub width: usize,
	pub height: usize,
	pub pitch: usize,
}

pub type KeyboardState = std::collections::HashSet<sdl2::keyboard::Scancode>;

impl SystemWindow
{
	pub fn new() -> Self
	{
		let context = sdl2::init().unwrap();

		let mut window = context
			.video()
			.unwrap()
			.window("Square Wheel", 640, 480)
			.resizable()
			.position_centered()
			.build()
			.unwrap();

		window.set_minimum_size(MIN_WIDTH, MIN_HEIGHT).unwrap();

		let event_pump = context.event_pump().unwrap();
		SystemWindow {
			sdl2_window: window,
			sdl2_event_pump: event_pump,
			sdl2_mouse: context.mouse(),
		}
	}

	pub fn resize(&mut self, width: u32, height: u32)
	{
		let _gnore = self.sdl2_window.set_size(
			width.max(MIN_WIDTH).min(MAX_WIDTH),
			height.max(MIN_HEIGHT).min(MAX_HEIGHT),
		);
	}

	pub fn set_windowed(&mut self)
	{
		let _gnore = self.sdl2_window.set_fullscreen(sdl2::video::FullscreenType::Off);
	}

	pub fn set_fullscreen_desktop(&mut self)
	{
		let _gnore = self.sdl2_window.set_fullscreen(sdl2::video::FullscreenType::Desktop);
	}

	pub fn set_fullscreen(&mut self)
	{
		// TODO - resize window properly before this?
		let _gnore = self.sdl2_window.set_fullscreen(sdl2::video::FullscreenType::True);
	}

	pub fn set_relative_mouse(&mut self, relative: bool)
	{
		self.sdl2_mouse.set_relative_mouse_mode(relative);
	}

	pub fn get_events(&mut self) -> Vec<sdl2::event::Event>
	{
		self.sdl2_event_pump.poll_iter().collect()
	}

	pub fn get_keyboard_state(&mut self) -> KeyboardState
	{
		self.sdl2_event_pump.keyboard_state().pressed_scancodes().collect()
	}

	pub fn get_window_surface_info(&self) -> SurfaceInfo
	{
		let surface = self.sdl2_window.surface(&self.sdl2_event_pump).unwrap();
		SurfaceInfo {
			width: surface.width() as usize,
			height: surface.height() as usize,
			pitch: surface.pitch() as usize / 4,
		}
	}

	pub fn update_window_surface<F: FnOnce(&mut [Color32], &SurfaceInfo)>(&mut self, draw_fn: F)
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
	}

	pub fn swap_buffers(&mut self)
	{
		let surface = self.sdl2_window.surface(&self.sdl2_event_pump).unwrap();
		let _ = surface.update_window();
	}
}

const MIN_WIDTH: u32 = 320;
const MIN_HEIGHT: u32 = 200;
const MAX_WIDTH: u32 = 4000;
const MAX_HEIGHT: u32 = 3000;
