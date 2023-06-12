use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaterEffect
{
	pub resolution_log2: [u32; 2],

	/// Greater value - less waves attenuation.
	/// attenuation = 1.0 - 1.0 / fluidity
	#[serde(default = "default_fluidity")]
	pub fluidity: f32,

	pub wave_sources: Vec<WaveSource>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WaveSource
{
	/// Produce sinusoidal vawe at given point.
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
}

fn default_fluidity() -> f32
{
	200.0
}

fn default_one() -> f32
{
	1.0
}
