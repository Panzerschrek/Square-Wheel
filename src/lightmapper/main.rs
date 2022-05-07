use common::{bsp_map_save_load, lightmaps_builder};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "lightmapper", about = "SquareWheel lightmapper.")]
struct Opt
{
	/// Input BSP file.
	#[structopt(parse(from_os_str), short = "i", required(true))]
	input: PathBuf,

	/// Output file
	#[structopt(parse(from_os_str), short = "o", required(true))]
	output: PathBuf,
}

fn main()
{
	// use "unwrap" in this function. It's fine to abort application if something is wrong.

	let opt = Opt::from_args();
	let mut map = bsp_map_save_load::load_map(&opt.input).unwrap().unwrap();
	lightmaps_builder::build_lightmaps(&mut map);
	bsp_map_save_load::save_map(&map, &opt.output).unwrap();
}
