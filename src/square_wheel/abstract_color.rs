use super::fast_math::*;
use crate::common::color::*;

pub trait AbstractColor: Default + Copy + Send + Sync + From<ColorVec> + From<ColorVecI> + Into<ColorVecI>
{
	fn average(a: Self, b: Self) -> Self;
	fn saturated_sum(a: Self, b: Self) -> Self;
	fn get_alpha(self) -> i32;
	fn alpha_blend(dst: Self, src: Self) -> Self;
	fn test_alpha(self) -> bool;
}

impl AbstractColor for Color32
{
	fn average(a: Self, b: Self) -> Self
	{
		color32_average(a, b)
	}

	fn saturated_sum(a: Self, b: Self) -> Self
	{
		color32_saturated_sum(a, b)
	}

	fn get_alpha(self) -> i32
	{
		(self.get_raw() >> 24) as i32
	}

	fn alpha_blend(dst: Self, src: Self) -> Self
	{
		let alpha = src.get_alpha();
		let dst_vec: ColorVecI = dst.into();
		let src_vec: ColorVecI = src.into();
		ColorVecI::shift_right::<8>(&ColorVecI::add(
			&ColorVecI::mul_scalar(&dst_vec, 255 - alpha),
			&ColorVecI::mul_scalar(&src_vec, alpha),
		))
		.into()
	}

	fn test_alpha(self) -> bool
	{
		// TODO - speed-up this?
		self.get_raw() & 0xFF000000 > 0x7F000000
	}
}

impl From<ColorVec> for Color32
{
	fn from(v: ColorVec) -> Color32
	{
		v.into_color32()
	}
}

impl From<ColorVecI> for Color32
{
	fn from(v: ColorVecI) -> Color32
	{
		v.into_color32()
	}
}

impl From<Color32> for ColorVecI
{
	fn from(c: Color32) -> ColorVecI
	{
		ColorVecI::from_color32(c)
	}
}

impl AbstractColor for Color64
{
	fn average(a: Self, b: Self) -> Self
	{
		color64_average(a, b)
	}

	fn saturated_sum(a: Self, b: Self) -> Self
	{
		color64_saturated_sum(a, b)
	}

	fn get_alpha(self) -> i32
	{
		(self.get_raw() >> 48) as i32
	}

	fn alpha_blend(dst: Self, src: Self) -> Self
	{
		let alpha = src.get_alpha();
		let dst_vec: ColorVecI = dst.into();
		let src_vec: ColorVecI = src.into();
		ColorVecI::shift_right::<8>(&ColorVecI::add(
			&ColorVecI::mul_scalar(&dst_vec, 255 - alpha),
			&ColorVecI::mul_scalar(&src_vec, alpha),
		))
		.into()
	}

	fn test_alpha(self) -> bool
	{
		// TODO - speed-up this?
		self.get_raw() & 0xFFFF000000000000 > 0x007F000000000000
	}
}

impl From<ColorVec> for Color64
{
	fn from(v: ColorVec) -> Color64
	{
		v.into_color64()
	}
}

impl From<ColorVecI> for Color64
{
	fn from(v: ColorVecI) -> Color64
	{
		v.into_color64()
	}
}

impl From<Color64> for ColorVecI
{
	fn from(c: Color64) -> ColorVecI
	{
		ColorVecI::from_color64(c)
	}
}
