use serde::{Deserialize, Serialize};

// Reperesentation of some simple one parameter function, used by materials.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum SingleArgumentFunction
{
	Constant(f32),
	Linear
	{
		/// y = x * scale + offset
		#[serde(default = "default_one")]
		scale: f32,

		#[serde(default)]
		offset: f32,
	},
	SinWave
	{
		/// y = amplitude * sin(2 * pi * x + phase) + offset

		#[serde(default = "default_one")]
		frequency: f32,

		#[serde(default)]
		phase: f32,

		#[serde(default = "default_one")]
		amplitude: f32,

		#[serde(default)]
		offset: f32,
	},
	// TODO - add other functions
}

impl Default for SingleArgumentFunction
{
	fn default() -> Self
	{
		SingleArgumentFunction::Constant(0.0)
	}
}

impl SingleArgumentFunction
{
	pub fn evaluate(&self, x: f32) -> f32
	{
		match self
		{
			SingleArgumentFunction::Constant(c) => *c,
			SingleArgumentFunction::Linear { scale, offset } => x * scale + offset,
			SingleArgumentFunction::SinWave {
				frequency,
				phase,
				amplitude,
				offset,
			} => (x * frequency * std::f32::consts::TAU + phase).sin() * amplitude + offset,
		}
	}
}

fn default_one() -> f32
{
	1.0
}
