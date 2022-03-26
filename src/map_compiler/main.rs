use common::{bsp_builder, map_file, map_polygonizer};
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
		let map_polygonized = map_polygonizer::polygonize_map(&map_file_parsed);
		let bsp_tree = bsp_builder::build_leaf_bsp_tree(&map_polygonized[0]);
		let mut stats = BSPStats::default();
		calculate_bsp_tree_stats_r(&bsp_tree, &mut stats);
		println!("Initial polygons: {}", map_polygonized[0].polygons.len());
		println!("BSP Tree stats: {:?}", stats);
	}
	else
	{
		println!("Failed to parse map file: {:?}", file_content);
	}
}

#[derive(Debug, Default)]
struct BSPStats
{
	num_nodes: usize,
	num_leafs: usize,
	num_polygons: usize,
	num_polygon_vertices: usize,
}

fn calculate_bsp_tree_stats_r(node_child: &bsp_builder::BSPNodeChild, stats: &mut BSPStats)
{
	match node_child
	{
		bsp_builder::BSPNodeChild::NodeChild(node) =>
		{
			stats.num_nodes += 1;
			for child in &node.children
			{
				calculate_bsp_tree_stats_r(child, stats);
			}
		},
		bsp_builder::BSPNodeChild::LeafChild(leaf) =>
		{
			stats.num_leafs += 1;
			stats.num_polygons += leaf.polygons.len();
			for polygon in &leaf.polygons
			{
				stats.num_polygon_vertices += polygon.vertices.len();
			}
		},
	}
}
