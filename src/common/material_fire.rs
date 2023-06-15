use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FireEffect
{
	pub resolution_log2: [u32; 2],

	/// Number of update steps, performed per second.
	/// Greater frequency - faster fire but slower computation.
	#[serde(default = "default_update_frequency")]
	pub update_frequency: f32,
}

fn default_update_frequency() -> f32
{
	30.0
}
