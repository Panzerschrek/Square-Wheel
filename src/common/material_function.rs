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
	TriangleWave
	{
		// Like sin-wave, but triangular.
		#[serde(default = "default_one")]
		frequency: f32,

		#[serde(default)]
		phase: f32,

		#[serde(default = "default_one")]
		amplitude: f32,

		#[serde(default)]
		offset: f32,
	},
	SawToothWave
	{
		// Like sin-wave, but saw-tooth shaped.
		#[serde(default = "default_one")]
		frequency: f32,

		#[serde(default)]
		phase: f32,

		#[serde(default = "default_one")]
		amplitude: f32,

		#[serde(default)]
		offset: f32,
	},
	StepWave
	{
		// Like sin-wave, but step shaped (first 1, than -1).
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
			Self::Constant(c) => *c,
			Self::Linear { scale, offset } => x * scale + offset,
			Self::SinWave {
				frequency,
				phase,
				amplitude,
				offset,
			} => (x * frequency * std::f32::consts::TAU + phase).sin() * amplitude + offset,
			Self::TriangleWave {
				frequency,
				phase,
				amplitude,
				offset,
			} =>
			{
				let arg = x * frequency + phase;
				let arg_fract = arg - arg.floor();
				let wave = if arg_fract <= 0.25
				{
					4.0 * arg_fract
				}
				else if arg_fract <= 0.75
				{
					2.0 - 4.0 * arg_fract
				}
				else
				{
					-4.0 + 4.0 * arg_fract
				};
				wave * amplitude + offset
			},
			Self::SawToothWave {
				frequency,
				phase,
				amplitude,
				offset,
			} =>
			{
				let arg = x * frequency + phase;
				let arg_fract = arg - arg.floor();
				let wave = arg_fract * 2.0 - 1.0;
				wave * amplitude + offset
			},
			Self::StepWave {
				frequency,
				phase,
				amplitude,
				offset,
			} =>
			{
				let arg = x * frequency + phase;
				let arg_fract = arg - arg.floor();
				let wave = if arg_fract <= 0.5 { 1.0 } else { -1.0 };
				wave * amplitude + offset
			},
		}
	}
}

fn default_one() -> f32
{
	1.0
}
