#[derive(Clone, Copy)]
pub struct Color32(u32);

impl Color32
{
	pub fn black() -> Self
	{
		Color32(0)
	}

	pub fn white() -> Self
	{
		Color32(0xFFFFFFFF)
	}

	pub fn from_rgb(r: u8, g: u8, b: u8) -> Self
	{
		Color32(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
	}

	pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self
	{
		Color32(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
	}

	pub fn from_raw(raw: u32) -> Self
	{
		Color32(raw)
	}

	pub fn get_raw(self) -> u32
	{
		self.0
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

	pub fn get_rgb(&self) -> [u8; 3]
	{
		[
			((self.0 & 0x00FF0000) >> 16) as u8,
			((self.0 & 0x0000FF00) >> 8) as u8,
			(self.0 & 0x000000FF) as u8,
		]
	}

	pub const MAX_RGB_F32_COMPONENTS: [f32; 3] = [255.0 * (256.0 * 256.0), 255.0 * 256.0, 255.0];

	// Result components are shifted.
	pub fn unpack_to_rgb_f32(&self) -> [f32; 3]
	{
		[
			(self.0 & 0x00FF0000) as f32,
			(self.0 & 0x0000FF00) as f32,
			(self.0 & 0x000000FF) as f32,
		]
	}

	// Pack back shifted components.
	// Truncate extra bits in case of overflow.
	// TODO - add same method but with unsafe f32 to u32 cast.
	pub fn from_rgb_f32(rgb: &[f32; 3]) -> Self
	{
		Color32(((rgb[0] as u32) & 0x00FF0000) | ((rgb[1] as u32) & 0x0000FF00) | ((rgb[2] as u32) & 0x000000FF))
	}

	// Uses unchecked f32 to int conversion.
	// Undefine behaviour  in case of f32 -> u32 overflow/underflow or NaN.
	// Components also must be not greater than MAX_RGB_F32_COMPONENTS.
	pub unsafe fn from_rgb_f32_unchecked(rgb: &[f32; 3]) -> Self
	{
		Color32(
			(rgb[0].to_int_unchecked::<u32>() & 0x00FF0000) |
				(rgb[1].to_int_unchecked::<u32>() & 0x0000FF00) |
				rgb[2].to_int_unchecked::<u32>(),
		)
	}
}
