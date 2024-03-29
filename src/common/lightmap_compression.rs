use super::{bsp_map_compact::*, math_types::*};

pub type LightmapElementCompressed = CompressedColor;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirectionalLightmapElementCompressed
{
	pub ambient_light: CompressedColor,
	pub light_direction_vector_scaled: CompressedVector,
	pub directional_light_deviation: u8,
	pub directional_light_color: CompressedColor,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LightGridElementCompressed
{
	pub light_cube: [CompressedColor; 6],
	pub light_direction_vector_scaled: CompressedVector,
	pub directional_light_color: CompressedColor,
}

// Compact representation of color for lightmaps and other purposes.
// Compression is lossy.
// Minimum represented value - 1 / COLOR_SCALE, maximum - 255.0.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompressedColor
{
	components: [u8; 3],
	scale: u8,
}

// Compact representation of vector for lightmaps and other purposes.
// Compression is lossy.
// Minimum represented component value - 1 / VECTOR_SCALE, maximum - 255.0.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompressedVector
{
	components: [i8; 3],
	scale: u8,
}

impl CompressedColor
{
	pub fn compress(color: &[f32; 3]) -> Self
	{
		let max_component_value = color[0].max(color[1]).max(color[2]);
		let scale = max_component_value.ceil().max(1.0).min(255.0);

		let multiplier = COLOR_SCALE / scale;

		Self {
			components: color.map(|c| (c * multiplier).max(0.0).min(255.0) as u8),
			scale: scale as u8,
		}
	}

	pub fn decompress(&self) -> [f32; 3]
	{
		let multiplier = self.scale as f32 / COLOR_SCALE;
		self.components.map(|c| (c as f32) * multiplier)
	}
}

impl CompressedVector
{
	pub fn compress(v: &Vec3f) -> Self
	{
		let max_component_value = v.x.abs().max(v.y.abs()).max(v.z.abs());
		let scale = max_component_value.ceil().max(1.0).min(255.0);

		let multiplier = VECTOR_SCALE / scale;

		Self {
			components: [v.x, v.y, v.z].map(|c| (c * multiplier).max(-127.0).min(127.0) as i8),
			scale: scale as u8,
		}
	}

	pub fn decompress(&self) -> Vec3f
	{
		let multiplier = self.scale as f32 / VECTOR_SCALE;
		Vec3f::new(
			self.components[0] as f32,
			self.components[1] as f32,
			self.components[2] as f32,
		) * multiplier
	}
}

impl DirectionalLightmapElementCompressed
{
	pub fn compress(e: &DirectionalLightmapElement) -> Self
	{
		Self {
			ambient_light: CompressedColor::compress(&e.ambient_light),
			light_direction_vector_scaled: CompressedVector::compress(&e.light_direction_vector_scaled),
			directional_light_deviation: (e.directional_light_deviation * LIGHT_DEVIATION_SCALE)
				.max(0.0)
				.min(255.0) as u8,
			directional_light_color: CompressedColor::compress(&e.directional_light_color),
		}
	}

	pub fn decompress(&self) -> DirectionalLightmapElement
	{
		let mut res = DirectionalLightmapElement {
			ambient_light: CompressedColor::decompress(&self.ambient_light),
			light_direction_vector_scaled: CompressedVector::decompress(&self.light_direction_vector_scaled),
			directional_light_deviation: self.directional_light_deviation as f32 / LIGHT_DEVIATION_SCALE,
			directional_light_color: CompressedColor::decompress(&self.directional_light_color),
		};

		// Hack! This vector should be non-zero, but can be zero after compression because of quantisation.
		if res.light_direction_vector_scaled.magnitude2() <= 0.0
		{
			res.light_direction_vector_scaled = Vec3f::new(0.0, 0.0, 1.0 / 1024.0);
		}

		res
	}
}

impl LightGridElementCompressed
{
	pub fn compress(e: &LightGridElement) -> Self
	{
		Self {
			light_cube: e.light_cube.map(|s| CompressedColor::compress(&s)),
			light_direction_vector_scaled: CompressedVector::compress(&e.light_direction_vector_scaled),
			directional_light_color: CompressedColor::compress(&e.directional_light_color),
		}
	}

	pub fn decompress(&self) -> LightGridElement
	{
		LightGridElement {
			light_cube: self.light_cube.map(|s| CompressedColor::decompress(&s)),
			light_direction_vector_scaled: CompressedVector::decompress(&self.light_direction_vector_scaled),
			directional_light_color: CompressedColor::decompress(&self.directional_light_color),
		}
	}
}

const COLOR_SCALE: f32 = 255.0;
const VECTOR_SCALE: f32 = 127.0;
const LIGHT_DEVIATION_SCALE: f32 = 255.0;
