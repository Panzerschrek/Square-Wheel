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

	/// Size of sample grid. For sample-grid-size=N NxN calculations for each lightmap texel will be performed.
	#[structopt(long)]
	sample_grid_size: Option<u32>,

	/// Set scale for all light sources. Default scale is 1.
	#[structopt(long)]
	light_scale: Option<f32>,
}

fn main()
{
	// use "unwrap" in this function. It's fine to abort application if something is wrong.

	let opt = Opt::from_args();
	let mut map = bsp_map_save_load::load_map(&opt.input).unwrap().unwrap();
	lightmaps_builder::build_lightmaps(
		&lightmaps_builder::LightmappingSettings {
			sample_grid_size: opt.sample_grid_size.unwrap_or(1),
			light_scale: opt.light_scale.unwrap_or(1.0),
		},
		&mut map,
	);
	bsp_map_save_load::save_map(&map, &opt.output).unwrap();
}
