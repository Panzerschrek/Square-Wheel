use super::fast_math::*;
use common::color::*;

pub trait AbstractColor: Copy + Send + Sync + From<ColorVec> + From<ColorVecI> + Into<ColorVecI>
{
	fn average(a: Self, b: Self) -> Self;
	fn saturated_sum(a: Self, b: Self) -> Self;
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
