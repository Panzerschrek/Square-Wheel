use common::math_types::*;

pub struct TriangleModel
{
	pub meshes: Vec<TriangleModelMesh>,
}

// TODO -  support also skeleton-based animation.

pub struct TriangleModelMesh
{
	pub material_name: String,
	pub triangles: Vec<Triangle>,
	pub num_frames: u32,

	pub vertex_data_constant: Vec<VertexAnimatedVertexConstant>,
	// size = number of vertices * numvber of frames.
	pub vertex_data_variable: Vec<VertexAnimatedVertexVariable>,
}

// TODO - maybe use 8-byte alignment for triangle structure?
pub type Triangle = [VertexIndex; 3];

pub type VertexIndex = u16;

pub struct VertexAnimatedVertexConstant
{
	pub tex_coord: [f32; 2],
}

pub struct VertexAnimatedVertexVariable
{
	pub position: Vec3f,
	pub normal: Vec3f,
}
