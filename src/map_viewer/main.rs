use common::{color::*, debug_renderer::*, fixed_math::*, map_file, math_types::*, system_window};
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
				map_file_parsed.as_ref(),
			)
		});

		std::thread::sleep(Duration::from_secs_f32(frame_duration_s));
	}
}

fn draw_frame(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	view_matrix: &Mat4f,
	map: Option<&map_file::MapFileParsed>,
)
{
	draw_background(pixels);
	draw_map(pixels, surface_info, view_matrix, map);
}

fn draw_background(pixels: &mut [Color32])
{
	for pixel in pixels.iter_mut()
	{
		*pixel = Color32::from_rgb(16, 8, 4);
	}
}

fn draw_map(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	view_matrix: &Mat4f,
	map: Option<&map_file::MapFileParsed>,
)
{
	let mut renderer = DebugRenderer::new(pixels, surface_info);

	let fixed_scale = FIXED16_ONE as f32;
	let mat = Mat4f::from_nonuniform_scale(fixed_scale, fixed_scale, 1.0) * view_matrix;

	if let Some(entities) = map
	{
		for entity in entities
		{
			for (brush_number, brush) in entity.brushes.iter().enumerate()
			{
				let bush_number_scaled = brush_number * 16;
				let color = Color32::from_rgb(
					(bush_number_scaled & 255) as u8,
					((bush_number_scaled * 3) & 255) as u8,
					((bush_number_scaled * 5) & 255) as u8,
				);

				for brush_plane in brush
				{
					let lines = [
						(brush_plane.vertices[0], brush_plane.vertices[1], color),
						(brush_plane.vertices[1], brush_plane.vertices[2], color),
						(brush_plane.vertices[2], brush_plane.vertices[0], color),
					];
					for line in lines
					{
						draw_line(&mut renderer, &mat, &line);
					}
				}
			}
		}
	}

	let basis_lines = [
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(1.0, 0.0, 0.0),
			Color32::from_rgb(255, 0, 0),
		),
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(0.0, 1.0, 0.0),
			Color32::from_rgb(0, 255, 0),
		),
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(0.0, 0.0, 1.0),
			Color32::from_rgb(0, 0, 255),
		),
	];

	for line in &basis_lines
	{
		draw_line(&mut renderer, &mat, line);
	}
}

type WorldLine = (Vec3f, Vec3f, Color32);

fn draw_line(renderer: &mut DebugRenderer, transform_matrix: &Mat4f, line: &WorldLine)
{
	let fixed_scale = FIXED16_ONE as f32;
	let width = (renderer.get_width() as f32) * fixed_scale;
	let height = (renderer.get_width() as f32) * fixed_scale;

	let v0 = transform_matrix * line.0.extend(1.0);
	let v1 = transform_matrix * line.1.extend(1.0);

	// TODO - perform proper clipping
	if v0.w <= 0.1 || v1.w <= 0.1
	{
		return;
	}
	let v0 = v0.truncate() / v0.w;
	let v1 = v1.truncate() / v1.w;

	if v0.x < 0.0 ||
		v0.x > width ||
		v0.y < 0.0 ||
		v0.y > height ||
		v1.x < 0.0 ||
		v1.x > width ||
		v1.y < 0.0 ||
		v1.y > height
	{
		return;
	}

	renderer.draw_line(
		PointProjected {
			x: v0.x as Fixed16,
			y: v0.y as Fixed16,
			z: v0.z,
		},
		PointProjected {
			x: v1.x as Fixed16,
			y: v1.y as Fixed16,
			z: v1.z,
		},
		line.2,
	);
}
