pub mod test_game;
pub mod test_game_physics;

pub use square_wheel_lib::square_wheel::*;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "square_wheel", about = "SquareWheel engine.")]
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
	let mut h = host::Host::new(
		opt.config.unwrap_or_else(|| PathBuf::from("config.json")),
		opt.exec,
		|a, b, c, d| Box::new(test_game::Game::new(a, b, c, d)),
	);
	loop
	{
		if !h.process_frame()
		{
			break;
		}
	}
}
