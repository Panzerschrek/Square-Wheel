use common::{bbox::*, math_types::*};

pub struct TriangleModel
{
	pub frames_info: Vec<TriangleModelFrameInfo>,
	pub meshes: Vec<TriangleModelMesh>,
	// Non-empty for skeleton-animated models.
	pub bones: Vec<TriangleModelBoneInfo>,
	// Frame info for all bones of specific frame. Has sie = num_frames * num_bones.
	pub bone_frames: Vec<TriangleModelBoneFrame>,
	pub tc_shift: Vec2f,
}

// TODO -  support also skeleton-based animation.

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
	// size = number of vertices * numvber of frames.
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
