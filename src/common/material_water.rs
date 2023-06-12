use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaterEffect
{
	pub resolution_log2: [u32; 2],

	/// Number of update steps, performed per second.
	/// Greater frequency - faster waves but slower computation.
	#[serde(default = "default_update_frequency")]
	pub update_frequency: f32,

	/// Greater value - less waves attenuation.
	/// attenuation = 1.0 - 1.0 / fluidity
	#[serde(default = "default_fluidity")]
	pub fluidity: f32,

	#[serde(default)]
	pub color_texture_apply_mode: ColorTextureApplyMode,

	pub wave_sources: Vec<WaveSource>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum ColorTextureApplyMode
{
	/// Use single color from source texture.
	SingleColor,
	/// Use source texture as is.
	SourceTexture,
	/// Deform source texture, based on calculated water normals.
	SourceTextureNormalDeformed,
	/// Deform source texture, based on calculated water normals. Only X component of normal is used.
	/// Such deformation is faster, than previous, but looks not so good.
	SourceTextureNormalDeformedX,
}

impl Default for ColorTextureApplyMode
{
	fn default() -> Self
	{
		Self::SingleColor
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WaveSource
{
	/// Produce sinusoidal wave at given point.
	WavySpot
	{
		center: [u32; 2],

		#[serde(default = "default_one")]
		frequency: f32,

		#[serde(default)]
		phase: f32,

		#[serde(default = "default_one")]
		amplitude: f32,

		#[serde(default)]
		offset: f32,
	},
	/// Produce sinusoidal wave along given line.
	WavyLine
	{
		points: [[u32; 2]; 2],

		#[serde(default = "default_one")]
		frequency: f32,

		#[serde(default)]
		phase: f32,

		#[serde(default = "default_one")]
		amplitude: f32,

		#[serde(default)]
		offset: f32,
	},
	PeriodicDroplet
	{
		center: [u32; 2],

		#[serde(default = "default_one")]
		frequency: f32,

		/// In range 0 - 1.
		#[serde(default)]
		phase: f32,

		#[serde(default = "default_one")]
		amplitude: f32,
	},
	/// Produce random droplets.
	Rain
	{
		#[serde(default)]
		center: [u32; 2],

		/// How often drops are produced (by they are still random).
		/// If this value is greater than water effect update frequency - new droplet (but only one) will be emitted each frame.
		#[serde(default = "default_one")]
		frequency: f32,

		/// If radius is zero - produce rain droplets totally ranomly, if it is non-zero - produce droplets only around center.
		#[serde(default)]
		radius: f32,

		#[serde(default = "default_one")]
		amplitude: f32,
	},
}

fn default_fluidity() -> f32
{
	200.0
}

fn default_one() -> f32
{
	1.0
}

fn default_update_frequency() -> f32
{
	30.0
}
