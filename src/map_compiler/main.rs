use square_wheel_lib::common::{
	bsp_builder, bsp_map_compact, bsp_map_compact_conversion, bsp_map_save_load, image, lightmaps_builder, map_csg,
	map_file_q1, map_file_q4, map_polygonizer, material,
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "map_compiler", about = "SquareWheel map compiler.")]
struct Opt
{
	/// Input file (.map)
	#[structopt(parse(from_os_str), short = "i", required(true))]
	input: PathBuf,

	/// Output file
	#[structopt(parse(from_os_str), short = "o", required(true))]
	output: PathBuf,

	/// Perform CSG for initial brushes. This may produce more optimal map, or may not.
	/// Try to compile with/without this flag in order to know results.
	#[structopt(long)]
	perform_csg: bool,

	/// Print stats of input/result map
	#[structopt(long)]
	print_stats: bool,

	/// Print stats of submodels
	#[structopt(long)]
	print_submodels_stats: bool,

	/// Input map file format.
	#[structopt(long)]
	input_format: Option<String>,

	/// Path to directory containing materials.
	#[structopt(parse(from_os_str), long)]
	materials_dir: Option<PathBuf>,

	/// Path to directory containing textures.
	#[structopt(parse(from_os_str), long)]
	textures_dir: Option<PathBuf>,
}

fn main()
{
	// use "unwrap" in this function. It's fine to abort application if something is wrong.

	let opt = Opt::from_args();

	let materials = if let Some(dir) = opt.materials_dir
	{
		println!("Loading materials from {:?}", dir);
		material::load_materials(&dir)
	}
	else
	{
		material::MaterialsMap::new()
	};

	let textures_dir = opt.textures_dir;

	println!("Reading map file {:?}", opt.input);
	let file_contents_str = std::fs::read_to_string(opt.input).unwrap();

	println!("Polygonizing brushes");
	let map_polygonized = match opt.input_format.unwrap_or_default().as_str()
	{
		"quake4" =>
		{
			let map_file_parsed = map_file_q4::parse_map_file_content(&file_contents_str).unwrap();

			let mut textures_size_cache = std::collections::HashMap::<String, [u32; 2]>::new();

			map_polygonizer::polygonize_map_q4(&map_file_parsed, &mut |texture| {
				if let Some(value) = textures_size_cache.get(texture)
				{
					return *value;
				}
				let value = get_material_texture_size(&materials, textures_dir.as_ref(), texture);
				textures_size_cache.insert(texture.to_string(), value);
				value
			})
		},
		"" | "quake" | _ =>
		{
			let map_file_parsed = map_file_q1::parse_map_file_content(&file_contents_str).unwrap();
			map_polygonizer::polygonize_map(&map_file_parsed)
		},
	};

	let map_csg_processed = if opt.perform_csg
	{
		println!("Doing CSG for brushes");
		map_csg::perform_csg_for_map_brushes(&map_polygonized, &materials)
	}
	else
	{
		println!("Skipping CSG step");
		map_csg::perform_no_csg_for_map_brushes(&map_polygonized)
	};

	println!("Building BSP tree");
	let bsp_tree = bsp_builder::build_leaf_bsp_tree(&map_csg_processed, &materials);

	println!("Building submodels BSP trees");
	let submodels_bsp_trees = map_csg_processed[1 ..]
		.iter()
		.map(|s| bsp_builder::build_submodel_bsp_tree(s, &materials))
		.collect::<Vec<_>>();

	println!("Converting BSP tree in compact format");
	let mut map_compact = bsp_map_compact_conversion::convert_bsp_map_to_compact_format(
		&bsp_tree,
		&map_csg_processed,
		&submodels_bsp_trees,
		&materials,
	);

	println!("Creating dummy lightmaps");
	lightmaps_builder::build_dummy_lightmaps(&materials, &mut map_compact);

	println!("Saving BSP map {:?}", opt.output);
	bsp_map_save_load::save_map(&map_compact, &opt.output).unwrap();

	if opt.print_stats
	{
		print_stats(&map_polygonized, &map_csg_processed, &bsp_tree, &map_compact);
	}

	if opt.print_submodels_stats
	{
		for (index, submodels_bsp_tree) in submodels_bsp_trees.iter().enumerate()
		{
			let mut stats = SubmodelBSPStats::default();
			calculate_submodel_bsp_tree_stats_r(submodels_bsp_tree, 0, &mut stats);
			stats.average_depth /= stats.num_nodes as f32;

			println!(
				"Submodel {} BSP Tree stats: {:?}, average polygons in node: {}, average vertices in polygon: {}",
				index + 1,
				stats,
				(stats.num_polygons as f32) / (stats.num_nodes as f32),
				(stats.num_polygon_vertices as f32) / (stats.num_polygons.max(1) as f32),
			);
		}
	}
}

fn get_material_texture_size(
	materials: &material::MaterialsMap,
	textures_dir: Option<&PathBuf>,
	texture: &str,
) -> [u32; 2]
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

	if let Some(image) = image::load(&file_path)
	{
		return image.size;
	}

	// TODO - for plain texture try to use different file extensions.

	println!("Can't find image for material {}, using path {:?}", texture, file_path);
	[128, 128]
}

fn print_stats(
	map_polygonized: &map_polygonizer::MapPolygonized,
	map_csg_processed: &map_csg::MapCSGProcessed,
	bsp_tree: &bsp_builder::BSPTree,
	map_compact: &bsp_map_compact::BSPMap,
)
{
	let mut stats = BSPStats::default();
	calculate_bsp_tree_stats_r(&bsp_tree.root, 0, &mut stats);
	stats.average_depth /= stats.num_leafs as f32;

	let mut num_portal_vertices = 0;
	for portal in &bsp_tree.portals
	{
		num_portal_vertices += portal.borrow().vertices.len();
	}

	let mut initial_polygons = 0;
	for brush in &map_polygonized[0].brushes
	{
		initial_polygons += brush.len();
	}

	println!("Initial polygons: {}", initial_polygons);
	println!("Polygons after CSG: {}", map_csg_processed[0].polygons.len());
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

	println!(
		"Compact map nodes: {}, leafs: {}, polygons: {}, portals: {}, leafs_portals: {}, vertices: {}, textures: {}, \
		 submodels: {}, sumbodels nodes: {}, lightmap texels: {}",
		map_compact.nodes.len(),
		map_compact.leafs.len(),
		map_compact.polygons.len(),
		map_compact.portals.len(),
		map_compact.leafs_portals.len(),
		map_compact.vertices.len(),
		map_compact.textures.len(),
		map_compact.submodels.len(),
		map_compact.submodels_bsp_nodes.len(),
		map_compact.lightmaps_data.len()
	);
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

#[derive(Debug, Default)]
struct SubmodelBSPStats
{
	num_nodes: usize,
	num_polygons: usize,
	num_polygon_vertices: usize,
	min_polygons_in_node: usize,
	max_polygons_in_node: usize,
	min_depth: usize,
	max_depth: usize,
	average_depth: f32,
}

fn calculate_submodel_bsp_tree_stats_r(node: &bsp_builder::SubmodelBSPNode, depth: usize, stats: &mut SubmodelBSPStats)
{
	stats.num_nodes += 1;
	for child in node.children.iter().flatten()
	{
		calculate_submodel_bsp_tree_stats_r(child, depth + 1, stats);
	}

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

	if stats.min_polygons_in_node == 0
	{
		stats.min_polygons_in_node = std::cmp::max(1, node.polygons.len());
	}
	else
	{
		stats.min_polygons_in_node = std::cmp::min(stats.min_polygons_in_node, node.polygons.len());
	}
	stats.max_polygons_in_node = std::cmp::max(stats.max_polygons_in_node, node.polygons.len());

	stats.num_polygons += node.polygons.len();
	for polygon in &node.polygons
	{
		stats.num_polygon_vertices += polygon.vertices.len();
	}
}
