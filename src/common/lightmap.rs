use super::{bsp_map_compact, math_types::*};

// If this chaged, map file version must be changed too!
pub const LIGHTMAP_SCALE_LOG2: u32 = 4;
pub const LIGHTMAP_SCALE: u32 = 1 << LIGHTMAP_SCALE_LOG2;

// Limit used for polygons splitting.
// Actual size may be a little big greater.
pub const MAX_LIGHTMAP_SIZE: u32 = 17;

pub fn get_polygon_lightmap_size(polygon: &bsp_map_compact::Polygon) -> [u32; 2]
{
	[
		get_lightmap_size(polygon.tex_coord_min[0], polygon.tex_coord_max[0]),
		get_lightmap_size(polygon.tex_coord_min[1], polygon.tex_coord_max[1]),
	]
}

pub fn get_lightmap_size(tc_min: i32, tc_max: i32) -> u32
{
	// If this chaged, map file version must be changed too!
	debug_assert!(tc_min < tc_max);
	let result = (((tc_max + ((LIGHTMAP_SCALE - 1) as i32)) >> LIGHTMAP_SCALE_LOG2) - (tc_min >> LIGHTMAP_SCALE_LOG2) +
		1) as u32;
	debug_assert!(result >= 2);
	result
}

pub struct LightmapBasis
{
	pub pos: Vec3f,
	pub u_vec: Vec3f,
	pub v_vec: Vec3f,
}

pub fn calculate_lightmap_basis(polygon: &bsp_map_compact::Polygon) -> LightmapBasis
{
	// Calculate inverse matrix for tex_coord equation and plane equation in order to calculate world position for UV.

	let tc_basis_scale = 1.0 / (LIGHTMAP_SCALE as f32);
	let tex_coord_basis = Mat4f::from_cols(
		polygon.tex_coord_equation[0]
			.vec
			.extend(polygon.tex_coord_equation[0].dist) *
			tc_basis_scale,
		polygon.tex_coord_equation[1]
			.vec
			.extend(polygon.tex_coord_equation[1].dist) *
			tc_basis_scale,
		polygon.plane.vec.extend(-polygon.plane.dist),
		Vec4f::new(0.0, 0.0, 0.0, 1.0),
	);
	let tex_coord_basis_inverted = tex_coord_basis.transpose().invert().unwrap(); // TODO - avoid "unwrap"?

	let u_vec = tex_coord_basis_inverted.x.truncate();
	let v_vec = tex_coord_basis_inverted.y.truncate();

	let pos = tex_coord_basis_inverted.w.truncate() +
		u_vec * ((polygon.tex_coord_min[0] >> LIGHTMAP_SCALE_LOG2) as f32) +
		v_vec * ((polygon.tex_coord_min[1] >> LIGHTMAP_SCALE_LOG2) as f32);

	LightmapBasis { pos, u_vec, v_vec }
}
