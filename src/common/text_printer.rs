use super::{color::*, system_window};

// Simple function for printing of helper texts (console, debug messages, etc.).
// Supports only ASCII symbols.
pub fn print(
	pixels: &mut [Color32],
	surface_info: &system_window::SurfaceInfo,
	text: &str,
	x: i32,
	y: i32,
	color: Color32,
)
{
	// TODO - optimize this.
	let src_bitmap = &FONT_BITMAP_FILE_CONTENT[BMP_HEADER_LEN ..];

	let mut cur_y = y;
	let mut cur_x = x;
	for c in text.chars()
	{
		if c == '\n'
		{
			cur_x = x;
			cur_y += GLYPH_HEIGHT as i32;
			continue;
		}

		let mut c_num = c as usize;
		if c_num < 32 || c_num >= 128
		{
			c_num = '?' as usize;
		}

		let glyph_index = (c_num as usize) - 32;
		for glyph_y in 0 .. GLYPH_HEIGHT
		{
			let dst_y = cur_y + (glyph_y as i32);
			if dst_y < 0 || dst_y >= (surface_info.height as i32)
			{
				continue;
			}
			let glyph_line_byte = src_bitmap[glyph_index + ((GLYPH_HEIGHT - 1) - glyph_y) * NUM_GLYPHS];
			let dst_base = dst_y * (surface_info.pitch as i32);
			for glyph_x in 0 .. (GLYPH_WIDTH as i32)
			{
				if (glyph_line_byte & (1 << ((glyph_x & 7) ^ 7))) != 0
				{
					let dst_x = cur_x + glyph_x;
					if dst_x >= 0 && dst_x < (surface_info.width as i32)
					{
						pixels[(dst_base + dst_x) as usize] = color;
					}
				}
			}
		}

		cur_x += GLYPH_WIDTH as i32;
	}
}

const GLYPH_WIDTH: usize = 8;
const GLYPH_HEIGHT: usize = 18;
const NUM_GLYPHS: usize = 96;
const BMP_HEADER_LEN: usize = 62;
// 1 bit font bitmap.
const FONT_BITMAP_FILE_CONTENT: &[u8; BMP_HEADER_LEN + GLYPH_WIDTH * GLYPH_HEIGHT * NUM_GLYPHS / 8] =
	include_bytes!("fixedsys8x18.bmp");
