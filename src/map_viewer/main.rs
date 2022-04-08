use common::{
	bsp_builder, bsp_map_compact, bsp_map_save_load, debug_renderer, map_file, map_polygonizer, system_window,
};
use sdl2::{event::Event, keyboard::Keycode};
use std::{path::PathBuf, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "map_viewer", about = "SquareWheel map viewer tool.")]
struct Opt
{
	/// Input file
	#[structopt(parse(from_os_str), short = "i")]
	input: Option<PathBuf>,

	/// Input file (compiled)
	#[structopt(parse(from_os_str), short = "I")]
	input_compiled: Option<PathBuf>,

	#[structopt(long)]
	draw_raw_map: bool,

	#[structopt(long)]
	draw_polygonized_map: bool,

	#[structopt(long)]
	draw_bsp_map: bool,

	#[structopt(long)]
	draw_bsp_map_compact: bool,

	#[structopt(long)]
	draw_map_sectors_graph: bool,

	#[structopt(long)]
	draw_map_sectors_graph_compact: bool,

	#[structopt(long)]
	draw_all_portals: bool,

	#[structopt(long)]
	draw_polygon_normals: bool,
}

pub fn main()
{
	let opt = Opt::from_args();

	let mut window = system_window::SystemWindow::new();
	let mut camera_controller = common::camera_controller::CameraController::new();

	let mut map_file_parsed_opt = None;
	let mut map_polygonized_opt = None;
	let mut map_bsp_tree_opt = None;
	let mut map_bsp_compact_opt = None;
	if let Some(path) = &opt.input
	{
		let file_contents_str = std::fs::read_to_string(path).unwrap();
		map_file_parsed_opt = map_file::parse_map_file_content(&file_contents_str).ok();
		if opt.draw_polygonized_map ||
			opt.draw_bsp_map ||
			opt.draw_bsp_map_compact ||
			opt.draw_map_sectors_graph ||
			opt.draw_map_sectors_graph_compact
		{
			if let Some(map_file) = &map_file_parsed_opt
			{
				let map_polygonized = map_polygonizer::polygonize_map(map_file);
				if opt.draw_bsp_map ||
					opt.draw_bsp_map_compact ||
					opt.draw_map_sectors_graph ||
					opt.draw_map_sectors_graph_compact
				{
					map_bsp_tree_opt = Some(bsp_builder::build_leaf_bsp_tree(&map_polygonized));
					if opt.draw_bsp_map_compact || opt.draw_map_sectors_graph_compact
					{
						map_bsp_compact_opt = Some(bsp_map_compact::convert_bsp_map_to_compact_format(
							map_bsp_tree_opt.as_ref().unwrap(),
							&map_polygonized[1 ..],
						));
					}
				}
				map_polygonized_opt = Some(map_polygonized);
			}
		}
	}
	if let Some(path) = &opt.input_compiled
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

		window.end_frame(|pixels, surface_info| {
			debug_renderer::draw_frame(
				pixels,
				surface_info,
				&debug_renderer::DrawOptions {
					draw_raw_map: opt.draw_raw_map,
					draw_polygonized_map: opt.draw_polygonized_map,
					draw_bsp_map: opt.draw_bsp_map,
					draw_bsp_map_compact: opt.draw_bsp_map_compact,
					draw_map_sectors_graph: opt.draw_map_sectors_graph,
					draw_map_sectors_graph_compact: opt.draw_map_sectors_graph_compact,
					draw_only_first_entity: false,
					draw_polygon_normals: opt.draw_polygon_normals,
					draw_all_portals: opt.draw_all_portals,
				},
				&camera_controller.build_view_matrix(surface_info.width as f32, surface_info.height as f32),
				map_file_parsed_opt.as_ref(),
				map_polygonized_opt.as_ref(),
				map_bsp_tree_opt.as_ref(),
				map_bsp_compact_opt.as_ref(),
			)
		});

		let frame_end_time = std::time::Instant::now();
		let frame_time_s = (frame_end_time - prev_time).as_secs_f32();
		let min_frame_time = 0.01;
		if frame_time_s < min_frame_time
		{
			std::thread::sleep(Duration::from_secs_f32(min_frame_time - frame_time_s));
		}
	}
}
