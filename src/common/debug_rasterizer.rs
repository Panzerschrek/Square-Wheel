use super::{color::*, fixed_math::*, system_window};

pub struct DebugRasterizer<'a>
{
	color_buffer: &'a mut [Color32],
	width: i32,
	height: i32,
	row_size: i32,
	depth_buffer: Vec<f32>,
}

impl<'a> DebugRasterizer<'a>
{
	pub fn new(color_buffer: &'a mut [Color32], surface_info: &system_window::SurfaceInfo) -> Self
	{
		DebugRasterizer {
			color_buffer,
			width: surface_info.width as i32,
			height: surface_info.height as i32,
			row_size: (surface_info.pitch) as i32,
			depth_buffer: vec![1.0e10; surface_info.width * surface_info.pitch],
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

	pub fn fill_triangle(&mut self, vertices: &[PointProjected; 3], depth_equation: &DepthEquation, color: Color32)
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

			if lower_part_dy >= FIXED16_HALF
			{
				self.fill_polygon_part(
					vertices[lower_index].y,
					vertices[middle_index].y,
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: fixed16_div(vertices[middle_index].x - vertices[lower_index].x, lower_part_dy),
					},
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: long_edge_dx_dy,
					},
					depth_equation,
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
					},
					PolygonSide {
						x_start: long_edge_x_in_middle,
						dx_dy: long_edge_dx_dy,
					},
					depth_equation,
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

			if lower_part_dy >= FIXED16_HALF
			{
				self.fill_polygon_part(
					vertices[lower_index].y,
					vertices[middle_index].y,
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: long_edge_dx_dy,
					},
					PolygonSide {
						x_start: vertices[lower_index].x,
						dx_dy: fixed16_div(vertices[middle_index].x - vertices[lower_index].x, lower_part_dy),
					},
					depth_equation,
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
					},
					PolygonSide {
						x_start: vertices[middle_index].x,
						dx_dy: fixed16_div(vertices[upper_index].x - vertices[middle_index].x, upper_part_dy),
					},
					depth_equation,
					color,
				);
			}
		}
	}
	
	// Fill convex clockwise polygon.
	pub fn fill_polygon(
		&mut self,
		vertices : &[PointProjected],
		depth_equation: &DepthEquation,
		color: Color32,
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
			let next_y = std::cmp::min( vertices[next_left_index].y, vertices[next_right_index].y );
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
					color,
				);
			}
			else
			{
				// TODO - handle thin polygon segments
			}
			
			if vertices[next_right_index].y < vertices[next_left_index].y
			{
				right_index = next_right_index;
			}
			else
			{
				left_index = next_left_index;
			}
			if left_index == right_index
			{
				break;
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
		color: Color32,
	)
	{
		// TODO replace "F32" with Fixed16 for Z calculation.
		let y_start_int = fixed16_round_to_int(y_start).max(0);
		let y_end_int = fixed16_round_to_int(y_end).min(self.height);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		let mut line_z = (y_start_int as f32 + 0.5) * depth_equation.dz_dy + depth_equation.k;
		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(0);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.width);
			let mut z = (x_start_int as f32 + 0.5) * depth_equation.dz_dx + line_z;
			for x_int in x_start_int .. x_end_int
			{
				let pix_address = (x_int + y_int * self.row_size) as usize;
				if z <= self.depth_buffer[pix_address]
				{
					let inv_z = -0.001 / z;
					let mut brightness = 255.0 * inv_z;
					if brightness < 0.0 { brightness = 0.0; }
					if brightness > 255.0 { brightness= 255.0; }

					//self.color_buffer[pix_address] = Color32::from_rgb(dz_dx as u8, dz_dy as u8, brightness as u8);
					self.color_buffer[pix_address] = color;
					self.depth_buffer[pix_address] = z;
				}

				z += depth_equation.dz_dx;
			}

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			line_z += depth_equation.dz_dy;
		}
	}
}

#[derive(Copy, Clone)]
pub struct PointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
	pub z: f32,
}

pub struct DepthEquation
{
	pub dz_dx : f32,
	pub dz_dy : f32,
	pub k : f32,
}

struct PolygonSide
{
	x_start: Fixed16,
	dx_dy: Fixed16,
}
