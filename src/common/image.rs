use super::color::*;
use std::io::{BufRead, Seek};

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Image
{
	pub size: [u32; 2],
	pub pixels: Vec<Color32>,
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Image64
{
	pub size: [u32; 2],
	pub pixels: Vec<Color64>,
}

pub fn load(file_path: &std::path::Path) -> Option<Image>
{
	let src_image = image::open(file_path).ok()?.into_rgba8();
	Some(convert_loaded_image(&src_image))
}

pub fn load_from_reader<F: BufRead + Seek>(reader: &mut F, format: image::ImageFormat) -> Option<Image>
{
	let src_image = image::load(reader, format).ok()?.into_rgba8();
	Some(convert_loaded_image(&src_image))
}

fn convert_loaded_image(src_image: &image::RgbaImage) -> Image
{
	Image {
		size: [src_image.width(), src_image.height()],
		pixels: src_image
			.pixels()
			.map(|p| Color32::from_rgba(p[0], p[1], p[2], p[3]))
			.collect(),
	}
}

pub fn load64(file_path: &std::path::Path) -> Option<Image64>
{
	Some(convert_loaded_image_64(image::open(file_path).ok()?))
}

pub fn load64_from_reader<F: BufRead + Seek>(reader: &mut F, format: image::ImageFormat) -> Option<Image64>
{
	Some(convert_loaded_image_64(image::load(reader, format).ok()?))
}

fn convert_loaded_image_64(src_image: image::DynamicImage) -> Image64
{
	if let Some(image16) = src_image.as_rgba16()
	{
		return Image64 {
			size: [image16.width(), image16.height()],
			pixels: image16
				.pixels()
				.map(|p| Color64::from_rgba(p[0], p[1], p[2], p[3]))
				.collect(),
		};
	}
	if let Some(image16) = src_image.as_rgb16()
	{
		return Image64 {
			size: [image16.width(), image16.height()],
			pixels: image16.pixels().map(|p| Color64::from_rgb(p[0], p[1], p[2])).collect(),
		};
	}

	let image8 = src_image.into_rgba8();

	Image64 {
		size: [image8.width(), image8.height()],
		pixels: image8
			.pixels()
			.map(|p| Color64::from_rgba(p[0] as u16, p[1] as u16, p[2] as u16, p[3] as u16))
			.collect(),
	}
}

pub fn save(image: &Image, file_path: &std::path::Path) -> bool
{
	let pixels_swapped = image
		.pixels
		.iter()
		.map(|p| {
			let argb = p.get_argb();
			Color32::from_rgba(argb[3], argb[2], argb[1], argb[0])
		})
		.collect::<Vec<_>>();

	if let Err(e) = image::save_buffer(
		file_path,
		unsafe {
			std::slice::from_raw_parts(
				pixels_swapped.as_ptr() as *const u8,
				pixels_swapped.len() * std::mem::size_of::<Color32>(),
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

pub fn make_stub64() -> Image64
{
	let size = 32;
	let mut result = Image64 {
		size: [size, size],
		pixels: vec![Color64::from_rgb(0, 0, 0); (size * size) as usize],
	};

	for y in 0 .. result.size[1]
	{
		for x in 0 .. result.size[0]
		{
			let color = if (((x >> 3) ^ (y >> 3)) & 1) != 0
			{
				Color64::from_rgba(224, 224, 224, 255)
			}
			else
			{
				Color64::from_rgba(160, 160, 160, 128)
			};
			result.pixels[(x + y * result.size[0]) as usize] = color;
		}
	}

	result
}
