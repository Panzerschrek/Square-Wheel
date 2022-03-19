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

	pub fn get_events(&mut self) -> Vec<sdl2::event::Event> { self.sdl2_event_pump.poll_iter().collect() }

	pub fn end_frame<F: FnOnce(&mut [u8], &SurfaceInfo)>(&mut self, draw_fn: F)
	{
		let mut surface = self.sdl2_window.surface(&self.sdl2_event_pump).unwrap();

		let surface_info = SurfaceInfo {
			width: surface.width() as usize,
			height: surface.height() as usize,
			pitch: surface.pitch() as usize,
		};

		surface.with_lock_mut(|pixels| draw_fn(pixels, &surface_info));

		let _ = surface.update_window();
	}
}
