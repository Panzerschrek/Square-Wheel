use common::{bbox::*, math_types::*};

pub struct TriangleModel
{
	pub animations: Vec<TriangleModelAnimation>,
	pub frames_info: Vec<TriangleModelFrameInfo>,
	pub meshes: Vec<TriangleModelMesh>,
	// Non-empty for skeleton-animated models.
	pub bones: Vec<TriangleModelBoneInfo>,
	// Frame info for all bones of specific frame. Has size = num_frames * num_bones.
	pub frame_bones: Vec<TriangleModelBoneFrame>,
	pub tc_shift: Vec2f,
}

pub struct TriangleModelAnimation
{
	pub name: String,
	pub start_frame: u32,
	pub num_frames: u32,
	pub frames_per_second: f32,
	pub looped: bool,
}

pub struct TriangleModelFrameInfo
{
	pub bbox: BBox,
}

pub struct TriangleModelBoneInfo
{
	pub name: String,
	pub parent: u32, // invalid index if has no parent
}

#[derive(Copy, Clone)]
pub struct TriangleModelBoneFrame
{
	pub matrix: Mat4f,
}

pub struct TriangleModelMesh
{
	pub name: String,
	pub material_name: String,
	pub triangles: Vec<Triangle>,
	pub num_frames: u32,

	pub vertex_data: VertexData,
}

// TODO - maybe use 8-byte alignment for triangle structure?
pub type Triangle = [VertexIndex; 3];

pub type VertexIndex = u16;

pub enum VertexData
{
	VertexAnimated(VertexAnimatedVertexData),
	SkeletonAnimated(Vec<SkeletonAnimatedVertex>),
}

pub struct VertexAnimatedVertexData
{
	pub constant: Vec<VertexAnimatedVertexConstant>,
	// size = number of vertices * number of frames.
	pub variable: Vec<VertexAnimatedVertexVariable>,
}

#[derive(Copy, Clone)]
pub struct VertexAnimatedVertexConstant
{
	pub tex_coord: [f32; 2],
}

#[derive(Copy, Clone)]
pub struct VertexAnimatedVertexVariable
{
	pub position: Vec3f,
	pub normal: Vec3f,
}

#[derive(Debug, Copy, Clone)]
pub struct SkeletonAnimatedVertex
{
	pub tex_coord: [f32; 2],
	pub position: Vec3f,
	pub normal: Vec3f,
	pub bones_description: [VertexBoneDescription; 4],
}

#[derive(Debug, Copy, Clone)]
pub struct VertexBoneDescription
{
	pub bone_index: u8,
	pub weight: u8,
}

pub const MAX_TRIANGLE_MODEL_BONES: usize = 255;
