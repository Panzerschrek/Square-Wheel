use super::{abstract_color::*, equations::*, fast_math::*};
use crate::common::{fixed_math::*, system_window};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TetureCoordinatesInterpolationMode
{
	FullPerspective,
	LineZCorrection,
	Affine,
}

const TEXTURE_COORDINATES_INTERPOLATION_MODE_FULL_PERSPECTIVE: usize = 0;
const TEXTURE_COORDINATES_INTERPOLATION_MODE_LINE_Z_CORRECTION: usize = 1;
const TEXTURE_COORDINATES_INTERPOLATION_MODE_FULL_AFFINE: usize = 2;

pub use crate::common::material::BlendingMode;

const BLENDING_MODE_NONE: usize = 0;
const BLENDING_MODE_AVERAGE: usize = 1;
const BLENDING_MODE_ADDITIVE: usize = 2;
const BLENDING_MODE_ALPHA_TEST: usize = 3;
const BLENDING_MODE_ALPHA_BLEND: usize = 4;

pub struct Rasterizer<'a, ColorT: AbstractColor>
{
	color_buffer: &'a mut [ColorT],
	row_size: i32,
	clip_rect: ClipRect,
}

impl<'a, ColorT: AbstractColor> Rasterizer<'a, ColorT>
{
	pub fn new(color_buffer: &'a mut [ColorT], surface_info: &system_window::SurfaceInfo, clip_rect: ClipRect) -> Self
	{
		Rasterizer {
			color_buffer,
			row_size: (surface_info.pitch) as i32,
			clip_rect,
		}
	}

	// Fill convex clockwise polygon.
	pub fn fill_polygon(
		&mut self,
		vertices: &[PolygonPointProjected],
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[ColorT],
		texture_coordinates_interpolation_mode: TetureCoordinatesInterpolationMode,
		blending_mode: BlendingMode,
	)
	{
		match texture_coordinates_interpolation_mode
		{
			TetureCoordinatesInterpolationMode::FullPerspective =>
			{
				self.fill_polygon_impl_1_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE_FULL_PERSPECTIVE>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
					blending_mode,
				)
			},
			TetureCoordinatesInterpolationMode::LineZCorrection =>
			{
				self.fill_polygon_impl_1_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE_LINE_Z_CORRECTION>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
					blending_mode,
				)
			},
			TetureCoordinatesInterpolationMode::Affine => self
				.fill_polygon_impl_1_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE_FULL_AFFINE>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
					blending_mode,
				),
		}
	}

	fn fill_polygon_impl_1_static_params<const TEXTURE_COORDINATES_INTERPOLATION_MODE: usize>(
		&mut self,
		vertices: &[PolygonPointProjected],
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[ColorT],
		blending_mode: BlendingMode,
	)
	{
		match blending_mode
		{
			BlendingMode::None => self
				.fill_polygon_impl_2_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE, BLENDING_MODE_NONE>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
				),
			BlendingMode::Average => self
				.fill_polygon_impl_2_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE, BLENDING_MODE_AVERAGE>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
				),
			BlendingMode::Additive => self
				.fill_polygon_impl_2_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE, BLENDING_MODE_ADDITIVE>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
				),
			BlendingMode::AlphaTest => self
				.fill_polygon_impl_2_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE, BLENDING_MODE_ALPHA_TEST>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
				),
			BlendingMode::AlphaBlend => self
				.fill_polygon_impl_2_static_params::<TEXTURE_COORDINATES_INTERPOLATION_MODE, BLENDING_MODE_ALPHA_BLEND>(
					vertices,
					depth_equation,
					tex_coord_equation,
					texture_info,
					texture_data,
				),
		}
	}

	fn fill_polygon_impl_2_static_params<
		const TEXTURE_COORDINATES_INTERPOLATION_MODE: usize,
		const BLENDING_MODE: usize,
	>(
		&mut self,
		vertices: &[PolygonPointProjected],
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[ColorT],
	)
	{
		let draw_func = match TEXTURE_COORDINATES_INTERPOLATION_MODE
		{
			TEXTURE_COORDINATES_INTERPOLATION_MODE_FULL_PERSPECTIVE => Self::fill_polygon_part::<BLENDING_MODE>,
			TEXTURE_COORDINATES_INTERPOLATION_MODE_LINE_Z_CORRECTION =>
			{
				Self::fill_polygon_part_line_z_corrected::<BLENDING_MODE>
			},
			TEXTURE_COORDINATES_INTERPOLATION_MODE_FULL_AFFINE => Self::fill_polygon_part_affine::<BLENDING_MODE>,
			_ => Self::fill_polygon_part::<BLENDING_MODE>,
		};

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

			// TODO - use unchecked fetch?
			let left_vertex = &vertices[left_index];
			let next_left_vertex = &vertices[next_left_index];
			let right_vertex = &vertices[right_index];
			let next_right_vertex = &vertices[next_right_index];

			let dy_left = next_left_vertex.y - left_vertex.y;
			let dy_right = next_right_vertex.y - right_vertex.y;
			let next_y = std::cmp::min(next_left_vertex.y, next_right_vertex.y);
			if dy_left > FIXED16_HALF && dy_right > FIXED16_HALF
			{
				let dx_dy_left = fixed16_div(next_left_vertex.x - left_vertex.x, dy_left);
				let dx_dy_right = fixed16_div(next_right_vertex.x - right_vertex.x, dy_right);
				draw_func(
					self,
					cur_y,
					next_y,
					PolygonSide {
						x_start: left_vertex.x + fixed16_mul(dx_dy_left, cur_y - left_vertex.y),
						dx_dy: dx_dy_left,
					},
					PolygonSide {
						x_start: right_vertex.x + fixed16_mul(dx_dy_right, cur_y - right_vertex.y),
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
					let x_start_left = left_vertex.x +
						fixed16_mul_div(thin_line_y - left_vertex.y, next_left_vertex.x - left_vertex.x, dy_left);
					let x_start_right = right_vertex.x +
						fixed16_mul_div(
							thin_line_y - right_vertex.y,
							next_right_vertex.x - right_vertex.x,
							dy_right,
						);
					draw_func(
						self,
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

			if next_right_vertex.y < next_left_vertex.y
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

	fn fill_polygon_part<const BLENDING_MODE: usize>(
		&mut self,
		y_start: Fixed16,
		y_end: Fixed16,
		left_side: PolygonSide,
		right_side: PolygonSide,
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[ColorT],
	)
	{
		debug_assert!(texture_data.len() >= (texture_info.size[0] * texture_info.size[1]) as usize);

		let d_inv_z_dx = unchecked_to_int64(INV_Z_SCALE * depth_equation.d_inv_z_dx);
		let d_inv_z_dy = unchecked_to_int64(INV_Z_SCALE * depth_equation.d_inv_z_dy);
		// Add extra 0.5 to shift to pixel center.
		let inv_z_k = unchecked_to_int64(
			INV_Z_SCALE * (depth_equation.k + (depth_equation.d_inv_z_dx + depth_equation.d_inv_z_dy) * 0.5),
		);
		let d_tc_dx = [
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dx[0]),
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dx[1]),
		];
		let d_tc_dy = [
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dy[0]),
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dy[1]),
		];
		let tc_k = [
			unchecked_to_int64(
				TC_SCALE *
					(tex_coord_equation.k[0] + (tex_coord_equation.d_tc_dx[0] + tex_coord_equation.d_tc_dy[0]) * 0.5),
			),
			unchecked_to_int64(
				TC_SCALE *
					(tex_coord_equation.k[1] + (tex_coord_equation.d_tc_dx[1] + tex_coord_equation.d_tc_dy[1]) * 0.5),
			),
		];

		let texture_size_minus_one = [texture_info.size[0] - 1, texture_info.size[1] - 1];
		let texture_width = texture_info.size[0] as u32;

		// TODO - avoid adding "0.5" for some calculations.
		let y_start_int = fixed16_round_to_int(y_start).max(self.clip_rect.min_y);
		let y_end_int = fixed16_round_to_int(y_end).min(self.clip_rect.max_y);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		let mut line_inv_z = (y_start_int as i64) * d_inv_z_dy + inv_z_k;
		let mut line_tc = [
			(y_start_int as i64) * d_tc_dy[0] + tc_k[0],
			(y_start_int as i64) * d_tc_dy[1] + tc_k[1],
		];

		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(self.clip_rect.min_x);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.clip_rect.max_x);
			if x_start_int < x_end_int
			{
				let line_buffer_offset = y_int * self.row_size;
				let line_dst = unchecked_slice_range_mut(
					&mut self.color_buffer,
					(x_start_int + line_buffer_offset) as usize,
					(x_end_int + line_buffer_offset) as usize,
				);

				let span_start_x = x_start_int as i64;
				let span_end_x = (x_end_int - 1) as i64;
				let span_length_minus_one = (x_end_int - 1) - x_start_int;

				// Correct inv_z to prevent overflow/underflow, negative values and divsion by zero.
				let mut span_inv_z;
				let span_d_inv_z;
				{
					let inv_z_min = 1 << INV_Z_PRE_SHIFT;
					let inv_z_max = 1 << 29;
					let span_inv_z_start = std::cmp::max(
						inv_z_min,
						std::cmp::min(span_start_x * d_inv_z_dx + line_inv_z, inv_z_max),
					) as i32;
					let span_inv_z_end = std::cmp::max(
						inv_z_min,
						std::cmp::min(span_end_x * d_inv_z_dx + line_inv_z, inv_z_max),
					) as i32;
					if span_length_minus_one > 0
					{
						span_d_inv_z = (span_inv_z_end - span_inv_z_start) / span_length_minus_one;
					}
					else
					{
						span_d_inv_z = 0;
					}
					span_inv_z = span_inv_z_start;
				}

				// Correct texture coordinates in order to avoid negative values.
				// TODO - prove that this code generates proper texture coordinates equation thats guarantee us 100% protection against overflow/underflow.
				let mut span_tc = [0, 0];
				let mut span_d_tc = [0, 0];
				for i in 0 .. 2
				{
					let tc_max_shift = INV_Z_SHIFT - TC_SHIFT;
					let tc_max_max = 1 << 29;
					// TODO - prove that such tc_max values are correct in all possible situations.
					let span_inv_z_corrected = (span_inv_z - (1 << INV_Z_PRE_SHIFT)) as i64;
					let tc_max_start = std::cmp::max(
						0,
						std::cmp::min(
							(span_inv_z_corrected * (texture_info.size[i] as i64) >> tc_max_shift) - 1,
							tc_max_max,
						),
					);
					let span_tc_start =
						std::cmp::max(0, std::cmp::min(span_start_x * d_tc_dx[i] + line_tc[i], tc_max_start)) as i32;

					if span_length_minus_one > 0
					{
						let tc_max_end = std::cmp::max(
							0,
							std::cmp::min(
								((span_inv_z_corrected + ((span_d_inv_z * span_length_minus_one) as i64)) *
									(texture_info.size[i] as i64) >> tc_max_shift) -
									1,
								tc_max_max,
							),
						);
						let span_tc_end =
							std::cmp::max(0, std::cmp::min(span_end_x * d_tc_dx[i] + line_tc[i], tc_max_end)) as i32;

						// We need to make sure than tc still is in range even if step is rounded.
						span_d_tc[i] = (span_tc_end - span_tc_start) / span_length_minus_one;
						if span_tc_end > span_tc_start
						{
							span_tc[i] = span_tc_start;
						}
						else
						{
							span_tc[i] = span_tc_end - span_d_tc[i] * span_length_minus_one;
						}
						debug_assert!(span_tc[i] >= 0);
						debug_assert!(span_tc[i] <= span_tc_start);
						debug_assert!(span_tc[i] + span_d_tc[i] * span_length_minus_one >= 0);
						debug_assert!(span_tc[i] + span_d_tc[i] * span_length_minus_one <= span_tc_end);
					}
					else
					{
						span_tc[i] = span_tc_start;
					}
				}

				for dst_pixel in line_dst
				{
					debug_assert!(span_tc[0] >= 0);
					debug_assert!(span_tc[1] >= 0);
					debug_assert!(span_inv_z >= (1 << INV_Z_PRE_SHIFT));

					let z = unchecked_div(1 << Z_CALC_SHIFT, (span_inv_z as u32) >> INV_Z_PRE_SHIFT);
					let pix_tc = [
						(((z as u64) * (span_tc[0] as u64)) >> TC_FINAL_SHIFT) as u32,
						(((z as u64) * (span_tc[1] as u64)) >> TC_FINAL_SHIFT) as u32,
					];

					debug_assert!(pix_tc[0] <= texture_size_minus_one[0] as u32);
					debug_assert!(pix_tc[1] <= texture_size_minus_one[1] as u32);
					let texel_address = (pix_tc[0] + pix_tc[1] * texture_width) as usize;
					let texel = unchecked_texture_fetch(texture_data, texel_address);
					write_into_framebuffer::<ColorT, BLENDING_MODE>(dst_pixel, texel);

					span_inv_z += span_d_inv_z;
					span_tc[0] += span_d_tc[0];
					span_tc[1] += span_d_tc[1];
				} // for span pixels
			} // if span is non-empty

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			line_inv_z += d_inv_z_dy;
			line_tc[0] += d_tc_dy[0];
			line_tc[1] += d_tc_dy[1];
		} // for lines
	}

	fn fill_polygon_part_line_z_corrected<const BLENDING_MODE: usize>(
		&mut self,
		y_start: Fixed16,
		y_end: Fixed16,
		left_side: PolygonSide,
		right_side: PolygonSide,
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[ColorT],
	)
	{
		debug_assert!(texture_data.len() >= (texture_info.size[0] * texture_info.size[1]) as usize);

		const LINE_TC_SHIFT: i64 = 16;

		let d_inv_z_dx = unchecked_to_int64(INV_Z_SCALE * depth_equation.d_inv_z_dx);
		let d_inv_z_dy = unchecked_to_int64(INV_Z_SCALE * depth_equation.d_inv_z_dy);
		// Add extra 0.5 to shift to pixel center.
		let inv_z_k = unchecked_to_int64(
			INV_Z_SCALE * (depth_equation.k + (depth_equation.d_inv_z_dx + depth_equation.d_inv_z_dy) * 0.5),
		);
		let d_tc_dx = [
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dx[0]),
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dx[1]),
		];
		let d_tc_dy = [
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dy[0]),
			unchecked_to_int64(TC_SCALE * tex_coord_equation.d_tc_dy[1]),
		];
		let tc_k = [
			unchecked_to_int64(
				TC_SCALE *
					(tex_coord_equation.k[0] + (tex_coord_equation.d_tc_dx[0] + tex_coord_equation.d_tc_dy[0]) * 0.5),
			),
			unchecked_to_int64(
				TC_SCALE *
					(tex_coord_equation.k[1] + (tex_coord_equation.d_tc_dx[1] + tex_coord_equation.d_tc_dy[1]) * 0.5),
			),
		];

		let texture_width = texture_info.size[0] as u32;

		// TODO - avoid adding "0.5" for some calculations.
		let y_start_int = fixed16_round_to_int(y_start).max(self.clip_rect.min_y);
		let y_end_int = fixed16_round_to_int(y_end).min(self.clip_rect.max_y);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		let mut line_inv_z = (y_start_int as i64) * d_inv_z_dy + inv_z_k;
		let mut line_tc = [
			(y_start_int as i64) * d_tc_dy[0] + tc_k[0],
			(y_start_int as i64) * d_tc_dy[1] + tc_k[1],
		];

		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(self.clip_rect.min_x);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.clip_rect.max_x);
			if x_start_int < x_end_int
			{
				let line_buffer_offset = y_int * self.row_size;
				let line_dst = unchecked_slice_range_mut(
					&mut self.color_buffer,
					(x_start_int + line_buffer_offset) as usize,
					(x_end_int + line_buffer_offset) as usize,
				);

				// Calculate z for span start/end, calculate texture coordinates based on this z.
				// Then just use liner interpolation of texture coordinates across span.
				// This is fine if z variation across span is very small.

				let span_start_x = x_start_int as i64;
				let span_end_x = (x_end_int - 1) as i64;
				let span_length_minus_one = (x_end_int - 1) - x_start_int;

				let inv_z_min = 1 << INV_Z_PRE_SHIFT;
				let inv_z_max = 1 << 29;
				let span_inv_z_start = (span_start_x * d_inv_z_dx + line_inv_z).max(inv_z_min).min(inv_z_max) as u32;
				let span_inv_z_end = (span_end_x * d_inv_z_dx + line_inv_z).max(inv_z_min).min(inv_z_max) as u32;
				let span_z_start = unchecked_div(1 << Z_CALC_SHIFT, span_inv_z_start >> INV_Z_PRE_SHIFT) as u64;
				let span_z_end = unchecked_div(1 << Z_CALC_SHIFT, span_inv_z_end >> INV_Z_PRE_SHIFT) as u64;

				// Perform interpolation of textures coordinates inside span using 32-bit values.
				let mut span_tc = [0, 0];
				let mut span_d_tc = [0, 0];
				for i in 0 .. 2
				{
					let tc_start = (line_tc[i] + span_start_x * d_tc_dx[i]).max(0) as u64;
					let tc_end = (line_tc[i] + span_end_x * d_tc_dx[i]).max(0) as u64;
					let max_tc = ((texture_info.size[i] as u64) << LINE_TC_SHIFT) - 1;
					let tc_mul_z_start =
						((span_z_start * tc_start) >> (TC_FINAL_SHIFT - LINE_TC_SHIFT)).min(max_tc) as i32;
					let tc_mul_z_end = ((span_z_end * tc_end) >> (TC_FINAL_SHIFT - LINE_TC_SHIFT)).min(max_tc) as i32;

					span_tc[i] = tc_mul_z_start;
					if span_length_minus_one > 0
					{
						span_d_tc[i] = (tc_mul_z_end - tc_mul_z_start) / span_length_minus_one;
					}
				}

				for dst_pixel in line_dst
				{
					let tc_int = [span_tc[0] >> LINE_TC_SHIFT, span_tc[1] >> LINE_TC_SHIFT];
					debug_assert!(tc_int[0] >= 0);
					debug_assert!(tc_int[1] >= 0);
					debug_assert!(tc_int[0] < texture_info.size[0] as i32);
					debug_assert!(tc_int[1] < texture_info.size[1] as i32);

					let texel_address = ((tc_int[0] as u32) + (tc_int[1] as u32) * texture_width) as usize;
					let texel = unchecked_texture_fetch(texture_data, texel_address);
					write_into_framebuffer::<ColorT, BLENDING_MODE>(dst_pixel, texel);

					span_tc[0] += span_d_tc[0];
					span_tc[1] += span_d_tc[1];
				} // for span pixels
			} // if span is non-empty

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			line_inv_z += d_inv_z_dy;
			line_tc[0] += d_tc_dy[0];
			line_tc[1] += d_tc_dy[1];
		} // for lines
	}

	fn fill_polygon_part_affine<const BLENDING_MODE: usize>(
		&mut self,
		y_start: Fixed16,
		y_end: Fixed16,
		left_side: PolygonSide,
		right_side: PolygonSide,
		depth_equation: &DepthEquation,
		tex_coord_equation: &TexCoordEquation,
		texture_info: &TextureInfo,
		texture_data: &[ColorT],
	)
	{
		debug_assert!(texture_data.len() >= (texture_info.size[0] * texture_info.size[1]) as usize);

		let y_start_f32 = fixed16_to_f32(y_start);
		let y_end_f32 = fixed16_to_f32(y_end);
		let y_delta = y_end - y_start;
		let x_start_left = fixed16_to_f32(left_side.x_start);
		let x_end_left = fixed16_to_f32(left_side.x_start + fixed16_mul(y_delta, left_side.dx_dy));
		let x_start_right = fixed16_to_f32(right_side.x_start);
		let x_end_right = fixed16_to_f32(right_side.x_start + fixed16_mul(y_delta, right_side.dx_dy));
		let z_start_left = 1.0 /
			(depth_equation.d_inv_z_dx * x_start_left + depth_equation.d_inv_z_dy * y_start_f32 + depth_equation.k);
		let z_end_left =
			1.0 / (depth_equation.d_inv_z_dx * x_end_left + depth_equation.d_inv_z_dy * y_end_f32 + depth_equation.k);
		let z_start_right = 1.0 /
			(depth_equation.d_inv_z_dx * x_start_right + depth_equation.d_inv_z_dy * y_start_f32 + depth_equation.k);
		let z_end_right =
			1.0 / (depth_equation.d_inv_z_dx * x_end_right + depth_equation.d_inv_z_dy * y_end_f32 + depth_equation.k);

		// Prevent division by zero or overflow.
		let y_delta_for_tc_interpoltion = y_delta.max(FIXED16_ONE);

		let mut tc_left = [0, 0];
		let mut tc_right = [0, 0];
		let mut d_tc_left = [0, 0];
		let mut d_tc_right = [0, 0];
		for i in 0 .. 2
		{
			let max_tc = int_to_fixed16(texture_info.size[i]) - 1;

			let tc_start_left = unsafe {
				f32_to_fixed16_unchecked(
					z_start_left *
						(x_start_left * tex_coord_equation.d_tc_dx[i] +
							y_start_f32 * tex_coord_equation.d_tc_dy[i] +
							tex_coord_equation.k[i]),
				)
				.max(0)
				.min(max_tc)
			};
			let tc_end_left = unsafe {
				f32_to_fixed16_unchecked(
					z_end_left *
						(x_end_left * tex_coord_equation.d_tc_dx[i] +
							y_end_f32 * tex_coord_equation.d_tc_dy[i] +
							tex_coord_equation.k[i]),
				)
				.max(0)
				.min(max_tc)
			};

			let tc_start_right = unsafe {
				f32_to_fixed16_unchecked(
					z_start_right *
						(x_start_right * tex_coord_equation.d_tc_dx[i] +
							y_start_f32 * tex_coord_equation.d_tc_dy[i] +
							tex_coord_equation.k[i]),
				)
				.max(0)
				.min(max_tc)
			};
			let tc_end_right = unsafe {
				f32_to_fixed16_unchecked(
					z_end_right *
						(x_end_right * tex_coord_equation.d_tc_dx[i] +
							y_end_f32 * tex_coord_equation.d_tc_dy[i] +
							tex_coord_equation.k[i]),
				)
				.max(0)
				.min(max_tc)
			};

			d_tc_left[i] = fixed16_div(tc_end_left - tc_start_left, y_delta_for_tc_interpoltion);
			d_tc_right[i] = fixed16_div(tc_end_right - tc_start_right, y_delta_for_tc_interpoltion);
			tc_left[i] = tc_start_left;
			tc_right[i] = tc_start_right;
		}

		// TODO - avoid adding "0.5" for some calculations.
		let y_start_int = fixed16_round_to_int(y_start).max(self.clip_rect.min_y);
		let y_end_int = fixed16_round_to_int(y_end).min(self.clip_rect.max_y);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		for i in 0 .. 2
		{
			tc_left[i] += fixed16_mul(y_start_delta, d_tc_left[i]);
			tc_right[i] += fixed16_mul(y_start_delta, d_tc_right[i]);
		}

		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(self.clip_rect.min_x);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.clip_rect.max_x);
			if x_start_int < x_end_int
			{
				let line_buffer_offset = y_int * self.row_size;
				let line_dst = unchecked_slice_range_mut(
					&mut self.color_buffer,
					(x_start_int + line_buffer_offset) as usize,
					(x_end_int + line_buffer_offset) as usize,
				);

				let mut tc = [tc_left[0], tc_left[1]];
				// Prevent division by zero or overflow.
				let x_delta_for_tc_interpolation = (x_right - x_left).max(FIXED16_ONE);
				let d_tc = [
					fixed16_div(tc_right[0] - tc_left[0], x_delta_for_tc_interpolation),
					fixed16_div(tc_right[1] - tc_left[1], x_delta_for_tc_interpolation),
				];

				for dst_pixel in line_dst
				{
					// TODO - use unsigned shift for conversion to int?
					let tc_int = [fixed16_floor_to_int(tc[0]), fixed16_floor_to_int(tc[1])];
					debug_assert!(tc_int[0] >= 0);
					debug_assert!(tc_int[1] >= 0);
					debug_assert!(tc_int[0] < texture_info.size[0] as i32);
					debug_assert!(tc_int[1] < texture_info.size[1] as i32);
					let texel_address = (tc_int[0] + tc_int[1] * texture_info.size[0]) as usize;
					let texel = unchecked_texture_fetch(texture_data, texel_address);
					write_into_framebuffer::<ColorT, BLENDING_MODE>(dst_pixel, texel);

					tc[0] += d_tc[0];
					tc[1] += d_tc[1];
				} // for span pixels
			} // if span is non-empty

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			tc_left[0] += d_tc_left[0];
			tc_left[1] += d_tc_left[1];
			tc_right[0] += d_tc_right[0];
			tc_right[1] += d_tc_right[1];
		} // for lines
	}

	pub fn fill_triangle<TextureColorT: AbstractColor>(
		&mut self,
		vertices: &[TrianglePointProjected; 3],
		texture_info: &TextureInfo,
		texture_data: &[TextureColorT],
		blending_mode: BlendingMode,
	)
	{
		match blending_mode
		{
			BlendingMode::None =>
			{
				self.fill_triangle_impl::<TextureColorT, BLENDING_MODE_NONE>(vertices, texture_info, texture_data)
			},
			BlendingMode::Average =>
			{
				self.fill_triangle_impl::<TextureColorT, BLENDING_MODE_AVERAGE>(vertices, texture_info, texture_data)
			},
			BlendingMode::Additive =>
			{
				self.fill_triangle_impl::<TextureColorT, BLENDING_MODE_ADDITIVE>(vertices, texture_info, texture_data)
			},
			BlendingMode::AlphaTest =>
			{
				self.fill_triangle_impl::<TextureColorT, BLENDING_MODE_ALPHA_TEST>(vertices, texture_info, texture_data)
			},
			BlendingMode::AlphaBlend => self.fill_triangle_impl::<TextureColorT, BLENDING_MODE_ALPHA_BLEND>(
				vertices,
				texture_info,
				texture_data,
			),
		}
	}

	pub fn fill_triangle_impl<TextureColorT: AbstractColor, const BLENDING_MODE: usize>(
		&mut self,
		vertices: &[TrianglePointProjected; 3],
		texture_info: &TextureInfo,
		texture_data: &[TextureColorT],
	)
	{
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
		middle_index = 3 - upper_index - lower_index;

		let upper_vertex = &vertices[upper_index];
		let lower_vertex = &vertices[lower_index];
		let middle_vertex = &vertices[middle_index];

		// Use hack with miminun dy to avoid division by zero and overflows.
		let long_edge_dy = (upper_vertex.y - lower_vertex.y).max(FIXED16_HALF);
		let lower_part_dy = (middle_vertex.y - lower_vertex.y).max(FIXED16_HALF);
		let upper_part_dy = (upper_vertex.y - middle_vertex.y).max(FIXED16_HALF);

		let long_edge_dx_dy = fixed16_div(upper_vertex.x - lower_vertex.x, long_edge_dy);
		let long_edge_x_in_middle = lower_vertex.x + fixed16_mul(long_edge_dx_dy, middle_vertex.y - lower_vertex.y);

		let lower_part_dx_dy = fixed16_div(middle_vertex.x - lower_vertex.x, lower_part_dy);
		let upper_part_dx_dy = fixed16_div(upper_vertex.x - middle_vertex.x, upper_part_dy);

		let mut long_edge_d_tc_dy = [0, 0];
		let mut long_edge_tc_in_middle = [0, 0];
		let mut d_tc_dy_lower = [0, 0];
		let mut d_tc_dy_upper = [0, 0];
		for i in 0 .. 2
		{
			long_edge_d_tc_dy[i] = fixed16_div(upper_vertex.tc[i] - lower_vertex.tc[i], long_edge_dy);
			long_edge_tc_in_middle[i] = lower_vertex.tc[i] + fixed16_mul(long_edge_d_tc_dy[i], lower_part_dy);

			d_tc_dy_lower[i] = fixed16_div(middle_vertex.tc[i] - lower_vertex.tc[i], lower_part_dy);
			d_tc_dy_upper[i] = fixed16_div(upper_vertex.tc[i] - middle_vertex.tc[i], upper_part_dy);
		}

		let mut long_edge_d_light_dy = [0, 0, 0];
		let mut long_edge_light_in_middle = [0, 0, 0];
		let mut d_light_dy_lower = [0, 0, 0];
		let mut d_light_dy_upper = [0, 0, 0];
		for i in 0 .. 3
		{
			long_edge_d_light_dy[i] = fixed16_div(upper_vertex.light[i] - lower_vertex.light[i], long_edge_dy);
			long_edge_light_in_middle[i] = lower_vertex.light[i] + fixed16_mul(long_edge_d_light_dy[i], lower_part_dy);

			d_light_dy_lower[i] = fixed16_div(middle_vertex.light[i] - lower_vertex.light[i], lower_part_dy);
			d_light_dy_upper[i] = fixed16_div(upper_vertex.light[i] - middle_vertex.light[i], upper_part_dy);
		}

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

			let middle_dx = (long_edge_x_in_middle - middle_vertex.x).max(FIXED16_HALF);

			let mut d_tc_dx = [0, 0];
			for i in 0 .. 2
			{
				d_tc_dx[i] = fixed16_div(long_edge_tc_in_middle[i] - middle_vertex.tc[i], middle_dx);
			}

			let mut d_light_dx = [0, 0, 0];
			for i in 0 .. 3
			{
				d_light_dx[i] = fixed16_div(long_edge_light_in_middle[i] - middle_vertex.light[i], middle_dx);
			}

			self.fill_triangle_part::<TextureColorT, BLENDING_MODE>(
				lower_vertex.y,
				middle_vertex.y,
				PolygonSide {
					x_start: lower_vertex.x,
					dx_dy: lower_part_dx_dy,
				},
				PolygonSide {
					x_start: lower_vertex.x,
					dx_dy: long_edge_dx_dy,
				},
				lower_vertex.tc,
				d_tc_dy_lower,
				d_tc_dx,
				lower_vertex.light,
				d_light_dy_lower,
				d_light_dx,
				texture_info,
				texture_data,
			);
			self.fill_triangle_part::<TextureColorT, BLENDING_MODE>(
				middle_vertex.y,
				upper_vertex.y,
				PolygonSide {
					x_start: middle_vertex.x,
					dx_dy: upper_part_dx_dy,
				},
				PolygonSide {
					x_start: long_edge_x_in_middle,
					dx_dy: long_edge_dx_dy,
				},
				middle_vertex.tc,
				d_tc_dy_upper,
				d_tc_dx,
				middle_vertex.light,
				d_light_dy_upper,
				d_light_dx,
				texture_info,
				texture_data,
			);
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

			let middle_dx = (middle_vertex.x - long_edge_x_in_middle).max(FIXED16_HALF);

			let mut d_tc_dx = [0, 0];
			for i in 0 .. 2
			{
				d_tc_dx[i] = fixed16_div(middle_vertex.tc[i] - long_edge_tc_in_middle[i], middle_dx);
			}

			let mut d_light_dx = [0, 0, 0];
			for i in 0 .. 3
			{
				d_light_dx[i] = fixed16_div(middle_vertex.light[i] - long_edge_light_in_middle[i], middle_dx);
			}

			self.fill_triangle_part::<TextureColorT, BLENDING_MODE>(
				lower_vertex.y,
				middle_vertex.y,
				PolygonSide {
					x_start: lower_vertex.x,
					dx_dy: long_edge_dx_dy,
				},
				PolygonSide {
					x_start: lower_vertex.x,
					dx_dy: lower_part_dx_dy,
				},
				lower_vertex.tc,
				long_edge_d_tc_dy,
				d_tc_dx,
				lower_vertex.light,
				long_edge_d_light_dy,
				d_light_dx,
				texture_info,
				texture_data,
			);
			self.fill_triangle_part::<TextureColorT, BLENDING_MODE>(
				middle_vertex.y,
				upper_vertex.y,
				PolygonSide {
					x_start: long_edge_x_in_middle,
					dx_dy: long_edge_dx_dy,
				},
				PolygonSide {
					x_start: middle_vertex.x,
					dx_dy: upper_part_dx_dy,
				},
				long_edge_tc_in_middle,
				long_edge_d_tc_dy,
				d_tc_dx,
				long_edge_light_in_middle,
				long_edge_d_light_dy,
				d_light_dx,
				texture_info,
				texture_data,
			);
		}
	}

	fn fill_triangle_part<TextureColorT: AbstractColor, const BLENDING_MODE: usize>(
		&mut self,
		y_start: Fixed16,
		y_end: Fixed16,
		left_side: PolygonSide,
		right_side: PolygonSide,
		tc_start_left: [Fixed16; 2],
		mut d_tc_dy_left: [Fixed16; 2],
		mut d_tc_dx: [Fixed16; 2],
		light_start_left: [Fixed16; 3],
		mut d_light_left: [Fixed16; 3],
		mut d_light_dx: [Fixed16; 3],
		texture_info: &TextureInfo,
		texture_data: &[TextureColorT],
	)
	{
		let y_start_int = fixed16_round_to_int(y_start).max(self.clip_rect.min_y);
		let y_end_int = fixed16_round_to_int(y_end).min(self.clip_rect.max_y);
		if y_start_int >= y_end_int
		{
			return;
		}

		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		// Add 0.5 to simplify rounding in inner loop.
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;

		let y_end_delta = y_end - (y_start + y_start_delta);
		let d_x_lower = (x_right - x_left).max(0);
		let x_upper_left = x_left + fixed16_mul(y_end_delta, left_side.dx_dy);
		let x_upper_right = x_right + fixed16_mul(y_end_delta, right_side.dx_dy);
		let d_x_upper = (x_upper_right - x_upper_left).max(0);

		// Correct tc equation to fix possible out of borders coordinates.
		let mut tc_left = [0, 0];
		for i in 0 .. 2
		{
			let max_tc = int_to_fixed16(texture_info.size[i] as i32) - 1;

			// Try to avoid d_tc_dx correction, prefer correction of left side equation.

			{
				tc_left[i] = (tc_start_left[i] + fixed16_mul(y_start_delta, d_tc_dy_left[i]))
					.max(0)
					.min(max_tc);
				let tc_lower_delta = fixed16_mul(d_tc_dx[i], d_x_lower);
				if tc_lower_delta > max_tc
				{
					tc_left[i] = 0;
					d_tc_dx[i] = fixed16_div(max_tc, d_x_lower);
				}
				else if tc_lower_delta < -max_tc
				{
					tc_left[i] = max_tc;
					d_tc_dx[i] = fixed16_div(-max_tc, d_x_lower);
				}
				else
				{
					let tc_lower_right = tc_left[i] + tc_lower_delta;
					if tc_lower_right > max_tc
					{
						tc_left[i] -= tc_lower_right - max_tc;
					}
					else if tc_lower_right < 0
					{
						tc_left[i] -= tc_lower_right;
					}
				}
			}
			debug_assert!(tc_left[i] >= 0);
			debug_assert!(tc_left[i] <= max_tc);
			debug_assert!(tc_left[i] + fixed16_mul(d_tc_dx[i], d_x_lower) >= 0);
			debug_assert!(tc_left[i] + fixed16_mul(d_tc_dx[i], d_x_lower) <= max_tc);

			{
				let mut tc_upper_left = (tc_left[i] + fixed16_mul(y_end_delta, d_tc_dy_left[i]))
					.max(0)
					.min(max_tc);
				let tc_upper_delta = fixed16_mul(d_tc_dx[i], d_x_upper);
				if tc_upper_delta > max_tc
				{
					tc_upper_left = 0;
					d_tc_dx[i] = fixed16_div(max_tc, d_x_upper);
				}
				else if tc_upper_delta < -max_tc
				{
					tc_upper_left = max_tc;
					d_tc_dx[i] = fixed16_div(-max_tc, d_x_upper);
				}
				else
				{
					let tc_upper_right = tc_upper_left + tc_upper_delta;
					if tc_upper_right > max_tc
					{
						tc_upper_left -= tc_upper_right - max_tc;
					}
					else if tc_upper_right < 0
					{
						tc_upper_left -= tc_upper_right;
					}
				}
				debug_assert!(tc_upper_left >= 0);
				debug_assert!(tc_upper_left <= max_tc);
				if y_end_delta > 0
				{
					d_tc_dy_left[i] = fixed16_div(tc_upper_left - tc_left[i], y_end_delta);
				}
				else
				{
					d_tc_dy_left[i] = 0;
				}

				// Perform final d_tc_dx correction if coordinates are still out of borders after correction d_tc_dy_left.
				let tc_upper_left = tc_left[i] + fixed16_mul(y_end_delta, d_tc_dy_left[i]);
				let tc_upper_delta = fixed16_mul(d_tc_dx[i], d_x_upper);
				let tc_upper_right = tc_upper_left + tc_upper_delta;
				if tc_upper_right < 0
				{
					if d_x_upper > 0
					{
						d_tc_dx[i] = fixed16_div(-tc_upper_left, d_x_upper);
					}
					else
					{
						d_tc_dx[i] = 0;
					}
				}
				else if tc_upper_right > max_tc
				{
					if d_x_upper > 0
					{
						d_tc_dx[i] = fixed16_div(max_tc - tc_upper_left, d_x_upper);
					}
					else
					{
						d_tc_dx[i] = 0;
					}
				}
			}

			debug_assert!(tc_left[i] >= 0);
			debug_assert!(tc_left[i] <= max_tc);
			debug_assert!(tc_left[i] + fixed16_mul(d_tc_dx[i], d_x_lower) >= 0);
			debug_assert!(tc_left[i] + fixed16_mul(d_tc_dx[i], d_x_lower) <= max_tc);

			debug_assert!(tc_left[i] + fixed16_mul(y_end_delta, d_tc_dy_left[i]) >= 0);
			debug_assert!(tc_left[i] + fixed16_mul(y_end_delta, d_tc_dy_left[i]) <= max_tc);
			debug_assert!(
				tc_left[i] + fixed16_mul(y_end_delta, d_tc_dy_left[i]) + fixed16_mul(d_tc_dx[i], d_x_upper) >= 0
			);
			debug_assert!(
				tc_left[i] + fixed16_mul(y_end_delta, d_tc_dy_left[i]) + fixed16_mul(d_tc_dx[i], d_x_upper) <= max_tc
			);
		}

		// Correct light equation to fix possible underflow.
		// TODO - prevent overflow too?
		let mut light_left = [0, 0, 0];
		for i in 0 .. 3
		{
			light_left[i] = (light_start_left[i] + fixed16_mul(y_start_delta, d_light_left[i])).max(0);
			let light_lower_right = light_left[i] + fixed16_mul(d_x_lower, d_light_dx[i]);
			if light_lower_right < 0
			{
				light_left[i] -= light_lower_right;
			}
			debug_assert!(light_left[i] >= 0);
			debug_assert!(light_left[i] + fixed16_mul(d_x_lower, d_light_dx[i]) >= 0);

			{
				let mut light_upper_left = (light_left[i] + fixed16_mul(y_end_delta, d_light_left[i])).max(0);
				let light_upper_right = light_upper_left + fixed16_mul(d_x_upper, d_light_dx[i]);
				if light_upper_right < 0
				{
					light_upper_left -= light_upper_right;
				}
				debug_assert!(light_upper_left >= 0);
				if y_end_delta > 0
				{
					d_light_left[i] = fixed16_div(light_upper_left - light_left[i], y_end_delta);
				}
				else
				{
					d_light_left[i] = 0;
				}

				// Perform final d_light_dx correction if light is still may be negative after correction d_light_left.
				let light_upper_left = light_left[i] + fixed16_mul(y_end_delta, d_light_left[i]);
				let light_upper_right = light_upper_left + fixed16_mul(d_x_upper, d_light_dx[i]);
				if light_upper_right < 0
				{
					if d_x_upper > 0
					{
						d_light_dx[i] = fixed16_div(-light_upper_left, d_x_upper);
					}
					else
					{
						d_light_dx[i] = 0;
					}
				}
			}
			debug_assert!(light_left[i] >= 0);
			debug_assert!(light_left[i] + fixed16_mul(d_x_lower, d_light_dx[i]) >= 0);
			debug_assert!(light_left[i] + fixed16_mul(y_end_delta, d_light_left[i]) >= 0);
			debug_assert!(
				light_left[i] + fixed16_mul(y_end_delta, d_light_left[i]) + fixed16_mul(d_x_upper, d_light_dx[i]) >= 0
			);
		}

		let d_light_dx_vec = ColorVecI::from_color_i32x3(&d_light_dx);

		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(self.clip_rect.min_x);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.clip_rect.max_x);
			if x_start_int < x_end_int
			{
				let x_start_delta = int_to_fixed16(x_start_int) + FIXED16_ONE - x_left;
				debug_assert!(x_start_delta >= 0);

				let mut line_tc = [
					tc_left[0] + fixed16_mul(x_start_delta, d_tc_dx[0]),
					tc_left[1] + fixed16_mul(x_start_delta, d_tc_dx[1]),
				];

				let mut line_light = ColorVecI::from_color_i32x3(&[
					light_left[0] + fixed16_mul(x_start_delta, d_light_dx[0]),
					light_left[1] + fixed16_mul(x_start_delta, d_light_dx[1]),
					light_left[2] + fixed16_mul(x_start_delta, d_light_dx[2]),
				]);

				let line_buffer_offset = y_int * self.row_size;
				let line_dst = unchecked_slice_range_mut(
					&mut self.color_buffer,
					(x_start_int + line_buffer_offset) as usize,
					(x_end_int + line_buffer_offset) as usize,
				);

				for dst_pixel in line_dst
				{
					debug_assert!(line_light.extract::<0>() >= 0);
					debug_assert!(line_light.extract::<1>() >= 0);
					debug_assert!(line_light.extract::<2>() >= 0);

					let u = fixed16_floor_to_int(line_tc[0]);
					let v = fixed16_floor_to_int(line_tc[1]);
					debug_assert!(u >= 0);
					debug_assert!(u < texture_info.size[0]);
					debug_assert!(v >= 0);
					debug_assert!(v < texture_info.size[1]);

					let texel_address = (u + v * texture_info.size[0]) as usize;
					let texel = unchecked_texture_fetch(texture_data, texel_address);

					let texel_vec = texel.into();
					let texel_vec_lighted = ColorVecI::shift_right::<16>(&ColorVecI::mul(&texel_vec, &line_light));

					// TODO - fix this, remove unnecessary conversions of ColorVecI
					if BLENDING_MODE == BLENDING_MODE_NONE
					{
						*dst_pixel = texel_vec_lighted.into();
					}
					else if BLENDING_MODE == BLENDING_MODE_AVERAGE
					{
						*dst_pixel = ColorT::average(*dst_pixel, texel_vec_lighted.into());
					}
					else if BLENDING_MODE == BLENDING_MODE_ADDITIVE
					{
						*dst_pixel = ColorT::saturated_sum(*dst_pixel, texel_vec_lighted.into());
					}
					else if BLENDING_MODE == BLENDING_MODE_ALPHA_TEST
					{
						let texel_converted: ColorT = texel_vec_lighted.into();
						if texel_converted.test_alpha()
						{
							*dst_pixel = texel_converted;
						}
					}
					else if BLENDING_MODE == BLENDING_MODE_ALPHA_BLEND
					{
						let alpha = texel.get_alpha();
						let dst_vec: ColorVecI = (*dst_pixel).into();
						let blend_result = ColorVecI::shift_right::<8>(&ColorVecI::add(
							&ColorVecI::mul_scalar(&dst_vec, 255 - alpha),
							&ColorVecI::mul_scalar(&texel_vec_lighted, alpha),
						));
						*dst_pixel = blend_result.into();
					}

					for i in 0 .. 2
					{
						line_tc[i] += d_tc_dx[i];
					}
					line_light = ColorVecI::add(&line_light, &d_light_dx_vec);
				}
			}
			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			for i in 0 .. 2
			{
				tc_left[i] += d_tc_dy_left[i];
			}
			for i in 0 .. 3
			{
				light_left[i] += d_light_left[i];
			}
		}
	}
}

pub struct DepthRasterizer<'a>
{
	depth_buffer: &'a mut [f32],
	width: i32,
	height: i32,
}

impl<'a> DepthRasterizer<'a>
{
	pub fn new(depth_buffer: &'a mut [f32], width: u32, height: u32) -> Self
	{
		Self {
			depth_buffer,
			width: width as i32,
			height: height as i32,
		}
	}

	// Fill convex clockwise polygon.
	pub fn fill_polygon(&mut self, vertices: &[PolygonPointProjected], depth_equation: &DepthEquation)
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

			// TODO - use unchecked fetch?
			let left_vertex = &vertices[left_index];
			let next_left_vertex = &vertices[next_left_index];
			let right_vertex = &vertices[right_index];
			let next_right_vertex = &vertices[next_right_index];

			let dy_left = next_left_vertex.y - left_vertex.y;
			let dy_right = next_right_vertex.y - right_vertex.y;
			let next_y = std::cmp::min(next_left_vertex.y, next_right_vertex.y);
			if dy_left > FIXED16_HALF && dy_right > FIXED16_HALF
			{
				let dx_dy_left = fixed16_div(next_left_vertex.x - left_vertex.x, dy_left);
				let dx_dy_right = fixed16_div(next_right_vertex.x - right_vertex.x, dy_right);
				self.fill_polygon_part(
					cur_y,
					next_y,
					PolygonSide {
						x_start: left_vertex.x + fixed16_mul(dx_dy_left, cur_y - left_vertex.y),
						dx_dy: dx_dy_left,
					},
					PolygonSide {
						x_start: right_vertex.x + fixed16_mul(dx_dy_right, cur_y - right_vertex.y),
						dx_dy: dx_dy_right,
					},
					depth_equation,
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
					let x_start_left = left_vertex.x +
						fixed16_mul_div(thin_line_y - left_vertex.y, next_left_vertex.x - left_vertex.x, dy_left);
					let x_start_right = right_vertex.x +
						fixed16_mul_div(
							thin_line_y - right_vertex.y,
							next_right_vertex.x - right_vertex.x,
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
	)
	{
		// TODO - avoid adding "0.5" for some calculations.
		let y_start_int = fixed16_round_to_int(y_start).max(0);
		let y_end_int = fixed16_round_to_int(y_end).min(self.height);
		let y_start_delta = int_to_fixed16(y_start_int) + FIXED16_HALF - y_start;
		let mut x_left = left_side.x_start + fixed16_mul(y_start_delta, left_side.dx_dy) + FIXED16_HALF;
		let mut x_right = right_side.x_start + fixed16_mul(y_start_delta, right_side.dx_dy) + FIXED16_HALF;
		let mut line_inv_z = (y_start_int as f32) * depth_equation.d_inv_z_dy +
			0.5 * (depth_equation.d_inv_z_dx + depth_equation.d_inv_z_dy) +
			depth_equation.k;

		for y_int in y_start_int .. y_end_int
		{
			let x_start_int = fixed16_floor_to_int(x_left).max(0);
			let x_end_int = fixed16_floor_to_int(x_right).min(self.width);
			if x_start_int < x_end_int
			{
				let line_buffer_offset = y_int * self.width;
				let line_dst = &mut self.depth_buffer
					[(x_start_int + line_buffer_offset) as usize .. (x_end_int + line_buffer_offset) as usize];

				let mut inv_z = line_inv_z + (x_start_int as f32) * depth_equation.d_inv_z_dx;
				for dst_pixel in line_dst
				{
					*dst_pixel = inv_z;
					inv_z += depth_equation.d_inv_z_dx;
				} // for span pixels
			} // if span is non-empty

			x_left += left_side.dx_dy;
			x_right += right_side.dx_dy;
			line_inv_z += depth_equation.d_inv_z_dy;
		} // for lines
	}
}

pub struct ClipRect
{
	pub min_x: i32,
	pub min_y: i32,
	pub max_x: i32,
	pub max_y: i32,
}

#[derive(Copy, Clone)]
pub struct PolygonPointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
}

#[derive(Copy, Clone)]
pub struct TrianglePointProjected
{
	pub x: Fixed16,
	pub y: Fixed16,
	pub tc: [Fixed16; 2],
	pub light: [Fixed16; 3],
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

fn write_into_framebuffer<ColorT: AbstractColor, const BLENDING_MODE: usize>(dst_pixel: &mut ColorT, texel: ColorT)
{
	if BLENDING_MODE == BLENDING_MODE_NONE
	{
		*dst_pixel = texel;
	}
	else if BLENDING_MODE == BLENDING_MODE_AVERAGE
	{
		*dst_pixel = ColorT::average(*dst_pixel, texel);
	}
	else if BLENDING_MODE == BLENDING_MODE_ADDITIVE
	{
		*dst_pixel = ColorT::saturated_sum(*dst_pixel, texel);
	}
	else if BLENDING_MODE == BLENDING_MODE_ALPHA_TEST
	{
		if texel.test_alpha()
		{
			*dst_pixel = texel;
		}
	}
	else if BLENDING_MODE == BLENDING_MODE_ALPHA_BLEND
	{
		*dst_pixel = ColorT::alpha_blend(*dst_pixel, texel);
	}
}

// We do not care if "y" is zero, because there is no difference between "panic!" and hardware exceptions.
// In both cases application will be terminated.
#[cfg(feature = "rasterizer_unchecked_div")]
fn unchecked_div(x: u32, y: u32) -> u32
{
	unsafe { std::intrinsics::unchecked_div(x, y) }
}

#[cfg(not(feature = "rasterizer_unchecked_div"))]
fn unchecked_div(x: u32, y: u32) -> u32
{
	x / y
}

fn unchecked_to_int64(x: f32) -> i64
{
	// Do not care about overflow, infinity, NaN. It is almost impossible.
	unsafe { x.to_int_unchecked::<i64>() }
}

fn unchecked_texture_fetch<ColorT: Copy>(texture_data: &[ColorT], texel_address: usize) -> ColorT
{
	// operator [] checks bounds and calls panic! handler in case if index is out of bounds.
	// This check is useless here since we clamp texture coordnates properly.
	// So, use "get_unchecked" in release mode.
	#[cfg(debug_assertions)]
	{
		texture_data[texel_address]
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		*texture_data.get_unchecked(texel_address)
	}
}

fn unchecked_slice_range_mut<T>(data: &mut [T], start: usize, end: usize) -> &mut [T]
{
	// operator [ .. ] checks bounds and calls panic! handler in case if index is out of bounds.
	// This check is useless here since we clamp properly.
	// So, use "get_unchecked_mut" in release mode.
	#[cfg(debug_assertions)]
	{
		&mut data[start .. end]
	}
	#[cfg(not(debug_assertions))]
	unsafe {
		data.get_unchecked_mut(start .. end)
	}
}

const TC_SHIFT: i32 = 24;
const INV_Z_SHIFT: i32 = 29;
const INV_Z_PRE_SHIFT: i32 = 8;
const Z_CALC_SHIFT: i32 = 31;
const TC_FINAL_SHIFT: i64 = (TC_SHIFT + Z_CALC_SHIFT - INV_Z_SHIFT + INV_Z_PRE_SHIFT) as i64;

const TC_SCALE: f32 = (1 << TC_SHIFT) as f32;
const INV_Z_SCALE: f32 = (1 << INV_Z_SHIFT) as f32;
