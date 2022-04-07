use common::{bsp_map_compact, bsp_map_save_load, debug_renderer, system_window};
use sdl2::{event::Event, keyboard::Keycode};
use std::{path::PathBuf, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "squar_wheel", about = "SquareWheel engine.")]
struct Opt
{
	/// Input file
	#[structopt(parse(from_os_str), short = "i")]
	input: Option<PathBuf>,
}

pub fn main()
{
	let opt = Opt::from_args();

	let mut window = system_window::SystemWindow::new();
	let mut camera_controller = common::camera_controller::CameraController::new();

	let mut map_bsp_compact_opt = None;
	if let Some(path) = &opt.input
	{
		map_bsp_compact_opt = bsp_map_save_load::load_map(path).unwrap();
	}

	let mut prev_time = std::time::Instant::now();

	'running: loop
	{
		for event in window.get_events()
		{
			match event
			{
				Event::Quit { .. } |
				Event::KeyDown {
					keycode: Some(Keycode::Escape),
					..
				} => break 'running,
				_ =>
				{},
			}
		}

		let cur_time = std::time::Instant::now();
		let time_delta_s = (cur_time - prev_time).as_secs_f32();
		prev_time = cur_time;

		camera_controller.update(&window.get_keyboard_state(), time_delta_s);

		window.end_frame(|pixels, surface_info| {});

		let frame_end_time = std::time::Instant::now();
		let frame_time_s = (frame_end_time - prev_time).as_secs_f32();
		let min_frame_time = 0.01;
		if frame_time_s < min_frame_time
		{
			std::thread::sleep(Duration::from_secs_f32(min_frame_time - frame_time_s));
		}
	}
}
