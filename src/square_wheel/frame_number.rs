// 32 bits are enough for frames enumeration.
// It is more than year at 60FPS.
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct FrameNumber(u32);

impl FrameNumber
{
	pub fn next(&mut self)
	{
		self.0 += 1;
	}
}
