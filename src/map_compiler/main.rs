use common::{map_file, map_polygonizer};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "map_compiler", about = "SquareWheel map compiler.")]
struct Opt
{
	/// Input file
	#[structopt(parse(from_os_str), short = "i", required(true))]
	input: PathBuf,
}

fn main()
{
	let opt = Opt::from_args();
	println!("Input file: {:?}", opt.input);

	let file_contents_str = std::fs::read_to_string(opt.input).unwrap();
	let file_content = map_file::parse_map_file_content(&file_contents_str);
	if let Ok(map_file_parsed) = &file_content
	{
		map_polygonizer::polygonize_map(&map_file_parsed);
	}
	else
	{
		println!("Failed to parse map file: {:?}", file_content);
	}
}
