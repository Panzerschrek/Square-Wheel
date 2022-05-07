use common::{bsp_map_compact, bsp_map_save_load};
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
	let map = bsp_map_save_load::load_map(&opt.input).unwrap().unwrap();

	for entity in &map.entities
	{
		println!("{{");
		for key_value_pair in &map.key_value_pairs[(entity.first_key_value_pair as usize) ..
			((entity.first_key_value_pair + entity.num_key_value_pairs) as usize)]
		{
			println!(
				"{}: {}",
				get_map_string(key_value_pair.key, &map),
				get_map_string(key_value_pair.value, &map)
			);
		}
		println!("}}");
	}
}

fn get_map_string(s: bsp_map_compact::StringRef, map: &bsp_map_compact::BSPMap) -> &str
{
	std::str::from_utf8(&map.strings_data[(s.offset as usize) .. ((s.offset + s.size) as usize)]).unwrap_or("")
}
