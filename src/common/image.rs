use super::color::*;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Image
{
	pub size: [u32; 2],
	pub pixels: Vec<Color32>,
}

pub fn load(file_path: &std::path::Path) -> Option<Image>
{
	let src_image = image::open(file_path).ok()?.into_rgba8();

	Some(Image {
		size: [src_image.width(), src_image.height()],
		pixels: src_image
			.pixels()
			.map(|p| Color32::from_rgba(p[0], p[1], p[2], p[3]))
			.collect(),
	})
}

pub fn save(image: &Image, file_path: &std::path::Path) -> bool
{
	if let Err(e) = image::save_buffer(
		file_path,
		unsafe {
			std::slice::from_raw_parts(
				image.pixels.as_ptr() as *const u8,
				image.pixels.len() * std::mem::size_of::<Color32>(),
			)
		},
		image.size[0],
		image.size[1],
		image::ColorType::Rgba8,
	)
	{
		println!("Failed to save image: {}", e);
		return false;
	}

	true
}

pub fn make_stub() -> Image
{
	let size = 32;
	let mut result = Image {
		size: [size, size],
		pixels: vec![Color32::from_rgb(0, 0, 0); (size * size) as usize],
	};

	for y in 0 .. result.size[1]
	{
		for x in 0 .. result.size[0]
		{
			let color = if (((x >> 3) ^ (y >> 3)) & 1) != 0
			{
				Color32::from_rgba(224, 224, 224, 255)
			}
			else
			{
				Color32::from_rgba(160, 160, 160, 128)
			};
			result.pixels[(x + y * result.size[0]) as usize] = color;
		}
	}

	result
}
