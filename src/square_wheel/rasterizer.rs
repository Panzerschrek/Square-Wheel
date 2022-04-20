use common::{color::*, fixed_math::*, system_window};

pub struct Rasterizer<'a>
{
	color_buffer: &'a mut [Color32],
	width: i32,
	height: i32,
	row_size: i32,
}

impl<'a> Rasterizer<'a>
{
	pub fn new(color_buffer: &'a mut [Color32], surface_info: &system_window::SurfaceInfo) -> Self
	{
		Rasterizer {
			color_buffer,
			width: surface_info.width as i32,
			height: surface_info.height as i32,
			row_size: (surface_info.pitch) as i32,
		}
	}

	pub fn get_width(&self) -> i32
	{
		self.width
	}

	pub fn get_height(&self) -> i32
	{
		self.height
	}

	// Fill convex clockwise polygon.
	pub fn fill_polygon(
		&mut self,
		vertices: &[PolygonPointProjected],
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[Color32],
	)
	{
		// Search for start vertex (with min y).
		let mut lower_vertex_index = 0;
		let mut min_y = vertices[0].y;
		for (index, vertex) in vertices.iter().enumerate()
		{
			if vertex.y < min_y
			{
				min_y = vertex.y;
				lower_vertex_index = index;
			}
		}

		let mut left_index = lower_vertex_index;
		let mut right_index = lower_vertex_index;
		let mut cur_y = min_y;
		loop
		{
			let mut next_left_index = left_index + vertices.len() - 1;
			if next_left_index >= vertices.len()
			{
				next_left_index -= vertices.len();
			}

			let mut next_right_index = right_index + 1;
			if next_right_index >= vertices.len()
			{
				next_right_index -= vertices.len();
			}

			let dy_left = vertices[next_left_index].y - vertices[left_index].y;
			let dy_right = vertices[next_right_index].y - vertices[right_index].y;
			let next_y = std::cmp::min(vertices[next_left_index].y, vertices[next_right_index].y);
			if dy_left > FIXED16_HALF && dy_right > FIXED16_HALF
			{
				let dx_dy_left = fixed16_div(vertices[next_left_index].x - vertices[left_index].x, dy_left);
				let dx_dy_right = fixed16_div(vertices[next_right_index].x - vertices[right_index].x, dy_right);
				self.fill_polygon_part(
					cur_y,
					next_y,
					PolygonSide {
						x_start: vertices[left_index].x + fixed16_mul(dx_dy_left, cur_y - vertices[left_index].y),
						dx_dy: dx_dy_left,
					},
					PolygonSide {
						x_start: vertices[right_index].x + fixed16_mul(dx_dy_right, cur_y - vertices[right_index].y),
						dx_dy: dx_dy_right,
					},
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
				);
			}
			else if dy_left > 0 && dy_right > 0
			{
				let cur_y_int = fixed16_round_to_int(cur_y);
				let next_y_int = fixed16_round_to_int(next_y);
				if cur_y_int < next_y_int
				{
					// Fill single line.
					let thin_line_y = int_to_fixed16(cur_y_int) + FIXED16_HALF;
					let x_start_left = vertices[left_index].x +
						fixed16_mul_div(
							thin_line_y - vertices[left_index].y,
							vertices[next_left_index].x - vertices[left_index].x,
							dy_left,
						);
					let x_start_right = vertices[right_index].x +
						fixed16_mul_div(
							thin_line_y - vertices[right_index].y,
							vertices[next_right_index].x - vertices[right_index].x,
							dy_right,
						);
					self.fill_polygon_part(
						cur_y,
						next_y,
						PolygonSide {
							x_start: x_start_left,
							dx_dy: 0,
						},
						PolygonSide {
							x_start: x_start_right,
							dx_dy: 0,
						},
						depth_equation,
						tex_coord_equation,
						texture_info,
						texture_data,
					);
				}
			}

			if next_left_index == next_right_index
			{
				break;
			}

			if vertices[next_right_index].y < vertices[next_left_index].y
			{
				right_index = next_right_index;
			}
			else
			{
				left_index = next_left_index;
			}
			cur_y = next_y;
		}
	}

	fn fill_polygon_part(
		&mut self,
		y_start: Fixed16,
		y_end: Fixed16,
		left_side: PolygonSide,
		right_side: PolygonSide,
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[Color32],
	)
	{
		const FIXED_SHIFT: i32 = 24;
		const FIXED_SCALE: f32 = (1 << FIXED_SHIFT) as f32;
		const INV_Z_SHIFT: i32 = 29;
		const INV_Z_SCALE: f32 = (1 << INV_Z_SHIFT) as f32;
		// let d_inv_z_dx = (INV_Z_SCALE * depth_equation.d_inv_z_dx) as i32;
		// let d_inv_z_dy = (INV_Z_SCALE * depth_equation.d_inv_z_dy) as i32;
		// Add extra 0.5 to shift to pixel center.
		// let inv_z_k =
		// (FIXED_SCALE * (depth_equation.k + (depth_equation.d_inv_z_dx + depth_equation.d_inv_z_dy) * 0.5)) as i64;
		// (INV_Z_SCALE * (depth_equation.k + (depth_equation.d_inv_z_dx + depth_equation.d_inv_z_dy) * 0.5)) as i64;
		let d_inv_z_dx = depth_equation.d_inv_z_dx;
		let d_inv_z_dy = depth_equation.d_inv_z_dy;
		// Add extra 0.5 to shift to pixel center.
		let inv_z_k = depth_equation.k + (depth_equation.d_inv_z_dx + depth_equation.d_inv_z_dy) * 0.5;
		let d_tc_dx = [
			(FIXED_SCALE * tex_coord_equation.d_tc_dx[0]) as i32,
			(FIXED_SCALE * tex_coord_equation.d_tc_dx[1]) as i32,
		];
		let d_tc_dy = [
			(FIXED_SCALE * tex_coord_equation.d_tc_dy[0]) as i32,
			(FIXED_SCALE * tex_coord_equation.d_tc_dy[1]) as i32,
		];
		let tc_k = [
			(FIXED_SCALE *
				(tex_coord_equation.k[0] + (tex_coord_equation.d_tc_dx[0] + tex_coord_equation.d_tc_dy[0]) * 0.5)) as i64,
			(FIXED_SCALE *
				(tex_coord_equation.k[1] + (tex_coord_equation.d_tc_dx[1] + tex_coord_equation.d_tc_dy[1]) * 0.5)) as i64,
		];

		// TODO - avoid adding "0.5" for some calculations.
		let y_start_int = fixed16_round_to_int(y_start).max(0);
		let y_end_int = fixed16_round_to_int(y_end).min(self.height);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		let mut line_inv_z = (y_start_int as f32) * d_inv_z_dy + inv_z_k;
		let mut line_tc = [
			(y_start_int as i64) * (d_tc_dy[0] as i64) + tc_k[0],
			(y_start_int as i64) * (d_tc_dy[1] as i64) + tc_k[1],
		];

		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(0);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.width);
			if x_start_int < x_end_int
			{
				let mut inv_z = (x_start_int as f32) * d_inv_z_dx + line_inv_z;
				let mut tc = [
					((x_start_int as i64) * (d_tc_dx[0] as i64) + line_tc[0]) as i32,
					((x_start_int as i64) * (d_tc_dx[1] as i64) + line_tc[1]) as i32,
				];
				let line_buffer_offset = y_int * self.row_size;
				let line_dst = &mut self.color_buffer
					[(x_start_int + line_buffer_offset) as usize .. (x_end_int + line_buffer_offset) as usize];

				for dst_pixel in line_dst
				{
					// TODO - correct inv_z start and step to avoid zero checks.
					// const INV_Z_PRE_SHIFT : i32 = 12;
					// let inv_z_shifted = inv_z >> INV_Z_PRE_SHIFT;
					// let z = if inv_z_shifted <= 0
					// {
					// 0
					// }
					// else
					// {
					// (1 << 31) / ((inv_z as u32) >> (INV_Z_PRE_SHIFT as u32))
					// };
					// let mut pix_tc = [
					// 	(((z as i64) * (tc[0] as i64)) >> ((FIXED_SHIFT + 31 - INV_Z_SHIFT + INV_Z_PRE_SHIFT)) as i64) as i32,
					// 	(((z as i64) * (tc[1] as i64)) >> ((FIXED_SHIFT + 31 - INV_Z_SHIFT + INV_Z_PRE_SHIFT)) as i64) as i32,
					// ];
					let z = INV_Z_SCALE / inv_z;
					let mut pix_tc = [
						(((z as i64) * (tc[0] as i64)) >> (FIXED_SHIFT + INV_Z_SHIFT)) as i32,
						(((z as i64) * (tc[1] as i64)) >> (FIXED_SHIFT + INV_Z_SHIFT)) as i32,
					];

					for i in 0 .. 2
					{
						if pix_tc[i] < 0
						{
							pix_tc[i] = 0;
						}
						if pix_tc[i] >= (texture_info.size[i] as i32)
						{
							pix_tc[i] = (texture_info.size[i] as i32) - 1;
						}
					}

					*dst_pixel = texture_data[(pix_tc[0] + pix_tc[1] * (texture_info.size[0] as i32)) as usize];

					inv_z += d_inv_z_dx;
					tc[0] += d_tc_dx[0];
					tc[1] += d_tc_dx[1];
				}
			}

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			line_inv_z += d_inv_z_dy;
			line_tc[0] += d_tc_dy[0] as i64;
			line_tc[1] += d_tc_dy[1] as i64;
		}
	}
}

#[derive(Copy, Clone)]
pub struct PolygonPointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
}

#[derive(Copy, Clone, Default)]
pub struct DepthEquation
{
	pub d_inv_z_dx: f32,
	pub d_inv_z_dy: f32,
	pub k: f32,
}

#[derive(Copy, Clone, Default)]
pub struct TexCoordEquation
{
	pub d_tc_dx: [f32; 2],
	pub d_tc_dy: [f32; 2],
	pub k: [f32; 2],
}

pub struct TextureInfo
{
	pub size: [i32; 2],
}

struct PolygonSide
{
	x_start: Fixed16,
	dx_dy: Fixed16,
}
