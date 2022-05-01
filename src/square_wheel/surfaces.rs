use common::{color::*, image};

pub fn build_surface(
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &image::Image,
	out_surface_data: &mut [Color32],
)
{
	for dst_y in 0 .. surface_size[1]
	{
		let dst_line_start = (dst_y * surface_size[0]) as usize;
		let dst_line = &mut out_surface_data[dst_line_start .. dst_line_start + (surface_size[0] as usize)];

		let src_y = (surface_tc_min[1] + (dst_y as i32)).rem_euclid(texture.size[1] as i32);
		let src_line_start = ((src_y as u32) * texture.size[0]) as usize;
		let src_line = &texture.pixels[src_line_start .. src_line_start + (texture.size[0] as usize)];
		let mut src_x = surface_tc_min[0].rem_euclid(texture.size[0] as i32);
		for dst_x in 0 .. surface_size[0]
		{
			dst_line[dst_x as usize] = src_line[src_x as usize];
			src_x += 1;
			if src_x == (texture.size[0] as i32)
			{
				src_x = 0;
			}
		}
	}
}
