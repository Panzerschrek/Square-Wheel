#[derive(Clone, Copy)]
pub struct Color32(u32);

impl Color32
{
	pub fn from_rgb(r: u8, g: u8, b: u8) -> Self
	{
		Color32(((r as u32) << 16) | ((g as u32) << 8) | ((b as u32) << 0))
	}
}
