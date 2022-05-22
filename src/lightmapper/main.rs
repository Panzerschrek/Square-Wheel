use common::{bsp_map_save_load, lightmaps_builder, material};
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

	/// Force add ambinet light with provided power.
	#[structopt(long)]
	ambient_light: Option<f32>,

	/// Path to directory containing materials.
	#[structopt(parse(from_os_str), long)]
	materials_dir: Option<PathBuf>,
}

fn main()
{
	// use "unwrap" in this function. It's fine to abort application if something is wrong.

	let opt = Opt::from_args();

	let materials = if let Some(dir) = opt.materials_dir
	{
		material::load_materials(&dir)
	}
	else
	{
		material::MaterialsMap::new()
	};

	let mut map = bsp_map_save_load::load_map(&opt.input).unwrap().unwrap();
	lightmaps_builder::build_lightmaps(
		&lightmaps_builder::LightmappingSettings {
			sample_grid_size: opt.sample_grid_size.unwrap_or(1),
			light_scale: opt.light_scale.unwrap_or(1.0),
			ambient_light: opt.ambient_light.unwrap_or(0.0),
		},
		&materials,
		&mut map,
	);
	bsp_map_save_load::save_map(&map, &opt.output).unwrap();
}
