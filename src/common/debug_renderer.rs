use super::{color::*, fixed_math::*, system_window};

pub struct DebugRenderer<'a>
{
	color_buffer: &'a mut [Color32],
	width: i32,
	height: i32,
	row_size: i32,
	depth_buffer: Vec<f32>,
}

impl<'a> DebugRenderer<'a>
{
	pub fn new(color_buffer: &'a mut [Color32], surface_info: &system_window::SurfaceInfo) -> Self
	{
		DebugRenderer {
			color_buffer,
			width: surface_info.width as i32,
			height: surface_info.height as i32,
			row_size: (surface_info.pitch) as i32,
			depth_buffer: vec![1.0; surface_info.width * surface_info.pitch],
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

	pub fn draw_line(&mut self, mut v0: PointProjected, mut v1: PointProjected, color: Color32)
	{
		// TODO - optimize this. Discard lines totally outside viewport.
		// TODO - process depth using fixed values, instead of floating point.

		if (v1.x - v0.x).abs() >= (v1.y - v0.y).abs()
		{
			if v0.x > v1.x
			{
				std::mem::swap(&mut v0, &mut v1);
			}
			if v0.x == v1.x
			{
				return;
			}

			let dy_dx = fixed16_div(v1.y - v0.y, v1.x - v0.x);
			let dz_dx = (v1.z - v0.z) / fixed16_to_f32(v1.x - v0.x);
			let x_int_start = fixed16_round_to_int(v0.x).max(0);
			let x_int_end = fixed16_round_to_int(v1.x).min(self.width);
			let x_delta = int_to_fixed16(x_int_start) + FIXED16_HALF - v0.x;
			let mut y = v0.y + fixed16_mul(x_delta, dy_dx) + FIXED16_HALF; // Add extra half to replace expensive "round" with cheap "floor" in loop.
			let mut z = v0.z + fixed16_to_f32(x_delta) * dz_dx;
			for x_int in x_int_start .. x_int_end
			{
				let y_int = fixed16_floor_to_int(y);
				if y_int >= 0 && y_int < self.height
				{
					let pix_address = (x_int + y_int * self.row_size) as usize;
					if z <= self.depth_buffer[pix_address]
					{
						self.color_buffer[pix_address] = color;
						self.depth_buffer[pix_address] = z;
					}
				}
				y += dy_dx;
				z += dz_dx;
			}
		}
		else
		{
			if v0.y > v1.y
			{
				std::mem::swap(&mut v0, &mut v1);
			}
			if v0.y == v1.y
			{
				return;
			}

			let dx_dy = fixed16_div(v1.x - v0.x, v1.y - v0.y);
			let dz_dy = (v1.z - v0.z) / fixed16_to_f32(v1.y - v0.y);
			let y_int_start = fixed16_round_to_int(v0.y).max(0);
			let y_int_end = fixed16_round_to_int(v1.y).min(self.height);
			let y_delta = int_to_fixed16(y_int_start) + FIXED16_HALF - v0.y;
			let mut x = v0.x + fixed16_mul(y_delta, dx_dy) + FIXED16_HALF; // Add extra half to replace expensive "round" with cheap "floor" in loop.
			let mut z = v0.z + fixed16_to_f32(y_delta) * dz_dy;
			for y_int in y_int_start .. y_int_end
			{
				let x_int = fixed16_floor_to_int(x);
				if x_int >= 0 && x_int < self.width
				{
					let pix_address = (x_int + y_int * self.row_size) as usize;
					if z <= self.depth_buffer[pix_address]
					{
						self.color_buffer[pix_address] = color;
						self.depth_buffer[pix_address] = z;
					}
				}
				x += dx_dy;
				z += dz_dy;
			}
		}
	}

	pub fn fill_triangle(&mut self, vertices: &[PointProjected; 3], color: Color32)
	{
		// TODO - process thin triangles specially.

		// Sort triangle vertices.
		let upper_index;
		let middle_index;
		let lower_index;
		if vertices[0].y >= vertices[1].y && vertices[0].y >= vertices[2].y
		{
			upper_index = 0;
			lower_index = if vertices[1].y < vertices[2].y { 1 } else { 2 };
		}
		else if vertices[1].y >= vertices[0].y && vertices[1].y >= vertices[2].y
		{
			upper_index = 1;
			lower_index = if vertices[0].y < vertices[2].y { 0 } else { 2 };
		}
		else
		{
			upper_index = 2;
			lower_index = if vertices[0].y < vertices[1].y { 0 } else { 1 };
		}
		middle_index = 0 + 1 + 2 - upper_index - lower_index;

		let long_edge_dy = vertices[upper_index].y - vertices[lower_index].y;
		if long_edge_dy < FIXED16_HALF
		{
			return;
		}

		let long_edge_dx_dy = fixed16_div(vertices[upper_index].x - vertices[lower_index].x, long_edge_dy);
		let long_edge_x_in_middle =
			vertices[lower_index].x + fixed16_mul(long_edge_dx_dy, vertices[middle_index].y - vertices[lower_index].y);

		let lower_part_dy = vertices[middle_index].y - vertices[lower_index].y;
		let upper_part_dy = vertices[upper_index].y - vertices[middle_index].y;

		let long_edge_dz_dy = (vertices[upper_index].z - vertices[lower_index].z) / fixed16_to_f32(long_edge_dy);
		let long_edge_z_in_middle = vertices[lower_index].z +
			long_edge_dz_dy * fixed16_to_f32(vertices[middle_index].y - vertices[lower_index].y);

		if long_edge_x_in_middle >= vertices[middle_index].x
		{
			//    /\
			//   /  \
			//  /    \
			// +_     \  <-
			//    _    \
			//      _   \
			//        _  \
			//          _ \
			//            _\

			let dz_dx = (long_edge_z_in_middle - vertices[middle_index].z) /
				fixed16_to_f32(long_edge_x_in_middle - vertices[middle_index].x);
			if lower_part_dy >= FIXED16_HALF
			{
				self.fill_polygon_part(
					vertices[lower_index].y,
					vertices[middle_index].y,
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: fixed16_div(vertices[middle_index].x - vertices[lower_index].x, lower_part_dy),
						z_start: vertices[lower_index].z,
						dz_dy: (vertices[middle_index].z - vertices[lower_index].z) / fixed16_to_f32(lower_part_dy),
					},
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: long_edge_dx_dy,
						z_start: vertices[lower_index].z,
						dz_dy: long_edge_dz_dy,
					},
					dz_dx,
					color,
				);
			}
			if upper_part_dy >= FIXED16_HALF
			{
				self.fill_polygon_part(
					vertices[middle_index].y,
					vertices[upper_index].y,
					PolygonSide {
						x_start: vertices[middle_index].x,
						dx_dy: fixed16_div(vertices[upper_index].x - vertices[middle_index].x, upper_part_dy),
						z_start: vertices[middle_index].z,
						dz_dy: (vertices[upper_index].z - vertices[middle_index].z) / fixed16_to_f32(upper_part_dy),
					},
					PolygonSide {
						x_start: long_edge_x_in_middle,
						dx_dy: long_edge_dx_dy,
						z_start: long_edge_z_in_middle,
						dz_dy: long_edge_dz_dy,
					},
					dz_dx,
					color,
				);
			}
		}
		else
		{
			//         /\
			//        /  \
			//       /    \
			// ->   /     _+
			//     /    _
			//    /   _
			//   /  _
			//  / _
			// /_

			let dz_dx = (vertices[middle_index].z - long_edge_z_in_middle) /
				fixed16_to_f32(vertices[middle_index].x - long_edge_x_in_middle);
			if lower_part_dy >= FIXED16_HALF
			{
				self.fill_polygon_part(
					vertices[lower_index].y,
					vertices[middle_index].y,
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: long_edge_dx_dy,
						z_start: vertices[lower_index].z,
						dz_dy: long_edge_dz_dy,
					},
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: fixed16_div(vertices[middle_index].x - vertices[lower_index].x, lower_part_dy),
						z_start: vertices[lower_index].z,
						dz_dy: (vertices[middle_index].z - vertices[lower_index].z) / fixed16_to_f32(lower_part_dy),
					},
					dz_dx,
					color,
				);
			}
			if upper_part_dy >= FIXED16_HALF
			{
				self.fill_polygon_part(
					vertices[middle_index].y,
					vertices[upper_index].y,
					PolygonSide {
						x_start: long_edge_x_in_middle,
						dx_dy: long_edge_dx_dy,
						z_start: long_edge_z_in_middle,
						dz_dy: long_edge_dz_dy,
					},
					PolygonSide {
						x_start: vertices[middle_index].x,
						dx_dy: fixed16_div(vertices[upper_index].x - vertices[middle_index].x, upper_part_dy),
						z_start: vertices[middle_index].z,
						dz_dy: (vertices[upper_index].z - vertices[middle_index].z) / fixed16_to_f32(upper_part_dy),
					},
					dz_dx,
					color,
				);
			}
		}
	}

	fn fill_polygon_part(
		&mut self,
		y_start: Fixed16,
		y_end: Fixed16,
		left_side: PolygonSide,
		right_side: PolygonSide,
		dz_dx: f32,
		color: Color32,
	)
	{
		// TODO replace "F32" with Fixed16 for Z calculation.
		let y_start_int = fixed16_round_to_int(y_start).max(0);
		let y_end_int = fixed16_round_to_int(y_end).min(self.height);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		let mut z_left = left_side.z_start + fixed16_to_f32(y_start_delta) * left_side.dz_dy;
		let mut _z_right = right_side.z_start + fixed16_to_f32(y_start_delta) * right_side.dz_dy;
		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(0);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.width);
			let x_start_delta = int_to_fixed16(x_start_int) + FIXED16_HALF - x_left;
			let mut z = z_left + fixed16_to_f32(x_start_delta) * dz_dx;
			for x_int in x_start_int .. x_end_int
			{
				let pix_address = (x_int + y_int * self.row_size) as usize;
				if z <= self.depth_buffer[pix_address]
				{
					self.color_buffer[pix_address] = color;
					self.depth_buffer[pix_address] = z;
				}

				z += dz_dx;
			}

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			z_left += left_side.dz_dy;
		}
	}
}

pub struct PointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
	pub z: f32,
}

struct PolygonSide
{
	x_start: Fixed16,
	dx_dy: Fixed16,
	z_start: f32,
	dz_dy: f32,
}
