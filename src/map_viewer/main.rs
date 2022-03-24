use common::{color::*, debug_rasterizer::*, fixed_math::*, map_file, map_polygonizer, math_types::*, system_window};
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
	let mut map_polygonized: Option<map_polygonizer::MapPolygonized> = None;
	if let Some(path) = opt.input
	{
		let file_contents_str = std::fs::read_to_string(path).unwrap();
		map_file_parsed = map_file::parse_map_file_content(&file_contents_str).ok();
		if let Some(map_file) = &map_file_parsed
		{
			map_polygonized = Some(map_polygonizer::polygonize_map(map_file));
		}
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
				map_polygonized.as_ref(),
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
	map_polygonized: Option<&map_polygonizer::MapPolygonized>,
)
{
	draw_background(pixels);
	draw_map(pixels, surface_info, view_matrix, map, map_polygonized);
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
	map_polygonized: Option<&map_polygonizer::MapPolygonized>,
)
{
	let draw_raw_map = false;
	let draw_polygonized_map = true;
	let draw_only_first_entity = false;

	let mut rasterizer = DebugRasterizer::new(pixels, surface_info);

	let fixed_scale = FIXED16_ONE as f32;
	let mat = Mat4f::from_nonuniform_scale(fixed_scale, fixed_scale, 1.0) * view_matrix;

	if draw_raw_map
	{
		if let Some(entities) = map
		{
			for entity in entities
			{
				for (brush_number, brush) in entity.brushes.iter().enumerate()
				{
					let color = get_pseudo_random_color(brush_number);

					for brush_plane in brush
					{
						draw_triangle(&mut rasterizer, &mat, &brush_plane.vertices, color);
					}
				}
				if draw_only_first_entity
				{
					break;
				}
			}
		}
	}

	if draw_polygonized_map
	{
		if let Some(entities) = map_polygonized
		{
			for entity in entities
			{
				for (polygon_number, polygon) in entity.polygons.iter().enumerate()
				{
					if polygon.vertices.len() < 3
					{
						continue;
					}
					let color = get_pseudo_random_color(polygon_number);

					for i in 0 .. polygon.vertices.len() - 2
					{
						let vertices = [polygon.vertices[0], polygon.vertices[i + 1], polygon.vertices[i + 2]];
						draw_triangle(&mut rasterizer, &mat, &vertices, color);
					}
				}
				if draw_only_first_entity
				{
					break;
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
		draw_line(&mut rasterizer, &mat, line);
	}
}

fn get_pseudo_random_color(num: usize) -> Color32
{
	let num = num * 16;
	Color32::from_rgb((num & 255) as u8, ((num * 3) & 255) as u8, ((num * 5) & 255) as u8)
}

type WorldLine = (Vec3f, Vec3f, Color32);

fn draw_line(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, line: &WorldLine)
{
	let fixed_scale = FIXED16_ONE as f32;
	let width = (rasterizer.get_width() as f32) * fixed_scale;
	let height = (rasterizer.get_width() as f32) * fixed_scale;

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

	rasterizer.draw_line(
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

fn draw_triangle(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, vertices: &[Vec3f; 3], color: Color32)
{
	let fixed_scale = FIXED16_ONE as f32;
	let width = (rasterizer.get_width() as f32) * fixed_scale;
	let height = (rasterizer.get_width() as f32) * fixed_scale;

	let v0 = transform_matrix * vertices[0].extend(1.0);
	let v1 = transform_matrix * vertices[1].extend(1.0);
	let v2 = transform_matrix * vertices[2].extend(1.0);

	// TODO - perform proper clipping
	if v0.w <= 0.1 || v1.w <= 0.1 || v2.w <= 0.1
	{
		return;
	}
	let v0 = v0.truncate() / v0.w;
	let v1 = v1.truncate() / v1.w;
	let v2 = v2.truncate() / v2.w;

	if v0.x < 0.0 ||
		v0.x > width ||
		v0.y < 0.0 ||
		v0.y > height ||
		v1.x < 0.0 ||
		v1.x > width ||
		v1.y < 0.0 ||
		v1.y > height ||
		v2.x < 0.0 ||
		v2.x > width ||
		v2.y < 0.0 ||
		v2.x > height
	{
		return;
	}

	if (v0.truncate() - v1.truncate()).perp_dot(v0.truncate() - v2.truncate()) <= 0.0
	{
		return;
	}

	rasterizer.fill_triangle(
		&[
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
			PointProjected {
				x: v2.x as Fixed16,
				y: v2.y as Fixed16,
				z: v2.z,
			},
		],
		color,
	);
}
