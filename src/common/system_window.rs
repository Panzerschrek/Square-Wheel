use super::color::*;

pub struct SystemWindow
{
	sdl2_event_pump: sdl2::EventPump,
	sdl2_mouse: sdl2::mouse::MouseUtil,

	canvas: sdl2::render::WindowCanvas,
	texture_creator: sdl2::render::TextureCreator<sdl2::video::WindowContext>,

	// HACK! SDL2-wrapper creates texture object as linked to texture_creator. So, we can't put into this texture.
	// Because of that create raw texture manually.
	raw_texture: *mut sdl2::sys::SDL_Texture,
	current_texture_size: (u32, u32),
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
	pub fn new(render_index: u32) -> Self
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

		let canvas = sdl2::render::CanvasBuilder::new(window)
			.accelerated()
			.index(render_index)
			.build()
			.unwrap();

		println!("Render name: {}", canvas.info().name);

		let texture_creator = canvas.texture_creator();

		let event_pump = context.event_pump().unwrap();
		SystemWindow {
			sdl2_event_pump: event_pump,
			sdl2_mouse: context.mouse(),

			canvas,
			texture_creator,
			raw_texture: std::ptr::null_mut(),
			current_texture_size: (0, 0),
		}
	}

	pub fn resize(&mut self, width: u32, height: u32)
	{
		let _gnore = self.canvas.window_mut().set_size(
			width.max(MIN_WIDTH).min(MAX_WIDTH),
			height.max(MIN_HEIGHT).min(MAX_HEIGHT),
		);
	}

	pub fn set_windowed(&mut self)
	{
		let _gnore = self
			.canvas
			.window_mut()
			.set_fullscreen(sdl2::video::FullscreenType::Off);
	}

	pub fn set_fullscreen_desktop(&mut self)
	{
		let _gnore = self
			.canvas
			.window_mut()
			.set_fullscreen(sdl2::video::FullscreenType::Desktop);
	}

	pub fn set_fullscreen(&mut self)
	{
		// TODO - resize window properly before this?
		let _gnore = self
			.canvas
			.window_mut()
			.set_fullscreen(sdl2::video::FullscreenType::True);
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
		// TODO - fix wrong pitch.
		let texture_size = self.canvas.output_size().unwrap();
		SurfaceInfo {
			width: texture_size.0 as usize,
			height: texture_size.1 as usize,
			pitch: texture_size.0 as usize,
		}
	}

	pub fn update_window_surface<F: FnOnce(&mut [Color32], &SurfaceInfo)>(&mut self, draw_fn: F)
	{
		let texture_size = self.canvas.output_size().unwrap();
		if texture_size != self.current_texture_size
		{
			self.create_texture();
		}
		if self.raw_texture.is_null()
		{
			return;
		}

		unsafe {
			let mut pixels_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
			let mut pitch: i32 = 0;
			sdl2::sys::SDL_LockTexture(
				self.raw_texture,
				std::ptr::null(),
				&mut pixels_ptr as *mut *mut core::ffi::c_void,
				&mut pitch as *mut i32,
			);
			if pixels_ptr.is_null()
			{
				return;
			}

			let pixels = std::slice::from_raw_parts_mut(pixels_ptr, pitch as usize * texture_size.1 as usize);
			{
				let surface_info = SurfaceInfo {
					width: texture_size.0 as usize,
					height: texture_size.1 as usize,
					pitch: pitch as usize / 4,
				};

				let pixels_32 = pixels.align_to_mut::<Color32>().1;
				draw_fn(pixels_32, &surface_info);
			}

			sdl2::sys::SDL_UnlockTexture(self.raw_texture);
		}
	}

	pub fn swap_buffers(&mut self)
	{
		if self.raw_texture.is_null()
		{
			return;
		}

		unsafe {
			sdl2::sys::SDL_RenderCopy(self.canvas.raw(), self.raw_texture, std::ptr::null(), std::ptr::null());
		}
		self.canvas.present();
	}

	fn create_texture(&mut self)
	{
		self.delete_raw_texture();

		let texture_size = self.canvas.output_size().unwrap();
		// 	let pixel_format = self.texture_creator.default_pixel_format();
		let pixel_format = sdl2::pixels::PixelFormatEnum::ARGB8888;

		unsafe {
			let raw_texture = sdl2::sys::SDL_CreateTexture(
				self.texture_creator.raw(),
				pixel_format as u32,
				sdl2::sys::SDL_TextureAccess::SDL_TEXTUREACCESS_STREAMING as i32,
				texture_size.0 as i32,
				texture_size.1 as i32,
			);
			if raw_texture.is_null()
			{
				return;
			}

			self.raw_texture = raw_texture;
		}
		self.current_texture_size = texture_size;
	}

	fn delete_raw_texture(&mut self)
	{
		if !self.raw_texture.is_null()
		{
			unsafe {
				sdl2::sys::SDL_DestroyTexture(self.raw_texture);
			}
			self.raw_texture = std::ptr::null_mut();
		}
	}
}

impl Drop for SystemWindow
{
	fn drop(&mut self)
	{
		self.delete_raw_texture();
	}
}

const MIN_WIDTH: u32 = 320;
const MIN_HEIGHT: u32 = 200;
const MAX_WIDTH: u32 = 4000;
const MAX_HEIGHT: u32 = 3000;
