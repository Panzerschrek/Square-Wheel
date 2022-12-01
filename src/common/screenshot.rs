use super::{color::*, image, system_window};

pub fn save_screenshot(pixels: &[Color32], surface_info: &system_window::SurfaceInfo)
{
	let mut i = image::Image {
		size: [surface_info.width as u32, surface_info.height as u32],
		pixels: vec![Color32::black(); surface_info.width * surface_info.height],
	};

	for y in 0 .. surface_info.height
	{
		let src_line = &pixels[y * surface_info.pitch .. y * surface_info.pitch + surface_info.width];
		let dst_line = &mut i.pixels[y * surface_info.width .. (y + 1) * surface_info.width];
		dst_line.copy_from_slice(src_line);

		// Set alpha to maximum value to avoid ugly screenshots with alpha surfaces.
		for pixel in dst_line
		{
			pixel.set_alpha(255);
		}
	}

	let _ = std::fs::create_dir(SCREENSHOTS_DIR);

	let t = match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)
	{
		Ok(n) => n.as_millis(),
		Err(_) => 0,
	};

	let mut path = std::path::PathBuf::from(SCREENSHOTS_DIR);
	path.push(format!("SquareWheel_screenshot_{}.png", t));

	println!("Saving screenshot {:?}", path);

	image::save(&i, &path);
}

const SCREENSHOTS_DIR: &str = "screenshots";
