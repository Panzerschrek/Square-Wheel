use square_wheel_lib::common::{bsp_map_save_load, image, lightmaps_builder, material};
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

	/// Path to directory containing textures.
	#[structopt(parse(from_os_str), long)]
	textures_dir: Option<PathBuf>,

	/// Disable export of primary light.
	#[structopt(long)]
	no_primary_light: bool,

	/// Disable export of secondary light.
	#[structopt(long)]
	no_secondary_light: bool,

	/// Disable calculation of light from emissive surfaces.
	#[structopt(long)]
	no_emissive_surfaces_light: bool,

	/// Disable calculation of directional lightmaps.
	#[structopt(long)]
	no_directional_lightmap: bool,

	/// Number of light passes.
	#[structopt(long)]
	num_passes: Option<u32>,

	/// Number of threads. If empty - all available CPU cores will be used.
	#[structopt(long)]
	num_threads: Option<u32>,

	/// Width (X and Y dimensions) of light grid cell.
	#[structopt(long)]
	light_grid_cell_width: Option<f32>,

	/// Height (Z dimenision) of light grid cell.
	#[structopt(long)]
	light_grid_cell_height: Option<f32>,
}

fn main()
{
	// use "unwrap" in this function. It's fine to abort application if something is wrong.

	let opt = Opt::from_args();

	// Setup global thread pool.
	{
		let num_threads = if let Some(n) = opt.num_threads
		{
			if n == 0
			{
				num_cpus::get()
			}
			else
			{
				n.min(64) as usize
			}
		}
		else
		{
			num_cpus::get()
		};
		rayon::ThreadPoolBuilder::new()
			.num_threads(num_threads)
			.stack_size(1024 * 1024 * 2)
			.build_global()
			.unwrap();
	}

	let materials = if let Some(dir) = &opt.materials_dir
	{
		material::load_materials(dir)
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
			save_primary_light: !opt.no_primary_light,
			save_secondary_light: !opt.no_secondary_light,
			build_emissive_surfaces_light: !opt.no_emissive_surfaces_light,
			build_directional_lightmap: !opt.no_directional_lightmap,
			num_passes: opt.num_passes.unwrap_or(1),
			light_grid_cell_width: opt.light_grid_cell_width.unwrap_or(64.0),
			light_grid_cell_height: opt.light_grid_cell_height.unwrap_or(64.0),
		},
		&materials,
		&mut map,
		|texture| load_texture_image(&materials, opt.textures_dir.as_ref(), texture),
	);
	bsp_map_save_load::save_map(&map, &opt.output).unwrap();
}

fn load_texture_image(
	materials: &material::MaterialsMap,
	textures_dir: Option<&PathBuf>,
	texture: &str,
) -> Option<image::Image>
{
	let mut file_name = texture.to_string();
	if let Some(material) = materials.get(texture)
	{
		if let Some(diffuse) = &material.diffuse
		{
			file_name = diffuse.clone();
		}
	}

	let file_path = if let Some(dir) = textures_dir
	{
		let mut p = dir.clone();
		p.push(file_name);
		p
	}
	else
	{
		PathBuf::from(file_name)
	};

	image::load(&file_path)
}
