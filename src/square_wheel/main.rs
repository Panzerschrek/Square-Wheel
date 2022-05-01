#![cfg_attr(feature = "rasterizer_unchecked_div", feature(core_intrinsics))]

mod clipping_polygon;
mod commands_processor;
mod commands_queue;
mod config;
mod console;
mod draw_ordering;
mod host;
mod host_config;
mod inline_models_index;
mod rasterizer;
mod renderer;
mod renderer_config;
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
