#![cfg_attr(feature = "rasterizer_unchecked_div", feature(core_intrinsics))]

mod clipping_polygon;
mod commands_processor;
mod commands_queue;
mod config;
mod console;
mod draw_ordering;
mod frame_number;
mod host;
mod host_config;
mod inline_models_index;
mod light;
mod map_visibility_calculator;
mod rasterizer;
mod renderer;
mod renderer_config;
mod surfaces;
mod textures;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "squar_wheel", about = "SquareWheel engine.")]
struct Opt
{
	#[structopt(long)]
	exec: Vec<String>,
}

pub fn main()
{
	let opt = Opt::from_args();
	let mut h = host::Host::new(opt.exec);
	loop
	{
		if !h.process_frame()
		{
			break;
		}
	}
}
