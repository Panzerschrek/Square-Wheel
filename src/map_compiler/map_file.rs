type Vec2f = cgmath::Vector2<f32>;
type Vec3f = cgmath::Vector3<f32>;

pub struct BrushPlane
{
	pub vertices: [Vec3f; 3],
	pub texture: String,
	pub tc_offset: Vec2f,
	pub tc_scale: Vec2f,
	pub tc_angle: f32,
}

pub type Brush = Vec<BrushPlane>;

pub struct Entity
{
	pub brushes: Vec<Brush>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapFileParsed = Vec<Entity>;
