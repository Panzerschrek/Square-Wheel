use common::{color::*, image, material, math_types::*};

// MAX_MIP must be not greater, than LIGHTMAP_SCALE_LOG2
pub const MAX_MIP: usize = 3;
pub const NUM_MIPS: usize = MAX_MIP + 1;
pub type TextureWithMips = [Texture; NUM_MIPS];

#[derive(Default)]
pub struct Texture
{
	pub size: [u32; 2],
	pub pixels: Vec<TextureElement>,
}

#[derive(Copy, Clone)]
pub struct TextureElement
{
	pub diffuse: Color32,
	/// Normal is always normalized.
	pub normal: Vec3f,
}

impl Default for TextureElement
{
	fn default() -> Self
	{
		Self {
			diffuse: Color32::black(),
			normal: Vec3f::unit_z(),
		}
	}
}

pub fn load_textures(materials: &[material::Material], textures_path: &std::path::Path) -> Vec<TextureWithMips>
{
	let mut result = Vec::new();

	for material in materials
	{
		let diffuse = if let Some(image) = load_image(
			&material.diffuse.clone().unwrap_or_else(|| String::new()),
			textures_path,
		)
		{
			image
		}
		else
		{
			make_stub_image()
		};

		let normals = if let Some(normal_map_texture) = &material.normal_map
		{
			load_image(&normal_map_texture.clone(), textures_path)
		}
		else
		{
			None
		};

		let mip0 = make_texture(diffuse, normals);

		result.push(build_mips(mip0));
	}

	result
}

fn load_image(file_name: &str, textures_path: &std::path::Path) -> Option<image::Image>
{
	let mut path = std::path::PathBuf::from(textures_path);
	path.push(file_name);
	image::load(&path)
}

fn make_stub_image() -> image::Image
{
	let size = 32;
	let mut result = image::Image {
		size: [size, size],
		pixels: vec![Color32::from_rgb(0, 0, 0); (size * size) as usize],
	};

	for y in 0 .. result.size[1]
	{
		for x in 0 .. result.size[0]
		{
			let color = if (((x >> 3) ^ (y >> 3)) & 1) != 0
			{
				Color32::from_rgb(224, 224, 224)
			}
			else
			{
				Color32::from_rgb(160, 160, 160)
			};
			result.pixels[(x + y * result.size[0]) as usize] = color;
		}
	}

	result
}

fn make_texture(diffuse: image::Image, normals: Option<image::Image>) -> Texture
{
	let mut result = Texture {
		size: diffuse.size,
		pixels: vec![TextureElement::default(); (diffuse.size[0] * diffuse.size[1]) as usize],
	};

	for (dst, src) in result.pixels.iter_mut().zip(diffuse.pixels.iter())
	{
		dst.diffuse = *src;
	}

	if let Some(mut n) = normals
	{
		// Normally normal map must have same size as diffuse texture.
		// But in case if it is not true - reseze normal map.
		if n.size != diffuse.size
		{
			let n_resized = resize_image(&n, diffuse.size);
			n = n_resized;
		}
		for (dst, src) in result.pixels.iter_mut().zip(n.pixels.iter())
		{
			let rgb = src.get_rgb();
			let zero_level = 128;
			let normal = Vec3f::new(
				((rgb[0] as i32) - zero_level) as f32,
				((rgb[1] as i32) - zero_level) as f32,
				((rgb[2] as i32) - zero_level) as f32,
			);
			dst.normal = renormalize_normal(normal);
		}
	}

	result
}

// Resize with simple nearset filter.
fn resize_image(image: &image::Image, target_size: [u32; 2]) -> image::Image
{
	let mut result = image::Image {
		size: target_size,
		pixels: vec![Color32::from_rgb(255, 0, 255); 16 * 16],
	};

	for y in 0 .. result.size[1]
	{
		let src_y = y * image.size[1] / result.size[1];
		for x in 0 .. result.size[0]
		{
			let src_x = x * image.size[0] / result.size[0];
			result.pixels[(x + y * result.size[0]) as usize] = image.pixels[(src_x + src_y * image.size[0]) as usize];
		}
	}
	result
}

fn build_mips(mip0: Texture) -> TextureWithMips
{
	// This function requires input texture with size multiple of ( 1 << MAX_MIP ).

	let mut result = TextureWithMips::default();

	result[0] = mip0;
	for i in 1 .. NUM_MIPS
	{
		let prev_mip = &mut result[i - 1];
		let mut mip = Texture {
			size: [prev_mip.size[0] >> 1, prev_mip.size[1] >> 1],
			pixels: Vec::new(),
		};

		mip.pixels = vec![TextureElement::default(); (mip.size[0] * mip.size[1]) as usize];
		for y in 0 .. mip.size[1] as usize
		{
			for x in 0 .. mip.size[0] as usize
			{
				let p00 = prev_mip.pixels[(x * 2) + (y * 2) * (prev_mip.size[0] as usize)];
				let p01 = prev_mip.pixels[(x * 2) + (y * 2 + 1) * (prev_mip.size[0] as usize)];
				let p10 = prev_mip.pixels[(x * 2 + 1) + (y * 2) * (prev_mip.size[0] as usize)];
				let p11 = prev_mip.pixels[(x * 2 + 1) + (y * 2 + 1) * (prev_mip.size[0] as usize)];
				let dst = &mut mip.pixels[x + y * (mip.size[0] as usize)];
				dst.diffuse = Color32::get_average_4([p00.diffuse, p01.diffuse, p10.diffuse, p11.diffuse]);
				dst.normal = renormalize_normal(p00.normal + p01.normal + p10.normal + p11.normal);
			}
		}
		result[i] = mip;
	}

	result
}

fn renormalize_normal(normal: Vec3f) -> Vec3f
{
	let len = normal.magnitude();
	if len <= 0.000001
	{
		Vec3f::unit_z()
	}
	else
	{
		normal / len
	}
}
