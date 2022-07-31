use super::fast_math::*;
use common::color::*;

pub trait AbstractColor: Default + Copy + Send + Sync + From<ColorVec> + From<ColorVecI> + Into<ColorVecI>
{
	fn average(a: Self, b: Self) -> Self;
	fn saturated_sum(a: Self, b: Self) -> Self;
	fn premultiplied_alpha_blend(dst: Self, src: Self) -> Self;
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

	fn premultiplied_alpha_blend(dst: Self, src: Self) -> Self
	{
		let inverted_alpha = (src.get_raw() >> 24) as i32;
		let dst_vec: ColorVecI = dst.into();
		let src_vec: ColorVecI = src.into();
		ColorVecI::add(
			&ColorVecI::shift_right::<8>(&ColorVecI::mul_scalar(&dst_vec, inverted_alpha)),
			&src_vec,
		)
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

impl Into<ColorVecI> for Color32
{
	fn into(self) -> ColorVecI
	{
		ColorVecI::from_color32(self)
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

	fn premultiplied_alpha_blend(dst: Self, src: Self) -> Self
	{
		let inverted_alpha = (src.get_raw() >> 48) as i32;
		let dst_vec: ColorVecI = dst.into();
		let src_vec: ColorVecI = src.into();
		ColorVecI::add(
			&ColorVecI::shift_right::<8>(&ColorVecI::mul_scalar(&dst_vec, inverted_alpha)),
			&src_vec,
		)
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

impl Into<ColorVecI> for Color64
{
	fn into(self) -> ColorVecI
	{
		ColorVecI::from_color64(self)
	}
}
