pub use rand::{Rng, RngCore, SeedableRng};

// Use simple and fast random engine for texture effects.
pub type RandEngine = rand::rngs::SmallRng;

pub fn create_rand_engine() -> RandEngine
{
	// Initialize random engine generator with good, but deterministic value.
	RandEngine::seed_from_u64(0b1001100000111010100101010101010111000111010110100101111001010101)
}
