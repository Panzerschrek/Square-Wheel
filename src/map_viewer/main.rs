use common::{color::*, map_file, system_window};
use sdl2::{event::Event, keyboard::Keycode};
use std::{path::PathBuf, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "map_compiler", about = "SquareWheel map compiler.")]
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

	let mut map_file_parsed: Option<map_file::MapFileParsed> = None;
	if let Some(path) = opt.input
	{
		let file_contents_str = std::fs::read_to_string(path).unwrap();
		map_file_parsed = map_file::parse_map_file_content(&file_contents_str).ok();
	}

	let frame_duration_s = 1.0 / 30.0;
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

		camera_controller.update(&window.get_keyboard_state(), frame_duration_s);

		window.end_frame(|pixels, surface_info| {
			draw_frame(
				pixels,
				surface_info,
				&camera_controller.build_view_matrix(surface_info.width as f32, surface_info.height as f32),
			)
		});

		std::thread::sleep(Duration::from_secs_f32(frame_duration_s));
	}
}

fn draw_frame(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	view_matrix: &common::math_types::Mat4f,
)
{
	draw_background(pixels, surface_info);
	draw_lines(pixels, surface_info, view_matrix);
}

fn draw_background(pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
{
	for x in 0 .. surface_info.width
	{
		for y in 0 .. surface_info.height
		{
			let index = x + y * surface_info.pitch;
			pixels[index] = Color32::from_rgb(((x + y * 2) & 255) as u8, 0, 0);
		}
	}
}

fn draw_lines(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	view_matrix: &common::math_types::Mat4f,
)
{
	use common::{debug_renderer::*, fixed_math::*, math_types::*};

	let lines = [
		(
			Vec3f::new(1.0, 1.0, 1.0),
			Vec3f::new(2.0, 1.1, 1.0),
			Color32::from_rgb(255, 0, 0),
		),
		(
			Vec3f::new(1.0, 1.0, 1.0),
			Vec3f::new(1.1, 2.0, 1.0),
			Color32::from_rgb(0, 255, 0),
		),
		(
			Vec3f::new(1.0, 1.0, 1.0),
			Vec3f::new(1.0, 1.0, 2.5),
			Color32::from_rgb(0, 0, 255),
		),
	];

	let mut renderer = DebugRenderer::new(pixels, surface_info);

	let fixed_scale = FIXED16_ONE as f32;
	let width = (surface_info.width as f32) * fixed_scale;
	let height = (surface_info.height as f32) * fixed_scale;
	let mat = Mat4f::from_nonuniform_scale(fixed_scale, fixed_scale, 1.0) * view_matrix;
	for line in lines
	{
		let v0 = mat * line.0.extend(1.0);
		let v1 = mat * line.1.extend(1.0);

		// TODO - perform proper clipping
		if v0.w <= 0.1 || v1.w <= 0.1
		{
			continue;
		}
		let v0 = v0.truncate() / v0.w;
		let v1 = v1.truncate() / v1.w;

		if v0.x < 0.0 ||
			v0.x > width || v0.y < 0.0 ||
			v0.y > height ||
			v1.x < 0.0 || v1.x > width ||
			v1.y < 0.0 || v1.y > height
		{
			continue;
		}

		renderer.draw_line(
			PointProjected {
				x: v0.x as Fixed16,
				y: v0.y as Fixed16,
			},
			PointProjected {
				x: v1.x as Fixed16,
				y: v1.y as Fixed16,
			},
			line.2,
		);
	}
}
