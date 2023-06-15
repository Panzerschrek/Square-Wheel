use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FireEffect
{
	pub resolution_log2: [u32; 2],

	/// Number of update steps, performed per second.
	/// Greater frequency - faster fire but slower computation.
	#[serde(default = "default_update_frequency")]
	pub update_frequency: f32,

	/// Sources of heat.
	/// Without any source fire texture is completely dark and boring.
	pub heat_sources: Vec<HeatSource>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HeatSource
{
	/// Set heat at given point.
	ConstantPoint
	{
		center: [u32; 2],

		#[serde(default = "default_one")]
		heat: f32,
	},
	/// Set heat along given line.
	ConstantLine
	{
		points: [[u32; 2]; 2],

		#[serde(default = "default_one")]
		heat: f32,
	},
}

fn default_one() -> f32
{
	1.0
}

fn default_update_frequency() -> f32
{
	30.0
}
