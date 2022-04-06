use common::{bsp_builder, bsp_map_compact, bsp_map_save_load, map_file, map_polygonizer};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "map_compiler", about = "SquareWheel map compiler.")]
struct Opt
{
	/// Input file
	#[structopt(parse(from_os_str), short = "i", required(true))]
	input: PathBuf,

	#[structopt(parse(from_os_str), short = "o", required(true))]
	output: PathBuf,
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
		let bsp_tree = bsp_builder::build_leaf_bsp_tree(&map_polygonized);
		let mut stats = BSPStats::default();
		calculate_bsp_tree_stats_r(&bsp_tree.root, 0, &mut stats);
		stats.average_depth /= stats.num_leafs as f32;

		let mut num_portal_vertices = 0;
		for portal in &bsp_tree.portals
		{
			num_portal_vertices += portal.borrow().vertices.len();
		}
		println!("Initial polygons: {}", map_polygonized[0].polygons.len());
		println!(
			"BSP Tree stats: {:?}, average polygons in leaf: {}, average vertices in polygon: {}, portals: {}, portal \
			 vertices: {}, average vertices in portal: {}",
			stats,
			(stats.num_polygons as f32) / (stats.num_leafs as f32),
			(stats.num_polygon_vertices as f32) / (stats.num_polygons as f32),
			bsp_tree.portals.len(),
			num_portal_vertices,
			(num_portal_vertices as f32) / (bsp_tree.portals.len() as f32),
		);

		let map_compact = bsp_map_compact::convert_bsp_map_to_compact_format(&bsp_tree);
		println!(
			"Compact map nodes: {}, leafs: {}, polygons: {}, portals: {}, leafs_portals: {}, vertices: {}, textures: \
			 {}",
			map_compact.nodes.len(),
			map_compact.leafs.len(),
			map_compact.polygons.len(),
			map_compact.portals.len(),
			map_compact.leafs_portals.len(),
			map_compact.vertices.len(),
			map_compact.textures.len(),
		);

		bsp_map_save_load::save_map(&map_compact, &opt.output).unwrap();
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
	min_polygons_in_leaf: usize,
	max_polygons_in_leaf: usize,
	min_depth: usize,
	max_depth: usize,
	average_depth: f32,
}

fn calculate_bsp_tree_stats_r(node_child: &bsp_builder::BSPNodeChild, depth: usize, stats: &mut BSPStats)
{
	match node_child
	{
		bsp_builder::BSPNodeChild::NodeChild(node) =>
		{
			stats.num_nodes += 1;
			for child in &node.borrow().children
			{
				calculate_bsp_tree_stats_r(child, depth + 1, stats);
			}
		},
		bsp_builder::BSPNodeChild::LeafChild(leaf_ptr) =>
		{
			let leaf = leaf_ptr.borrow();
			stats.num_leafs += 1;

			if stats.min_depth == 0
			{
				stats.min_depth = depth;
			}
			else
			{
				stats.min_depth = std::cmp::min(stats.min_depth, depth);
			}
			stats.max_depth = std::cmp::max(stats.max_depth, depth);
			stats.average_depth += depth as f32;

			if stats.min_polygons_in_leaf == 0
			{
				stats.min_polygons_in_leaf = std::cmp::max(1, leaf.polygons.len());
			}
			else
			{
				stats.min_polygons_in_leaf = std::cmp::min(stats.min_polygons_in_leaf, leaf.polygons.len());
			}
			stats.max_polygons_in_leaf = std::cmp::max(stats.max_polygons_in_leaf, leaf.polygons.len());

			stats.num_polygons += leaf.polygons.len();
			for polygon in &leaf.polygons
			{
				stats.num_polygon_vertices += polygon.vertices.len();
			}
		},
	}
}
