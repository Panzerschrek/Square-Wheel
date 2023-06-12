use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaterEffect
{
	pub resolution_log2: [u32; 2],

	/// Greater value - less waves attenuation.
	/// attenuation = 1.0 - 1.0 / fluidity
	#[serde(default = "default_fluidity")]
	pub fluidity: f32,
}

fn default_fluidity() -> f32
{
	200.0
}
