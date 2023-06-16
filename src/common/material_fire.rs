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

	/// If true - use slow heat propagation approach.
	#[serde(default)]
	pub slow: bool,

	/// If some - produce emissive texture by modulating heat map using this color.
	/// Color is in range [0; 1], out of range values will be clamped.
	/// If none - emissive image will be used as gradient image.
	#[serde(default)]
	pub color: Option<[f32; 3]>,

	/// Sources of heat.
	/// Without any source fire texture is completely dark and boring.
	pub heat_sources: Vec<HeatSource>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum HeatSource
{
	/// Set heat at given point.
	Point
	{
		center: [u32; 2],

		/// Offset function for x/y coordinates.
		#[serde(default)]
		offset: [ValueWithRandomDeviation; 2],

		#[serde(default = "default_heat")]
		heat: ValueWithRandomDeviation,
	},
	/// Set heat along given line.
	Line
	{
		points: [[u32; 2]; 2],

		#[serde(default = "default_heat")]
		heat: ValueWithRandomDeviation,
	},
	/// Draw lightning between specified points, using given heat.
	Lightning
	{
		points: [[u32; 2]; 2],

		/// Offset function for x/y coordinates of each point.
		#[serde(default)]
		offset: [[ValueWithRandomDeviation; 2]; 2],

		#[serde(default = "default_heat")]
		heat: ValueWithRandomDeviation,

		/// If true - decay heat intensity from start to end.
		#[serde(default)]
		ramp: bool,
	},
	/// Emit particles.
	Fountain
	{
		center: [u32; 2],

		/// Particles/s.
		#[serde(default = "default_one")]
		frequency: f32,

		/// Heat of emited particles.
		#[serde(default = "default_heat")]
		heat: ValueWithRandomDeviation,

		/// Angle of initial particle velocity.
		#[serde(default)]
		angle: ValueWithRandomDeviation,

		// Initial speed (pixels/s).
		#[serde(default)]
		speed: ValueWithRandomDeviation,

		/// pixels / (s * s).
		#[serde(default)]
		gravity: ValueWithRandomDeviation,

		/// Angle in which direction particle is spawned.
		#[serde(default)]
		spawn_angle: ValueWithRandomDeviation,

		/// Distance from center (in pixels), where particle is spawned.
		#[serde(default)]
		spawn_distance: ValueWithRandomDeviation,

		/// Lifetime of spawned particle (in seconds).
		#[serde(default = "default_particle_lifetime")]
		lifetime: ValueWithRandomDeviation,
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

fn default_heat() -> ValueWithRandomDeviation
{
	ValueWithRandomDeviation {
		value: SingleArgumentFunction::Constant(1.0),
		random_deviation: SingleArgumentFunction::default(),
	}
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
