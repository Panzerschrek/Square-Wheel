use super::{
	bsp_builder, camera_controller::CameraMatrices, color::*, debug_rasterizer::*, fixed_math::*, map_file,
	map_polygonizer, math_types::*, system_window,
};

#[derive(Default)]
pub struct DrawOptions
{
	pub draw_raw_map: bool,
	pub draw_polygonized_map: bool,
	pub draw_bsp_map: bool,
	pub draw_map_sectors_graph: bool,
	pub draw_all_portals: bool,
	pub draw_only_first_entity: bool,
	pub draw_polygon_normals: bool,
}

pub fn draw_frame(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	draw_options: &DrawOptions,
	camera_matrices: &CameraMatrices,
	map: Option<&map_file::MapFileParsed>,
	map_polygonized: Option<&map_polygonizer::MapPolygonized>,
	map_bsp: Option<&bsp_builder::BSPTree>,
)
{
	draw_background(pixels);
	draw_map(
		pixels,
		surface_info,
		draw_options,
		camera_matrices,
		map,
		map_polygonized,
		map_bsp,
	);

	pixels[surface_info.width / 2 + surface_info.height / 2 * surface_info.pitch] = Color32::from_rgb(255, 255, 255);
}

fn draw_background(pixels: &mut [Color32])
{
	for pixel in pixels.iter_mut()
	{
		*pixel = Color32::from_rgb(32, 16, 8);
	}
}

fn draw_map(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	draw_options: &DrawOptions,
	camera_matrices: &CameraMatrices,
	map: Option<&map_file::MapFileParsed>,
	map_polygonized: Option<&map_polygonizer::MapPolygonized>,
	map_bsp: Option<&bsp_builder::BSPTree>,
)
{
	let mut rasterizer = DebugRasterizer::new(pixels, surface_info);

	if draw_options.draw_raw_map
	{
		if let Some(map_non_opt) = map
		{
			draw_map_brushes(
				&mut rasterizer,
				camera_matrices,
				map_non_opt,
				draw_options.draw_only_first_entity,
			);
		}
	}

	if draw_options.draw_polygonized_map
	{
		if let Some(map_polygonized_non_opt) = map_polygonized
		{
			draw_map_polygonized(
				&mut rasterizer,
				camera_matrices,
				map_polygonized_non_opt,
				draw_options.draw_only_first_entity,
				draw_options.draw_polygon_normals,
			);
		}
	}

	if let Some(map_bsp_non_opt) = map_bsp
	{
		if draw_options.draw_bsp_map
		{
			let mut index = 0;
			draw_map_bsp_r(
				&mut rasterizer,
				camera_matrices,
				draw_options.draw_polygon_normals,
				&map_bsp_non_opt.root,
				&mut index,
			);

			if draw_options.draw_all_portals
			{
				for (index, portal) in map_bsp_non_opt.portals.iter().enumerate()
				{
					draw_portal(
						&mut rasterizer,
						camera_matrices,
						&portal.borrow(),
						// Color32::from_rgb(255, 255, 255),
						get_pseudo_random_color(index * 4),
					);
				}
			}
		}
		if draw_options.draw_map_sectors_graph
		{
			draw_map_sectors_graph(
				&mut rasterizer,
				camera_matrices,
				draw_options.draw_polygon_normals,
				map_bsp_non_opt,
			);
		}
	}

	draw_basis(&mut rasterizer, &camera_matrices.view_matrix);
}

fn draw_map_brushes(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	map: &map_file::MapFileParsed,
	draw_only_first_entity: bool,
)
{
	for entity in map
	{
		for (brush_number, brush) in entity.brushes.iter().enumerate()
		{
			let color = get_pseudo_random_color(brush_number);

			for brush_plane in brush
			{
				draw_triangle(rasterizer, &camera_matrices.view_matrix, &brush_plane.vertices, color);
			}
		}
		if draw_only_first_entity
		{
			break;
		}
	}
}

fn draw_map_polygonized(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	map: &map_polygonizer::MapPolygonized,
	draw_only_first_entity: bool,
	draw_polygon_normals: bool,
)
{
	for entity in map
	{
		for (polygon_number, polygon) in entity.polygons.iter().enumerate()
		{
			if polygon.vertices.len() < 3
			{
				continue;
			}
			let color = get_pseudo_random_color(polygon_number);
			draw_polygon(rasterizer, camera_matrices, polygon, color, draw_polygon_normals);
		}
		if draw_only_first_entity
		{
			break;
		}
	}
}

fn draw_map_bsp_r(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	draw_polygon_normals: bool,
	bsp_node: &bsp_builder::BSPNodeChild,
	index: &mut usize,
)
{
	match bsp_node
	{
		bsp_builder::BSPNodeChild::NodeChild(node_ptr) =>
		{
			let node = node_ptr.borrow();
			let plane_transformed = camera_matrices.planes_matrix * node.plane.vec.extend(-node.plane.dist);
			if plane_transformed.w >= 0.0
			{
				draw_map_bsp_r(
					rasterizer,
					camera_matrices,
					draw_polygon_normals,
					&node.children[0],
					index,
				);
				draw_map_bsp_r(
					rasterizer,
					camera_matrices,
					draw_polygon_normals,
					&node.children[1],
					index,
				);
			}
			else
			{
				draw_map_bsp_r(
					rasterizer,
					camera_matrices,
					draw_polygon_normals,
					&node.children[1],
					index,
				);
				draw_map_bsp_r(
					rasterizer,
					camera_matrices,
					draw_polygon_normals,
					&node.children[0],
					index,
				);
			}
		},
		bsp_builder::BSPNodeChild::LeafChild(leaf) =>
		{
			draw_map_bsp_leaf(
				rasterizer,
				camera_matrices,
				draw_polygon_normals,
				&leaf.borrow(),
				*index,
			);
			*index += 1;
		},
	}
}

fn draw_map_bsp_leaf(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	draw_polygon_normals: bool,
	bsp_leaf: &bsp_builder::BSPLeaf,
	index: usize,
)
{
	// let leaf_ptr_as_int = bsp_leaf as *const bsp_builder::BSPLeaf as usize;
	// let mut color = get_pseudo_random_color(leaf_ptr_as_int / std::mem::size_of::<bsp_builder::BSPLeaf>());
	// let color = Color32::from_rgb(
	// (index * 3 % 511 - 255) as u8,
	// (index * 5 % 511 - 255) as u8,
	// (index * 7 % 511 - 255) as u8 );
	let mut color = Color32::from_rgb(
		((index * 28).min(255)) as u8,
		((index * 24).min(255)) as u8,
		((index * 24).min(255)) as u8,
	);

	if index == 0
	{
		color = Color32::from_rgb(8, 8, 8);
	}

	for polygon in &bsp_leaf.polygons
	{
		draw_polygon(rasterizer, camera_matrices, polygon, color, draw_polygon_normals);
	}

	if index == 0
	{
		for portal_ptr_weak in &bsp_leaf.portals
		{
			let portal_ptr = portal_ptr_weak.upgrade().unwrap();
			draw_portal(
				rasterizer,
				camera_matrices,
				&portal_ptr.borrow(),
				Color32::from_rgb(255, 255, 255),
			);
		}
	}
}

fn draw_map_sectors_graph(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	draw_polygon_normals: bool,
	bsp_tree: &bsp_builder::BSPTree,
)
{
	let current_sector_ptr = find_current_sector(&bsp_tree.root, &camera_matrices.planes_matrix);

	let mut accessible_sectors = AccessisbleSectorsMap::new();
	find_accessible_sectors_r(&current_sector_ptr, 0, &mut accessible_sectors);

	for (_raw_ptr, (sector, depth)) in accessible_sectors
	{
		draw_map_bsp_leaf(
			rasterizer,
			camera_matrices,
			draw_polygon_normals,
			&sector.borrow(),
			depth,
		);
	}
}

fn find_current_sector(
	bsp_node_child: &bsp_builder::BSPNodeChild,
	planes_matrix: &Mat4f,
) -> std::rc::Rc<std::cell::RefCell<bsp_builder::BSPLeaf>>
{
	loop
	{
		match bsp_node_child
		{
			bsp_builder::BSPNodeChild::NodeChild(node_ptr) =>
			{
				let node = node_ptr.borrow();
				let plane_transformed = planes_matrix * node.plane.vec.extend(-node.plane.dist);
				if plane_transformed.w >= 0.0
				{
					return find_current_sector(&node.children[0], planes_matrix);
				}
				else
				{
					return find_current_sector(&node.children[1], planes_matrix);
				}
			},
			bsp_builder::BSPNodeChild::LeafChild(leaf_ptr) =>
			{
				return leaf_ptr.clone();
			},
		}
	}
}

type AccessisbleSectorsMap = std::collections::HashMap<
	*const bsp_builder::BSPLeaf,
	(std::rc::Rc<std::cell::RefCell<bsp_builder::BSPLeaf>>, usize),
>;
fn find_accessible_sectors_r(
	sector_ptr: &std::rc::Rc<std::cell::RefCell<bsp_builder::BSPLeaf>>,
	depth: usize,
	accessible_sectors: &mut AccessisbleSectorsMap,
)
{
	let max_depth = 16;
	if depth > max_depth
	{
		return;
	}

	let sector = sector_ptr.borrow();
	let sector_raw_ptr = (&*sector) as *const bsp_builder::BSPLeaf;
	if accessible_sectors.contains_key(&sector_raw_ptr)
	{
		let prev_depth = &mut accessible_sectors.get_mut(&sector_raw_ptr).unwrap().1;
		if *prev_depth <= depth
		{
			return;
		}
		*prev_depth = depth;
	}
	else
	{
		accessible_sectors.insert(sector_raw_ptr, (sector_ptr.clone(), depth));
	}

	for portal_ptr_weak in &sector.portals
	{
		let protal_ptr = portal_ptr_weak.upgrade().unwrap();
		let portal = protal_ptr.borrow();
		let leaf_front = portal.leaf_front.borrow();
		let linked_sector_ptr = if (&*leaf_front) as *const bsp_builder::BSPLeaf == sector_raw_ptr
		{
			&portal.leaf_back
		}
		else
		{
			&portal.leaf_front
		};
		find_accessible_sectors_r(linked_sector_ptr, depth + 1, accessible_sectors);
	}
}

fn draw_polygon(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	polygon: &map_polygonizer::Polygon,
	color: Color32,
	draw_normal: bool,
)
{
	if polygon.vertices.len() < 3
	{
		return;
	}

	let plane_transformed = camera_matrices.planes_matrix * polygon.plane.vec.extend(-polygon.plane.dist);
	// Cull back faces.
	if plane_transformed.w <= 0.0
	{
		return;
	}

	let width = rasterizer.get_width() as f32;
	let height = rasterizer.get_height() as f32;
	let half_width = width * 0.5;
	let half_height = height * 0.5;

	let d_inv_z_dx = plane_transformed.x / plane_transformed.w;
	let d_inv_z_dy = plane_transformed.y / plane_transformed.w;
	let depth_equation = DepthEquation {
		d_inv_z_dx,
		d_inv_z_dy,
		k: plane_transformed.z / plane_transformed.w - d_inv_z_dx * half_width - d_inv_z_dy * half_height,
	};

	const MAX_VERTICES: usize = 24;
	let mut vertex_count = polygon.vertices.len();

	// Perform initial matrix tranformation, obtain 3d vertices in camera-aligned space.
	let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
	for (index, vertex) in polygon.vertices.iter().enumerate().take(MAX_VERTICES)
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
		camera_matrices.planes_matrix *
			polygon.texture_info.tex_coord_equation[0]
				.vec
				.extend(polygon.texture_info.tex_coord_equation[0].dist),
		camera_matrices.planes_matrix *
			polygon.texture_info.tex_coord_equation[1]
				.vec
				.extend(polygon.texture_info.tex_coord_equation[1].dist),
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
		k: [-tc_basis_transformed[0].w, -tc_basis_transformed[1].w],
	};

	// Perform rasterization of fully clipped polygon.
	rasterizer.fill_polygon(
		&vertices_for_rasterizer[0 .. vertex_count],
		&depth_equation,
		&tc_equation,
		color,
	);

	if draw_normal
	{
		let mut vertices_sum = Vec3f::zero();
		for v in &polygon.vertices
		{
			vertices_sum += *v;
		}
		let center = vertices_sum / (polygon.vertices.len() as f32);
		let line = (
			center,
			center + polygon.plane.vec * (16.0 / polygon.plane.vec.magnitude()),
			color.get_inverted(),
		);
		draw_line(rasterizer, &camera_matrices.view_matrix, &line);
	}
}

fn clip_3d_polygon_by_z_plane(polygon: &[Vec3f], z_dist: f32, out_polygon: &mut [Vec3f]) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut out_vertex_count = 0;
	for v in polygon
	{
		if v.z > z_dist
		{
			if prev_v.z < z_dist
			{
				out_polygon[out_vertex_count] = get_line_z_intersection(prev_v, v, z_dist);
				out_vertex_count += 1;
				if out_vertex_count == out_polygon.len()
				{
					break;
				}
			}
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if v.z == z_dist
		{
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if prev_v.z > z_dist
		{
			out_polygon[out_vertex_count] = get_line_z_intersection(prev_v, v, z_dist);
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}

		prev_v = v;
	}

	out_vertex_count
}

fn get_line_z_intersection(v0: &Vec3f, v1: &Vec3f, z: f32) -> Vec3f
{
	let dist0 = v0.z - z;
	let dist1 = v1.z - z;
	let dist_sum = v1.z - v0.z;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
}

fn clip_2d_polygon(polygon: &[Vec2f], clip_line_equation: &Vec3f, out_polygon: &mut [Vec2f]) -> usize
{
	let mut prev_v = polygon.last().unwrap();
	let mut prev_dist = prev_v.dot(clip_line_equation.truncate()) - clip_line_equation.z;
	let mut out_vertex_count = 0;
	for v in polygon
	{
		let dist = v.dot(clip_line_equation.truncate()) - clip_line_equation.z;
		if dist > 0.0
		{
			if prev_dist < 0.0
			{
				out_polygon[out_vertex_count] = get_line_line_intersection(prev_v, v, clip_line_equation);
				out_vertex_count += 1;
				if out_vertex_count == out_polygon.len()
				{
					break;
				}
			}
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if dist == 0.0
		{
			out_polygon[out_vertex_count] = *v;
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}
		else if prev_dist > 0.0
		{
			out_polygon[out_vertex_count] = get_line_line_intersection(prev_v, v, clip_line_equation);
			out_vertex_count += 1;
			if out_vertex_count == out_polygon.len()
			{
				break;
			}
		}

		prev_v = v;
		prev_dist = dist;
	}

	out_vertex_count
}

fn get_line_line_intersection(v0: &Vec2f, v1: &Vec2f, line: &Vec3f) -> Vec2f
{
	let dist0 = v0.dot(line.truncate()) - line.z;
	let dist1 = v1.dot(line.truncate()) - line.z;
	let dist_sum = dist1 - dist0;
	let k0 = dist0 / dist_sum;
	let k1 = dist1 / dist_sum;
	v0 * k1 - v1 * k0
}

fn draw_basis(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f)
{
	let basis_lines = [
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(64.0, 0.0, 0.0),
			Color32::from_rgb(255, 0, 0),
		),
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(0.0, 64.0, 0.0),
			Color32::from_rgb(0, 255, 0),
		),
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(0.0, 0.0, 64.0),
			Color32::from_rgb(0, 0, 255),
		),
	];

	for line in &basis_lines
	{
		draw_line(rasterizer, &transform_matrix, line);
	}
}

fn draw_portal(
	rasterizer: &mut DebugRasterizer,
	camera_matrices: &CameraMatrices,
	portal: &bsp_builder::LeafsPortal,
	color: Color32,
)
{
	let shift_vec = portal.plane.vec * (2.0 / portal.plane.vec.magnitude());
	for v0 in &portal.vertices
	{
		let v0_sifted_plus = v0 + shift_vec;
		let v0_hifted_minus = v0 - shift_vec;
		for v1 in &portal.vertices
		{
			let v1_sifted_plus = v1 + shift_vec;
			let v1_hifted_minus = v1 - shift_vec;
			if v0 != v1
			{
				draw_line(
					rasterizer,
					&camera_matrices.view_matrix,
					&(v0_sifted_plus, v1_sifted_plus, color),
				);
				draw_line(
					rasterizer,
					&camera_matrices.view_matrix,
					&(v0_hifted_minus, v1_hifted_minus, color),
				);
			}
		}

		draw_line(
			rasterizer,
			&camera_matrices.view_matrix,
			&(v0_sifted_plus, v0_hifted_minus, color),
		);
	}
}

fn get_pseudo_random_color(num: usize) -> Color32
{
	Color32::from_rgb(
		((num * 11) & 255) as u8,
		((num * 17) & 255) as u8,
		((num * 23) & 255) as u8,
	)
}

type WorldLine = (Vec3f, Vec3f, Color32);

fn draw_line(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, line: &WorldLine)
{
	let v0 = transform_matrix * line.0.extend(1.0);
	let v1 = transform_matrix * line.1.extend(1.0);

	let mut v0 = Vec3f::new(v0.x, v0.y, v0.w);
	let mut v1 = Vec3f::new(v1.x, v1.y, v1.w);

	// Perform z_near clipping.
	const Z_NEAR: f32 = 1.0;
	if v0.z < Z_NEAR && v1.z < Z_NEAR
	{
		return;
	}
	if v0.z >= Z_NEAR && v1.z >= Z_NEAR
	{
	}
	else if v0.z > Z_NEAR && v1.z <= Z_NEAR
	{
		v1 = get_line_z_intersection(&v0, &v1, Z_NEAR);
	}
	else
	{
		v0 = get_line_z_intersection(&v0, &v1, Z_NEAR);
	}

	let z0 = v0.z;
	let z1 = v1.z;
	let v0 = v0.truncate() / z0;
	let v1 = v1.truncate() / z1;

	// TODO - perform proper clipping.
	// Now - just prevent Fixed16 overflow.
	const MAX_COORD: f32 = 8192.0;
	if v0.x < -MAX_COORD ||
		v0.x > MAX_COORD ||
		v0.y < -MAX_COORD ||
		v0.y > MAX_COORD ||
		v1.x < -MAX_COORD ||
		v1.x > MAX_COORD ||
		v1.y < -MAX_COORD ||
		v1.y > MAX_COORD
	{
		return;
	}

	rasterizer.draw_line(
		PointProjectedWithZ {
			x: f32_to_fixed16(v0.x),
			y: f32_to_fixed16(v0.y),
			z: -1.0 / z0,
		},
		PointProjectedWithZ {
			x: f32_to_fixed16(v1.x),
			y: f32_to_fixed16(v1.y),
			z: -1.0 / z1,
		},
		line.2,
	);
}

fn draw_triangle(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, vertices: &[Vec3f; 3], color: Color32)
{
	// TODO - perform scaling to "Fixed16" via prescaled matrix.
	let width = rasterizer.get_width() as f32;
	let height = rasterizer.get_height() as f32;

	let v0 = transform_matrix * vertices[0].extend(1.0);
	let v1 = transform_matrix * vertices[1].extend(1.0);
	let v2 = transform_matrix * vertices[2].extend(1.0);

	// TODO - perform proper clipping
	if v0.w <= 0.1 || v1.w <= 0.1 || v2.w <= 0.1
	{
		return;
	}
	let v0 = v0.truncate() / v0.w;
	let v1 = v1.truncate() / v1.w;
	let v2 = v2.truncate() / v2.w;

	if v0.x < 0.0 ||
		v0.x > width ||
		v0.y < 0.0 ||
		v0.y > height ||
		v1.x < 0.0 ||
		v1.x > width ||
		v1.y < 0.0 ||
		v1.y > height ||
		v2.x < 0.0 ||
		v2.x > width ||
		v2.y < 0.0 ||
		v2.y > height
	{
		return;
	}

	if (v0.truncate() - v1.truncate()).perp_dot(v0.truncate() - v2.truncate()) <= 0.0
	{
		return;
	}

	rasterizer.fill_triangle(
		&[
			PointProjectedWithZ {
				x: f32_to_fixed16(v0.x),
				y: f32_to_fixed16(v0.y),
				z: v0.z,
			},
			PointProjectedWithZ {
				x: f32_to_fixed16(v1.x),
				y: f32_to_fixed16(v1.y),
				z: v1.z,
			},
			PointProjectedWithZ {
				x: f32_to_fixed16(v2.x),
				y: f32_to_fixed16(v2.y),
				z: v2.z,
			},
		],
		color,
	);
}
