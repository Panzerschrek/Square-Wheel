use super::color::*;
use sdl2::image::{LoadSurface, SaveSurface};

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Image
{
	pub size: [u32; 2],
	pub pixels: Vec<Color32>,
}

pub fn load(file_path: &std::path::Path) -> Option<Image>
{
	let expected_format = sdl2::pixels::PixelFormatEnum::ARGB8888;
	if let Ok(mut surface) = sdl2::surface::Surface::from_file(file_path)
	{
		if surface.pixel_format_enum() != expected_format
		{
			if let Ok(surface_converted) = surface.convert_format(expected_format)
			{
				surface = surface_converted;
			}
			else
			{
				return None;
			}
		}

		let row_size = surface.pitch() / 4;
		let mut image = Image {
			size: [surface.width(), surface.height()],
			pixels: vec![Color32::from_rgb(0, 0, 0); (surface.height() * row_size) as usize],
		};

		surface.with_lock(|pixels| {
			// TODO - what if alignment is wrong?
			let pixels_32 = unsafe { pixels.align_to::<Color32>().1 };
			for y in 0 .. image.size[1]
			{
				let dst_start = y * image.size[0];
				let dst = &mut image.pixels[(dst_start as usize) .. (dst_start + image.size[0]) as usize];
				let src_start = y * row_size;
				let src = &pixels_32[(src_start as usize) .. (src_start + image.size[0]) as usize];

				dst.copy_from_slice(src);
			}
		});

		return Some(image);
	}

	None
}

pub fn save(image: &Image, file_path: &std::path::Path) -> bool
{
	let element_size = std::mem::size_of::<Color32>();
	let bytes = unsafe {
		std::slice::from_raw_parts_mut(
			(&image.pixels[0]) as *const Color32 as *mut Color32 as *mut u8,
			element_size * image.pixels.len(),
		)
	};

	if let Ok(surface) = sdl2::surface::Surface::from_data(
		bytes,
		image.size[0],
		image.size[1],
		image.size[0] * (element_size as u32),
		sdl2::pixels::PixelFormatEnum::ARGB8888,
	)
	{
		match surface.save(file_path)
		{
			Ok(()) => true,
			Err(e) =>
			{
				println!("Failed to save image: {}", e);
				false
			},
		}
	}
	else
	{
		false
	}
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
