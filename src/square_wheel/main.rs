mod commands_queue;
mod config;
mod host;
mod host_config;
mod rasterizer;
mod renderer;
mod renderer_config;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "squar_wheel", about = "SquareWheel engine.")]
struct Opt
{
	/// Input file
	#[structopt(parse(from_os_str), short = "i")]
	input: PathBuf,
}

pub fn main()
{
	let opt = Opt::from_args();

	let mut h = host::Host::new(&opt.input);
	loop
	{
		if !h.process_frame()
		{
			break;
		}
	}
}
