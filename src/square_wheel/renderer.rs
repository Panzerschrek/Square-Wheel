use common::{
	bsp_map_compact, camera_controller::CameraMatrices, clipping::*, color::*, debug_rasterizer::*, fixed_math::*,
	math_types::*, plane::*, system_window,
};

pub fn draw_frame(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	camera_matrices: &CameraMatrices,
	map: &bsp_map_compact::BSPMap,
)
{
	// TODO - avoid filling background?
	draw_background(pixels);

	draw_map(pixels, surface_info, camera_matrices, map);

	// TODO - remove such temporary fuinction.
	draw_crosshair(pixels, surface_info);
}

fn draw_background(pixels: &mut [Color32])
{
	for pixel in pixels.iter_mut()
	{
		*pixel = Color32::from_rgb(32, 16, 8);
	}
}

fn draw_crosshair(pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
{
	pixels[surface_info.width / 2 + surface_info.height / 2 * surface_info.pitch] = Color32::from_rgb(255, 255, 255);
}

fn draw_map(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	camera_matrices: &CameraMatrices,
	map: &bsp_map_compact::BSPMap,
)
{
	let mut rasterizer = DebugRasterizer::new(pixels, surface_info);
	let root_node = (map.nodes.len() - 1) as u32;
	let current_sector = find_current_sector(root_node, map, &camera_matrices.planes_matrix);

	// TODO - avoid allocating this HashMap each grame.
	let mut reachable_sectors = ReachablebleSectorsMap::new();
	find_reachable_sectors_r(current_sector, map, 0, &mut reachable_sectors);

	// Draw BSP tree in order, skip unreachable leafs (sectors).
	draw_tree_r(&mut rasterizer, camera_matrices, &reachable_sectors, &map, root_node);
}

fn find_current_sector(mut index: u32, map: &bsp_map_compact::BSPMap, planes_matrix: &Mat4f) -> u32
{
	loop
	{
		if index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			return index - bsp_map_compact::FIRST_LEAF_INDEX;
		}

		let node = &map.nodes[index as usize];
		let plane_transformed = planes_matrix * node.plane.vec.extend(-node.plane.dist);
		index = if plane_transformed.w >= 0.0
		{
			node.children[0]
		}
		else
		{
			node.children[1]
		};
	}
}

type ReachablebleSectorsMap = std::collections::HashMap<u32, usize>;
fn find_reachable_sectors_r(
	sector: u32,
	map: &bsp_map_compact::BSPMap,
	depth: usize,
	reachable_sectors: &mut ReachablebleSectorsMap,
)
{
	let max_depth = 16;
	if depth > max_depth
	{
		return;
	}

	if let Some(prev_depth) = reachable_sectors.get_mut(&sector)
	{
		if *prev_depth <= depth
		{
			return;
		}
		*prev_depth = depth;
	}
	else
	{
		reachable_sectors.insert(sector, depth);
	}

	let sector_value = &map.leafs[sector as usize];
	for portal in &map.leafs_portals[(sector_value.first_leaf_portal as usize) ..
		((sector_value.first_leaf_portal + sector_value.num_leaf_portals) as usize)]
	{
		let portal_value = &map.portals[(*portal) as usize];
		let next_sector = if portal_value.leafs[0] == sector
		{
			portal_value.leafs[1]
		}
		else
		{
			portal_value.leafs[0]
		};
		find_reachable_sectors_r(next_sector, map, depth + 1, reachable_sectors);
	}
}

fn draw_tree_r(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	reachable_sectors: &ReachablebleSectorsMap,
	map: &bsp_map_compact::BSPMap,
	current_index: u32,
)
{
	if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
	{
		let sector = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
		if let Some(depth) = reachable_sectors.get(&sector)
		{
			let color = Color32::from_rgb(
				((depth * 28).min(255)) as u8,
				((depth * 24).min(255)) as u8,
				((depth * 20).min(255)) as u8,
			);

			draw_sector(rasterizer, camera_matrices, &map.leafs[sector as usize], map, color);
		}
	}
	else
	{
		let node = &map.nodes[current_index as usize];
		let plane_transformed = camera_matrices.planes_matrix * node.plane.vec.extend(-node.plane.dist);
		let mask = if plane_transformed.w >= 0.0 { 0 } else { 1 };
		for i in 0 .. 2
		{
			draw_tree_r(
				rasterizer,
				camera_matrices,
				reachable_sectors,
				map,
				node.children[(i ^ mask) as usize],
			);
		}
	}
}

fn draw_sector(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	sector: &bsp_map_compact::BSPLeaf,
	map: &bsp_map_compact::BSPMap,
	color: Color32,
)
{
	for polygon in
		&map.polygons[(sector.first_polygon as usize) .. ((sector.first_polygon + sector.num_polygons) as usize)]
	{
		draw_polygon(
			rasterizer,
			camera_matrices,
			&polygon.plane,
			&map.vertices[(polygon.first_vertex as usize) .. ((polygon.first_vertex + polygon.num_vertices) as usize)],
			&polygon.tex_coord_equation,
			color,
		);
	}
}

fn draw_polygon(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	plane: &Plane,
	vertices: &[Vec3f],
	tex_coord_equation: &[Plane; 2],
	color: Color32,
)
{
	if vertices.len() < 3
	{
		return;
	}

	let plane_transformed = camera_matrices.planes_matrix * plane.vec.extend(-plane.dist);
	// Cull back faces.
	if plane_transformed.w <= 0.0
	{
		return;
	}

	let width = rasterizer.get_width() as f32;
	let height = rasterizer.get_height() as f32;
	let half_width = width * 0.5;
	let half_height = height * 0.5;

	let plane_transformed_w = -plane_transformed.w;
	let d_inv_z_dx = plane_transformed.x / plane_transformed_w;
	let d_inv_z_dy = plane_transformed.y / plane_transformed_w;
	let depth_equation = DepthEquation {
		d_inv_z_dx,
		d_inv_z_dy,
		k: plane_transformed.z / plane_transformed_w - d_inv_z_dx * half_width - d_inv_z_dy * half_height,
	};

	const MAX_VERTICES: usize = 24;
	let mut vertex_count = vertices.len();

	// Perform initial matrix tranformation, obtain 3d vertices in camera-aligned space.
	let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	for (index, vertex) in vertices.iter().enumerate().take(MAX_VERTICES)
	{
		let vertex_transformed = camera_matrices.view_matrix * vertex.extend(1.0);
		vertices_transformed[index] = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
	}

	// Perform z_near clipping.
	let mut vertices_transformed_z_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	const Z_NEAR: f32 = 1.0;
	vertex_count = clip_3d_polygon_by_z_plane(
		&vertices_transformed[.. vertex_count],
		Z_NEAR,
		&mut vertices_transformed_z_clipped,
	);
	if vertex_count < 3
	{
		return;
	}

	// Make 2d vertices, then perform clipping in 2d space.
	// This is needed to avoid later overflows for Fixed16 vertex coords in rasterizer.
	let mut vertices_2d_0 = [Vec2f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	let mut vertices_2d_1 = [Vec2f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	for (index, vertex_transformed) in vertices_transformed_z_clipped.iter().enumerate().take(vertex_count)
	{
		vertices_2d_0[index] = vertex_transformed.truncate() / vertex_transformed.z;
	}

	// TODO - optimize this. Perform clipping, using 3 planes (for screen-space triangle), not 4 (for rectangle).
	let clip_plane_eps = -1.0;
	vertex_count = clip_2d_polygon(
		&vertices_2d_0[.. vertex_count],
		&Vec3f::new(1.0, 0.0, clip_plane_eps),
		&mut vertices_2d_1[..],
	);
	if vertex_count < 3
	{
		return;
	}
	vertex_count = clip_2d_polygon(
		&vertices_2d_1[.. vertex_count],
		&Vec3f::new(-1.0, 0.0, clip_plane_eps - width),
		&mut vertices_2d_0[..],
	);
	if vertex_count < 3
	{
		return;
	}
	vertex_count = clip_2d_polygon(
		&vertices_2d_0[.. vertex_count],
		&Vec3f::new(0.0, 1.0, clip_plane_eps),
		&mut vertices_2d_1[..],
	);
	if vertex_count < 3
	{
		return;
	}
	vertex_count = clip_2d_polygon(
		&vertices_2d_1[.. vertex_count],
		&Vec3f::new(0.0, -1.0, clip_plane_eps - height),
		&mut vertices_2d_0[..],
	);
	if vertex_count < 3
	{
		return;
	}

	// Perform f32 to Fixed16 conversion.
	let mut vertices_for_rasterizer = [PolygonPointProjected { x: 0, y: 0 }; MAX_VERTICES]; // TODO - use uninitialized memory
	for (index, vertex_2d) in vertices_2d_0.iter().enumerate().take(vertex_count)
	{
		vertices_for_rasterizer[index] = PolygonPointProjected {
			x: f32_to_fixed16(vertex_2d.x),
			y: f32_to_fixed16(vertex_2d.y),
		};
	}

	let tc_basis_transformed = [
		camera_matrices.planes_matrix * tex_coord_equation[0].vec.extend(tex_coord_equation[0].dist),
		camera_matrices.planes_matrix * tex_coord_equation[1].vec.extend(tex_coord_equation[1].dist),
	];
	let tc_equation = TexCoordEquation {
		d_tc_dx: [tc_basis_transformed[0].x, tc_basis_transformed[1].x],
		d_tc_dy: [tc_basis_transformed[0].y, tc_basis_transformed[1].y],
		d_tc_dz: [
			tc_basis_transformed[0].z -
				tc_basis_transformed[0].x * half_width -
				tc_basis_transformed[0].y * half_height,
			tc_basis_transformed[1].z -
				tc_basis_transformed[1].x * half_width -
				tc_basis_transformed[1].y * half_height,
		],
		k: [tc_basis_transformed[0].w, tc_basis_transformed[1].w],
	};

	// Perform rasterization of fully clipped polygon.
	rasterizer.fill_polygon(
		&vertices_for_rasterizer[0 .. vertex_count],
		&depth_equation,
		&tc_equation,
		color,
	);
}
