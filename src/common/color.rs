#[derive(Clone, Copy)]
pub struct Color32(u32);

impl Color32
{
	pub fn from_rgb(r: u8, g: u8, b: u8) -> Self
	{
		Color32(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
	}

	pub fn get_inverted(self) -> Self
	{
		Color32(!self.0)
	}

	pub fn get_half_dark(self) -> Self
	{
		Color32((self.0 & 0xFEFEFEFE) >> 1)
	}

	pub fn get_average_4(colors: [Color32; 4]) -> Self
	{
		let mut components_sum = [0u32; 4];
		for color in colors
		{
			components_sum[0] += (color.0 & 255) as u32;
			components_sum[1] += ((color.0 >> 8) & 255) as u32;
			components_sum[2] += ((color.0 >> 16) & 255) as u32;
			components_sum[3] += (color.0 >> 24) as u32;
		}

		Color32(
			(components_sum[3] >> 2 << 24) |
				(components_sum[2] >> 2 << 16) |
				(components_sum[1] >> 2 << 8) |
				(components_sum[0] >> 2),
		)
	}
}
