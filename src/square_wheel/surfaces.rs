use common::{color::*, image};

pub fn build_surface(
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &image::Image,
	out_surface_data: &mut [Color32],
)
{
	let light = [1.5, 1.4, 1.3];

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
			let texel_value = src_line[src_x as usize];

			let components = texel_value.unpack_to_rgb_f32();
			let components_modulated = [
				(components[0] * light[0]).min(Color32::MAX_RGB_F32_COMPONENTS[0]),
				(components[1] * light[1]).min(Color32::MAX_RGB_F32_COMPONENTS[1]),
				(components[2] * light[2]).min(Color32::MAX_RGB_F32_COMPONENTS[2]),
			];

			// Here we 100% sure that components overflow is not possible (because of "min").
			// NaNs are not possible here too.
			let color_packed = unsafe { Color32::from_rgb_f32_unchecked(&components_modulated) };

			dst_line[dst_x as usize] = color_packed;
			src_x += 1;
			if src_x == (texture.size[0] as i32)
			{
				src_x = 0;
			}
		}
	}
}
