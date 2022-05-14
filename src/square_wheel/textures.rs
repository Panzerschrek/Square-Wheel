use common::{color::*, image, material};

// MAX_MIP must be not greater, than LIGHTMAP_SCALE_LOG2
pub const MAX_MIP: usize = 3;
pub const NUM_MIPS: usize = MAX_MIP + 1;
pub type TextureWithMips = [image::Image; NUM_MIPS];

pub fn load_textures(materials: &[material::Material], textures_path: &std::path::Path) -> Vec<TextureWithMips>
{
	let mut result = Vec::new();

	for material in materials
	{
		let mut texture_path = std::path::PathBuf::from(textures_path);
		texture_path.push(material.diffuse.clone().unwrap_or_else(|| String::new()));

		let mip0 = if let Some(image) = image::load(&texture_path)
		{
			image
		}
		else
		{
			println!("Failed to load texture {:?}", texture_path);
			make_stub_texture()
		};

		result.push(build_mips(mip0));
	}

	result
}

fn make_stub_texture() -> image::Image
{
	let mut result = image::Image {
		size: [16, 16],
		pixels: vec![Color32::from_rgb(255, 0, 255); 16 * 16],
	};

	for y in 0 .. result.size[1]
	{
		for x in 0 .. result.size[0]
		{
			let color = if (((x >> 3) ^ (y >> 3)) & 1) != 0
			{
				Color32::from_rgb(255, 0, 255)
			}
			else
			{
				Color32::from_rgb(128, 128, 128)
			};
			result.pixels[(x + y * result.size[0]) as usize] = color;
		}
	}

	result
}

fn build_mips(mip0: image::Image) -> TextureWithMips
{
	// This function requires input texture with size multiple of ( 1 << MAX_MIP ).

	let mut result = TextureWithMips::default();

	result[0] = mip0;
	for i in 1 .. NUM_MIPS
	{
		let prev_mip = &mut result[i - 1];
		let mut mip = image::Image {
			size: [prev_mip.size[0] >> 1, prev_mip.size[1] >> 1],
			pixels: Vec::new(),
		};

		mip.pixels = vec![Color32::from_rgb(0, 0, 0); (mip.size[0] * mip.size[1]) as usize];
		for y in 0 .. mip.size[1] as usize
		{
			for x in 0 .. mip.size[0] as usize
			{
				mip.pixels[x + y * (mip.size[0] as usize)] = Color32::get_average_4([
					prev_mip.pixels[(x * 2) + (y * 2) * (prev_mip.size[0] as usize)],
					prev_mip.pixels[(x * 2) + (y * 2 + 1) * (prev_mip.size[0] as usize)],
					prev_mip.pixels[(x * 2 + 1) + (y * 2) * (prev_mip.size[0] as usize)],
					prev_mip.pixels[(x * 2 + 1) + (y * 2 + 1) * (prev_mip.size[0] as usize)],
				]);
			}
		}
		result[i] = mip;
	}

	result
}
