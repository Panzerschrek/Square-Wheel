use super::{clipping_polygon::*, rasterizer::*, renderer_config::*};
use common::{
	bsp_map_compact, camera_controller::CameraMatrices, clipping::*, color::*, fixed_math::*, math_types::*, plane::*,
	system_window,
};

pub struct Renderer
{
	current_frame: FrameNumber,
	config: RendererConfig,
	map: bsp_map_compact::BSPMap,
	leafs_data: Vec<DrawLeafData>,
}

// Mutable data associated with map BSP Leaf.
#[derive(Default, Copy, Clone)]
struct DrawLeafData
{
	// Frame last time this leaf was visible.
	visible_frame: FrameNumber,
	// Depth for visible leafs search.
	search_depth: u32,
	current_frame_bounds: ClippingPolygon,
}

// 32 bits are enough for frames enumeration.
// It is more than year at 60FPS.
#[derive(Default, Copy, Clone, PartialEq, Eq)]
struct FrameNumber(u32);

impl Renderer
{
	pub fn new(app_config: &serde_json::Value, map: bsp_map_compact::BSPMap) -> Self
	{
		Renderer {
			current_frame: FrameNumber(0),
			config: RendererConfig::from_app_config(app_config),
			leafs_data: vec![DrawLeafData::default(); map.leafs.len()],
			map,
		}
	}

	pub fn draw_frame(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		camera_matrices: &CameraMatrices,
	)
	{
		self.current_frame.0 += 1;

		if self.config.clear_background
		{
			draw_background(pixels);
		}

		self.draw_map(pixels, surface_info, camera_matrices);

		// TODO - remove such temporary fuinction.
		draw_crosshair(pixels, surface_info);

		if self.config.show_stats
		{
			let mut num_visible_leafs = 0;
			for leaf_data in &self.leafs_data
			{
				if leaf_data.visible_frame == self.current_frame
				{
					num_visible_leafs += 1;
				}
			}

			common::text_printer::print(
				pixels,
				surface_info,
				&format!("leafs: {}", num_visible_leafs),
				0,
				0,
				Color32::from_rgb(255, 255, 255),
			);
		}
	}

	fn draw_map(
		&mut self,
		pixels: &mut [Color32],
		surface_info: &system_window::SurfaceInfo,
		camera_matrices: &CameraMatrices,
	)
	{
		let mut rasterizer = Rasterizer::new(pixels, surface_info);
		let root_node = (self.map.nodes.len() - 1) as u32;
		let current_leaf = self.find_current_leaf(root_node, &camera_matrices.planes_matrix);

		let frame_bounds = ClippingPolygon::from_box(0.0, 0.0, surface_info.width as f32, surface_info.height as f32);
		mark_reachable_leafs_r(
			current_leaf,
			&self.map,
			self.current_frame,
			camera_matrices,
			0,
			&frame_bounds,
			&mut self.leafs_data,
		);

		// Draw BSP tree in back to front order, skip unreachable leafs.
		self.draw_tree_r(&mut rasterizer, camera_matrices, root_node);
	}

	fn find_current_leaf(&self, mut index: u32, planes_matrix: &Mat4f) -> u32
	{
		loop
		{
			if index >= bsp_map_compact::FIRST_LEAF_INDEX
			{
				return index - bsp_map_compact::FIRST_LEAF_INDEX;
			}

			let node = &self.map.nodes[index as usize];
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

	fn draw_tree_r(&self, rasterizer: &mut Rasterizer, camera_matrices: &CameraMatrices, current_index: u32)
	{
		if current_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf = current_index - bsp_map_compact::FIRST_LEAF_INDEX;
			let leaf_data = &self.leafs_data[leaf as usize];
			if leaf_data.visible_frame == self.current_frame
			{
				let color = Color32::from_rgb(
					((leaf_data.search_depth * 28).min(255)) as u8,
					((leaf_data.search_depth * 24).min(255)) as u8,
					((leaf_data.search_depth * 20).min(255)) as u8,
				);

				self.draw_leaf(rasterizer, camera_matrices, &self.map.leafs[leaf as usize], color);
			}
		}
		else
		{
			let node = &self.map.nodes[current_index as usize];
			let plane_transformed = camera_matrices.planes_matrix * node.plane.vec.extend(-node.plane.dist);
			let mask = if plane_transformed.w >= 0.0 { 1 } else { 0 };
			for i in 0 .. 2
			{
				self.draw_tree_r(rasterizer, camera_matrices, node.children[(i ^ mask) as usize]);
			}
		}
	}

	fn draw_leaf(
		&self,
		rasterizer: &mut Rasterizer,
		camera_matrices: &CameraMatrices,
		leaf: &bsp_map_compact::BSPLeaf,
		color: Color32,
	)
	{
		for polygon in
			&self.map.polygons[(leaf.first_polygon as usize) .. ((leaf.first_polygon + leaf.num_polygons) as usize)]
		{
			draw_polygon(
				rasterizer,
				camera_matrices,
				&polygon.plane,
				&self.map.vertices
					[(polygon.first_vertex as usize) .. ((polygon.first_vertex + polygon.num_vertices) as usize)],
				&polygon.tex_coord_equation,
				color,
			);
		}
	}
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

fn mark_reachable_leafs_r(
	leaf: u32,
	map: &bsp_map_compact::BSPMap,
	current_frame: FrameNumber,
	camera_matrices: &CameraMatrices,
	depth: u32,
	bounds: &ClippingPolygon,
	leafs_data: &mut [DrawLeafData],
)
{
	let max_depth = 1024; // Prevent stack overflow in case of broken graph.
	if depth > max_depth
	{
		return;
	}

	let leaf_data = &mut leafs_data[leaf as usize];
	if leaf_data.visible_frame != current_frame
	{
		leaf_data.visible_frame = current_frame;
		leaf_data.search_depth = depth;
		leaf_data.current_frame_bounds = *bounds;
	}
	// TODO - return this check.
	// else if leaf_data.current_frame_bounds.contains(bounds)
	//{
	// 	return;
	//}
	else
	{
		leaf_data.search_depth = std::cmp::min(leaf_data.search_depth, depth);
		leaf_data.current_frame_bounds.extend(bounds);
	}

	let leaf_value = map.leafs[leaf as usize];
	for portal in &map.leafs_portals[(leaf_value.first_leaf_portal as usize) ..
		((leaf_value.first_leaf_portal + leaf_value.num_leaf_portals) as usize)]
	{
		let portal_value = &map.portals[(*portal) as usize];

		// Do not look through portals that are facing from camera.
		let portal_plane_pos =
			(camera_matrices.planes_matrix * portal_value.plane.vec.extend(-portal_value.plane.dist)).w;

		let next_leaf;
		if portal_value.leafs[0] == leaf
		{
			if portal_plane_pos < 0.0
			{
				continue;
			}
			next_leaf = portal_value.leafs[1];
		}
		else
		{
			if portal_plane_pos > 0.0
			{
				continue;
			}
			next_leaf = portal_value.leafs[0];
		}

		// Project portal, calculate its bounds, calculat intersection with current bounds.
		const MAX_VERTICES: usize = 24;
		let mut vertex_count = portal_value.num_vertices as usize;

		// Perform initial matrix tranformation, obtain 3d vertices in camera-aligned space.
		let mut vertices_transformed = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
		for (index, vertex) in map.vertices
			[(portal_value.first_vertex as usize) .. (portal_value.first_vertex + portal_value.num_vertices) as usize]
			.iter()
			.enumerate()
			.take(MAX_VERTICES)
		{
			let vertex_transformed = camera_matrices.view_matrix * vertex.extend(1.0);
			vertices_transformed[index] = Vec3f::new(vertex_transformed.x, vertex_transformed.y, vertex_transformed.w);
		}

		// Perform z_near clipping. Use very small z_near to avoid clipping portals.
		// TODO - add some workaround for really small z_near.
		let mut vertices_transformed_z_clipped = [Vec3f::zero(); MAX_VERTICES]; // TODO - use uninitialized memory
		const Z_NEAR: f32 = 0.01;
		vertex_count = clip_3d_polygon_by_z_plane(
			&vertices_transformed[.. vertex_count],
			Z_NEAR,
			&mut vertices_transformed_z_clipped,
		);
		if vertex_count < 3
		{
			continue;
		}

		let mut portal_polygon_bounds = ClippingPolygon::from_point(
			&(vertices_transformed_z_clipped[0].truncate() / vertices_transformed_z_clipped[0].z),
		);
		for vertex_transformed in &vertices_transformed_z_clipped[1 .. vertex_count]
		{
			portal_polygon_bounds.extend_with_point(&(vertex_transformed.truncate() / vertex_transformed.z));
		}

		let mut bounds_intersection = *bounds;
		bounds_intersection.intersect(&portal_polygon_bounds);

		if bounds_intersection.is_empty_or_invalid()
		{
			continue;
		}

		mark_reachable_leafs_r(
			next_leaf,
			map,
			current_frame,
			camera_matrices,
			depth + 1,
			&bounds_intersection,
			leafs_data,
		);
	}
}

fn draw_polygon(
	rasterizer: &mut Rasterizer,
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
