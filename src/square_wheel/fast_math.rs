// This module contains helper functions, based on various intrinsincs.

pub use fast_math_impl::*;

#[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "sse4.1"))]
mod fast_math_impl
{
	use common::color::*;

	#[cfg(target_arch = "x86")]
	use core::arch::x86::*;

	#[cfg(target_arch = "x86_64")]
	use core::arch::x86_64::*;

	// Relative erorr <= 1.5 * 2^(-12)
	pub fn inv_sqrt_fast(x: f32) -> f32
	{
		unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set1_ps(x))) }
	}

	// Relative erorr <= 1.5 * 2^(-12)
	pub fn inv_fast(x: f32) -> f32
	{
		unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set1_ps(x))) }
	}

	// Pack 4 floats into 4 signed bytes.
	pub fn pack_f32x4_into_bytes(v: &[f32; 4], pack_scale: &[f32; 4]) -> i32
	{
		unsafe {
			let values_f = _mm_set_ps(v[3], v[2], v[1], v[0]);
			let scale = _mm_set_ps(pack_scale[3], pack_scale[2], pack_scale[1], pack_scale[0]);
			let values_scaled = _mm_mul_ps(values_f, scale);
			let values_32bit = _mm_cvtps_epi32(values_scaled);
			let zero = _mm_setzero_si128();
			let values_16bit = _mm_packs_epi32(values_32bit, zero);
			let values_8bit = _mm_packs_epi16(values_16bit, zero);
			let values_packed = _mm_cvtsi128_si32(values_8bit);
			values_packed
		}
	}

	// Unpak 4 signed bytes to floats.
	pub fn upack_bytes_into_f32x4(b: i32, unpack_scale: &[f32; 4]) -> [f32; 4]
	{
		unsafe {
			let values_8bit = _mm_cvtsi32_si128(b);
			let values_32bit = _mm_cvtepi8_epi32(values_8bit);
			let values_f4 = _mm_cvtepi32_ps(values_32bit);
			let scale = _mm_set_ps(unpack_scale[3], unpack_scale[2], unpack_scale[1], unpack_scale[0]);
			let values_scaled = _mm_mul_ps(values_f4, scale);
			[
				f32::from_bits(_mm_extract_ps(values_scaled, 0) as u32),
				f32::from_bits(_mm_extract_ps(values_scaled, 1) as u32),
				f32::from_bits(_mm_extract_ps(values_scaled, 2) as u32),
				f32::from_bits(_mm_extract_ps(values_scaled, 3) as u32),
			]
		}
	}

	#[repr(C, align(32))]
	pub struct ColorVec(__m128);

	impl ColorVec
	{
		pub fn zero() -> Self
		{
			unsafe { Self(_mm_setzero_ps()) }
		}

		pub fn from_color32(c: Color32) -> Self
		{
			unsafe {
				let color_32bit = c.get_raw() as i32;
				let values_8bit = _mm_cvtsi32_si128(color_32bit);
				let values_32bit = _mm_cvtepu8_epi32(values_8bit);
				let values_f4 = _mm_cvtepi32_ps(values_32bit);
				Self(values_f4)
			}
		}

		pub fn into_color32(&self) -> Color32
		{
			// Here we 100% sure that components overflow is not possible (because of "min").
			// NaNs are not possible here too.
			unsafe {
				let values_clamped = _mm_min_ps(self.0, _mm_set_ps(255.0, 255.0, 255.0, 255.0));
				let values_32bit = _mm_cvtps_epi32(values_clamped);
				let shuffle_mask = _mm_set_epi8(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 8, 4, 0);
				let values_8bit = _mm_shuffle_epi8(values_32bit, shuffle_mask);
				let color_32bit = _mm_cvtsi128_si32(values_8bit);
				Color32::from_raw(color_32bit as u32)
			}
		}

		pub fn from_color_f32x3(c: &[f32; 3]) -> Self
		{
			unsafe { Self(_mm_set_ps(0.0, c[0], c[1], c[2])) }
		}

		pub fn mul(&self, other: &Self) -> Self
		{
			unsafe { Self(_mm_mul_ps(self.0, other.0)) }
		}

		pub fn scalar_mul(&self, scalar: f32) -> Self
		{
			unsafe { Self(_mm_mul_ps(self.0, _mm_broadcastss_ps(_mm_set1_ps(scalar)))) }
		}

		pub fn mul_scalar_add(&self, scalar: f32, b: &Self) -> Self
		{
			unsafe { Self(_mm_fmadd_ps(self.0, _mm_broadcastss_ps(_mm_set1_ps(scalar)), b.0)) }
		}
	} // impl ColorVec
}

#[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "sse4.1")))]
mod fast_math_impl
{
	use common::color::*;

	pub fn inv_sqrt_fast(x: f32) -> f32
	{
		1.0 / x.sqrt()
	}

	pub fn inv_fast(x: f32) -> f32
	{
		1.0 / x
	}

	// Pack 4 floats into 4 signed bytes.
	pub fn pack_f32x4_into_bytes(v: &[f32; 4], pack_scale: &[f32; 4]) -> i32
	{
		let mut res = 0;
		for i in 0 .. 4
		{
			res |= ((v[i] * pack_scale[i]) as i32) << (i * 8);
		}
		res
	}

	// Unpak 4 signed bytes to floats.
	pub fn upack_bytes_into_f32x4(b: i32, unpack_scale: &[f32; 4]) -> [f32; 4]
	{
		let mut res = [0.0; 4];
		for i in 0 .. 4
		{
			res[i] = ((((b as u32) >> (i * 8)) & 0xFF) as f32) * unpack_scale[i];
		}
		res
	}

	// TODO - maybe use here array of 3 floats?
	#[repr(C, align(32))]
	pub struct ColorVec([f32; 4]);

	impl ColorVec
	{
		pub fn zero() -> Self
		{
			Self([0.0, 0.0, 0.0, 0.0])
		}

		pub fn from_color32(c: Color32) -> Self
		{
			let mut res = [0.0; 4];
			for i in 0 .. 4
			{
				res[i] = ((c.get_raw() >> (i * 8)) & 0xFF) as f32;
			}
			Self(res)
		}

		pub fn into_color32(&self) -> Color32
		{
			// Here we 100% sure that components overflow is not possible (because of "min").
			// NaNs are not possible here too.
			let mut res = 0;
			unsafe {
				for i in 0 .. 4
				{
					res |= self.0[i].min(255.0).to_int_unchecked::<u32>() << (i * 8);
				}
			}
			Color32::from_raw(res)
		}

		pub fn from_color_f32x3(c: &[f32; 3]) -> Self
		{
			Self([c[0], c[1], c[2], 0.0])
		}

		pub fn mul(&self, other: &Self) -> Self
		{
			Self([
				self.0[0] * other.0[0],
				self.0[1] * other.0[1],
				self.0[2] * other.0[2],
				self.0[3] * other.0[3],
			])
		}

		pub fn scalar_mul(&self, scalar: f32) -> Self
		{
			Self([
				self.0[0] * scalar,
				self.0[1] * scalar,
				self.0[2] * scalar,
				self.0[3] * scalar,
			])
		}

		pub fn add(&self, other: &Self) -> Self
		{
			Self([
				self.0[0] + other.0[0],
				self.0[1] + other.0[1],
				self.0[2] + other.0[2],
				self.0[3] + other.0[3],
			])
		}

		pub fn mul_scalar_add(&self, scalar: f32, b: &Self) -> Self
		{
			Self([
				f32::mul_add(self.0[0], scalar, b.0[0]),
				f32::mul_add(self.0[1], scalar, b.0[1]),
				f32::mul_add(self.0[2], scalar, b.0[2]),
				f32::mul_add(self.0[3], scalar, b.0[3]),
			])
		}
	} // impl ColorVec
}
