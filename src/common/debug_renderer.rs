use super::{
	bsp_builder, color::*, debug_rasterizer::*, fixed_math::*, map_file, map_polygonizer, math_types::*, system_window,
};

#[derive(Default)]
pub struct DrawOptions
{
	pub draw_raw_map: bool,
	pub draw_polygonized_map: bool,
	pub draw_bsp_map: bool,
	pub draw_only_first_entity: bool,
	pub draw_polygon_normals: bool,
}

pub fn draw_frame(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	draw_options: &DrawOptions,
	view_matrix: &Mat4f,
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
		view_matrix,
		map,
		map_polygonized,
		map_bsp,
	);
}

fn draw_background(pixels: &mut [Color32])
{
	for pixel in pixels.iter_mut()
	{
		*pixel = Color32::from_rgb(16, 8, 4);
	}
}

fn draw_map(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	draw_options: &DrawOptions,
	view_matrix: &Mat4f,
	map: Option<&map_file::MapFileParsed>,
	map_polygonized: Option<&map_polygonizer::MapPolygonized>,
	map_bsp: Option<&bsp_builder::BSPTree>,
)
{
	let mut rasterizer = DebugRasterizer::new(pixels, surface_info);

	let fixed_scale = FIXED16_ONE as f32;
	let mat = Mat4f::from_nonuniform_scale(fixed_scale, fixed_scale, 1.0) * view_matrix;

	if draw_options.draw_raw_map
	{
		if let Some(map_non_opt) = map
		{
			draw_map_brushes(&mut rasterizer, &mat, map_non_opt, draw_options.draw_only_first_entity);
		}
	}

	if draw_options.draw_polygonized_map
	{
		if let Some(map_polygonized_non_opt) = map_polygonized
		{
			draw_map_polygonized(
				&mut rasterizer,
				&mat,
				map_polygonized_non_opt,
				draw_options.draw_only_first_entity,
				draw_options.draw_polygon_normals,
			);
		}
	}

	if draw_options.draw_bsp_map
	{
		if let Some(map_bsp_non_opt) = map_bsp
		{
			draw_map_bsp_r(&mut rasterizer, &mat, map_bsp_non_opt);
		}
	}

	draw_basis(&mut rasterizer, &mat);
}

fn draw_map_brushes(
	rasterizer: &mut DebugRasterizer,
	transform_matrix: &Mat4f,
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
				draw_triangle(rasterizer, &transform_matrix, &brush_plane.vertices, color);
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
	transform_matrix: &Mat4f,
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

			for i in 0 .. polygon.vertices.len() - 2
			{
				let vertices = [polygon.vertices[0], polygon.vertices[i + 1], polygon.vertices[i + 2]];
				draw_triangle(rasterizer, &transform_matrix, &vertices, color);
			}

			if draw_polygon_normals
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
				draw_line(rasterizer, &transform_matrix, &line);
			}
		}
		if draw_only_first_entity
		{
			break;
		}
	}
}

fn draw_map_bsp_r(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, bsp_node: &bsp_builder::BSPNodeChild)
{
	match bsp_node
	{
		bsp_builder::BSPNodeChild::NodeChild(node) =>
		{
			for child in &node.children
			{
				draw_map_bsp_r(rasterizer, transform_matrix, child);
			}
		},
		bsp_builder::BSPNodeChild::LeafChild(leaf) =>
		{
			draw_map_bsp_leaf(rasterizer, transform_matrix, leaf);
		},
	}
}

fn draw_map_bsp_leaf(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, bsp_leaf: &bsp_builder::BSPLeaf)
{
	let leaf_ptr_as_int = bsp_leaf as *const bsp_builder::BSPLeaf as usize;
	let color = get_pseudo_random_color(leaf_ptr_as_int / std::mem::size_of::<bsp_builder::BSPLeaf>());

	for polygon in &bsp_leaf.polygons
	{
		if polygon.vertices.len() < 3
		{
			continue;
		}

		for i in 0 .. polygon.vertices.len() - 2
		{
			let vertices = [polygon.vertices[0], polygon.vertices[i + 1], polygon.vertices[i + 2]];
			draw_triangle(rasterizer, &transform_matrix, &vertices, color);
		}
	}
}

fn draw_basis(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f)
{
	let basis_lines = [
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(1.0, 0.0, 0.0),
			Color32::from_rgb(255, 0, 0),
		),
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(0.0, 1.0, 0.0),
			Color32::from_rgb(0, 255, 0),
		),
		(
			Vec3f::new(0.0, 0.0, 0.0),
			Vec3f::new(0.0, 0.0, 1.0),
			Color32::from_rgb(0, 0, 255),
		),
	];

	for line in &basis_lines
	{
		draw_line(rasterizer, &transform_matrix, line);
	}
}

fn get_pseudo_random_color(num: usize) -> Color32
{
	let num = num * 16;
	Color32::from_rgb((num & 255) as u8, ((num * 3) & 255) as u8, ((num * 5) & 255) as u8)
}

type WorldLine = (Vec3f, Vec3f, Color32);

fn draw_line(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, line: &WorldLine)
{
	let fixed_scale = FIXED16_ONE as f32;
	let width = (rasterizer.get_width() as f32) * fixed_scale;
	let height = (rasterizer.get_width() as f32) * fixed_scale;

	let v0 = transform_matrix * line.0.extend(1.0);
	let v1 = transform_matrix * line.1.extend(1.0);

	// TODO - perform proper clipping
	if v0.w <= 0.1 || v1.w <= 0.1
	{
		return;
	}
	let v0 = v0.truncate() / v0.w;
	let v1 = v1.truncate() / v1.w;

	if v0.x < 0.0 ||
		v0.x > width ||
		v0.y < 0.0 ||
		v0.y > height ||
		v1.x < 0.0 ||
		v1.x > width ||
		v1.y < 0.0 ||
		v1.y > height
	{
		return;
	}

	rasterizer.draw_line(
		PointProjected {
			x: v0.x as Fixed16,
			y: v0.y as Fixed16,
			z: v0.z,
		},
		PointProjected {
			x: v1.x as Fixed16,
			y: v1.y as Fixed16,
			z: v1.z,
		},
		line.2,
	);
}

fn draw_triangle(rasterizer: &mut DebugRasterizer, transform_matrix: &Mat4f, vertices: &[Vec3f; 3], color: Color32)
{
	let fixed_scale = FIXED16_ONE as f32;
	let width = (rasterizer.get_width() as f32) * fixed_scale;
	let height = (rasterizer.get_width() as f32) * fixed_scale;

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
		v2.x > height
	{
		return;
	}

	if (v0.truncate() - v1.truncate()).perp_dot(v0.truncate() - v2.truncate()) <= 0.0
	{
		return;
	}

	rasterizer.fill_triangle(
		&[
			PointProjected {
				x: v0.x as Fixed16,
				y: v0.y as Fixed16,
				z: v0.z,
			},
			PointProjected {
				x: v1.x as Fixed16,
				y: v1.y as Fixed16,
				z: v1.z,
			},
			PointProjected {
				x: v2.x as Fixed16,
				y: v2.y as Fixed16,
				z: v2.z,
			},
		],
		color,
	);
}
