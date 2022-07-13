#![cfg_attr(feature = "rasterizer_unchecked_div", feature(core_intrinsics))]

mod abstract_color;
mod commands_processor;
mod commands_queue;
mod config;
mod console;
mod debug_stats_printer;
mod depth_renderer;
mod draw_ordering;
mod fast_math;
mod frame_info;
mod frame_number;
mod host;
mod host_config;
mod inline_models_index;
mod light;
mod map_materials_processor;
mod map_visibility_calculator;
mod performance_counter;
mod postprocessor;
mod postprocessor_config;
mod rasterizer;
mod rect_splitting;
mod renderer;
mod renderer_config;
mod resources_manager;
mod shadow_map;
mod surfaces;
mod test_game;
mod text_printer;
mod textures;
mod ticks_counter;
mod triangle_model;
mod triangle_model_md3;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "squar_wheel", about = "SquareWheel engine.")]
struct Opt
{
	/// Optional command to execute on start.
	#[structopt(long)]
	exec: Vec<String>,

	/// Optional path to config file. If empty - default path will be used.
	#[structopt(long, parse(from_os_str))]
	config: Option<PathBuf>,
}

pub fn main()
{
	let opt = Opt::from_args();
	let mut h = host::Host::new(opt.config.unwrap_or_else(|| PathBuf::from("config.json")), opt.exec);
	loop
	{
		if !h.process_frame()
		{
			break;
		}
	}
}
