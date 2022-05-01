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
		// Calculate sum value of unshifted components, than shift and mask it on order to get average value.
		// Exception - last component, which is shifted in oredr to prevent u32 overflow.
		// Such approach uses less instructions relative to naive approach with separate shift of components to range 0-255.
		for color in colors
		{
			components_sum[0] += (color.0 & 0x000000FF) as u32;
			components_sum[1] += (color.0 & 0x0000FF00) as u32;
			components_sum[2] += (color.0 & 0x00FF0000) as u32;
			components_sum[3] += ((color.0 & 0xFF000000) >> 2) as u32;
		}

		Color32(
			(components_sum[0] >> 2) |
				((components_sum[1] >> 2) & 0x0000FF00) |
				((components_sum[2] >> 2) & 0x00FF0000) |
				(components_sum[3] & 0xFF000000),
		)
	}
}
