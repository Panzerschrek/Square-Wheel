use common::{color::*, image, plane::*, math_types::*};

pub fn build_surface(
	surface_size: [u32; 2],
	surface_tc_min: [i32; 2],
	texture: &image::Image,
	plane: &Plane,
	tex_coord_equation: &[Plane; 2],
	light_pos : &Vec3f,
	out_surface_data: &mut [Color32],
)
{
	// Calculate inverse matrix for tex_coord aquation and plane equation in order to calculate world position for UV.
	// TODO - project tc equation to surface plane?
	let tex_coord_basis =
		Mat4f::from_cols(
			tex_coord_equation[0].vec.extend(tex_coord_equation[0].dist),
			tex_coord_equation[1].vec.extend(tex_coord_equation[1].dist),
			plane.vec.extend(-plane.dist),
			Vec4f::new(0.0, 0.0, 0.0, 1.0));

	let tex_coord_basis_inverted = tex_coord_basis.invert().unwrap(); // TODO - avoid "unwrap"?
	let x_equation = tex_coord_basis_inverted.x;
	let y_equation = tex_coord_basis_inverted.y;
	let pos_shift = tex_coord_basis_inverted.z;

	let plane_normal_normalized = plane.vec / plane.vec.magnitude();
	
	let constant_light = [1.5, 1.4, 1.3];
	let light_power = [ 100000.0, 100000.0, 100000.0 ];

	for dst_v in 0 .. surface_size[1]
	{
		let dst_line_start = (dst_v * surface_size[0]) as usize;
		let dst_line = &mut out_surface_data[dst_line_start .. dst_line_start + (surface_size[0] as usize)];

		let src_v = (surface_tc_min[1] + (dst_v as i32)).rem_euclid(texture.size[1] as i32);
		let src_line_start = ((src_v as u32) * texture.size[0]) as usize;
		let src_line = &texture.pixels[src_line_start .. src_line_start + (texture.size[0] as usize)];
		let mut src_u = surface_tc_min[0].rem_euclid(texture.size[0] as i32);
		let mut dst_u = 0;
		for dst_texel in dst_line.iter_mut()
		{
			// TODO - optimize this.
			// TODO - shift to pixel center.
			let uv = Vec4f::new( ((dst_u as i32) + surface_tc_min[0]) as f32, ((dst_v as i32) + surface_tc_min[1]) as f32, 0.0, 1.0 );
			let pos = Vec3f::new(x_equation.dot(uv), y_equation.dot(uv), pos_shift.dot(uv));

			 let vec_to_light = light_pos - pos;
			 let vec_to_light_len2 = vec_to_light.magnitude2();
			 // TODO - use fast inverse square root.
			 let angle_cos = plane_normal_normalized.dot(vec_to_light) / vec_to_light_len2.sqrt();

			let light_scale = angle_cos.max(0.0) / vec_to_light_len2;


			let total_light = [ constant_light[0] + light_power[0] * light_scale, constant_light[1] + light_power[1] * light_scale, constant_light[02] + light_power[2] * light_scale ];
			
			let texel_value = src_line[src_u as usize];

			let components = texel_value.unpack_to_rgb_f32();
			let components_modulated = [
				(components[0] * total_light[0]).min(Color32::MAX_RGB_F32_COMPONENTS[0]),
				(components[1] * total_light[1]).min(Color32::MAX_RGB_F32_COMPONENTS[1]),
				(components[2] * total_light[2]).min(Color32::MAX_RGB_F32_COMPONENTS[2]),
			];

			// Here we 100% sure that components overflow is not possible (because of "min").
			// NaNs are not possible here too.
			let color_packed = unsafe { Color32::from_rgb_f32_unchecked(&components_modulated) };

			*dst_texel = color_packed;
			src_u += 1;
			if src_u == (texture.size[0] as i32)
			{
				src_u = 0;
			}

			dst_u += 1;
		}
	}
}
