use super::material_function::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FireEffect
{
	pub resolution_log2: [u32; 2],

	/// Number of update steps, performed per second.
	/// Greater frequency - faster fire but slower computation.
	#[serde(default = "default_update_frequency")]
	pub update_frequency: f32,

	/// Greater value - less fire attenuation.
	/// attenuation = 1.0 - 1.0 / heat_conductivity
	#[serde(default = "default_heat_conductivity")]
	pub heat_conductivity: f32,

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

		#[serde(default = "default_heat")]
		heat: f32,
	},
	/// Set heat along given line.
	ConstantLine
	{
		points: [[u32; 2]; 2],

		#[serde(default = "default_heat")]
		heat: f32,
	},
	/// Emit particles.
	Fountain
	{
		center: [u32; 2],

		// Particles/s.
		/// If this value is greater than fire effect update frequency - new particle (but only one) will be emitted each frame.
		#[serde(default = "default_one")]
		frequency: f32,

		// Heat of emited particles.
		#[serde(default = "default_heat")]
		heat: f32,

		// Angle of initial particle velocity.
		#[serde(default)]
		particle_angle: ValueWithRandomDeviation,

		// Initial speed (pixels/s).
		#[serde(default)]
		particle_speed: ValueWithRandomDeviation,

		// Angle in which direction particle is spawned.
		#[serde(default)]
		particle_spawn_angle: ValueWithRandomDeviation,

		// Distance from center (in pixels), where particle is spawned.
		#[serde(default)]
		particle_spawn_distance: ValueWithRandomDeviation,

		// Lifetime of spawned particle (in seconds(.
		#[serde(default = "default_particle_lifetime")]
		particle_lifetime: ValueWithRandomDeviation,
	},
}

// Result = value +- random_deviation
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ValueWithRandomDeviation
{
	#[serde(default)]
	pub value: SingleArgumentFunction,
	#[serde(default)]
	pub random_deviation: SingleArgumentFunction,
}

fn default_heat() -> f32
{
	1.0
}

fn default_one() -> f32
{
	1.0
}

fn default_update_frequency() -> f32
{
	30.0
}

fn default_heat_conductivity() -> f32
{
	20.0
}

fn default_particle_lifetime() -> ValueWithRandomDeviation
{
	ValueWithRandomDeviation {
		value: SingleArgumentFunction::Constant(1.0),
		random_deviation: SingleArgumentFunction::default(),
	}
}
