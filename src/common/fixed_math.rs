pub type Fixed16 = i32;
pub const FIXED16_BASE: Fixed16 = 16;
pub const FIXED16_ONE: Fixed16 = 1 << FIXED16_BASE;
pub const FIXED16_HALF: Fixed16 = FIXED16_ONE >> 1;

pub fn int_to_fixed16(x: i32) -> Fixed16
{
	x << FIXED16_BASE
}

pub fn fixed16_round_to_int(x: Fixed16) -> i32
{
	(x + FIXED16_HALF) >> FIXED16_BASE
}

pub fn fixed16_floor_to_int(x: Fixed16) -> i32
{
	x >> FIXED16_BASE
}

pub fn fixed16_ceil_to_int(x: Fixed16) -> i32
{
	(x + (FIXED16_ONE - 1)) >> FIXED16_BASE
}

pub fn fixed16_to_f32(x: Fixed16) -> f32
{
	(x as f32) / (FIXED16_ONE as f32)
}

pub fn f32_to_fixed16(x: f32) -> Fixed16
{
	(x * (FIXED16_ONE as f32)) as Fixed16
}

pub fn fixed16_mul(x: Fixed16, y: Fixed16) -> Fixed16
{
	(((x as i64) * (y as i64)) >> FIXED16_BASE) as i32
}

pub fn fixed16_square(x: Fixed16) -> Fixed16
{
	fixed16_mul(x, x)
}

pub fn fixed16_mul_result_to_int(x: Fixed16, y: Fixed16) -> i32
{
	(((x as i64) * (y as i64)) >> (FIXED16_BASE * 2)) as i32
}

pub fn fixed16_div(x: Fixed16, y: Fixed16) -> Fixed16
{
	(((x as i64) << FIXED16_BASE) / (y as i64)) as i32
}

pub fn fixed16_invert(x: Fixed16) -> Fixed16
{
	((1 << (FIXED16_BASE * 2)) / (x as i64)) as i32
}

// result= x * y / z
// It is more precise than fixed16_div(fixed16_mul(x, y), z).
pub fn fixed16_mul_div(x: Fixed16, y: Fixed16, z: Fixed16) -> Fixed16
{
	((x as i64) * (y as i64) / (z as i64)) as i32
}
