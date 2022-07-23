use super::{triangle_model::*, triangle_model_loading::*};
use common::{bbox::*, math_types::*};
use std::io::Read;

pub fn load_model_iqm(file_path: &std::path::Path) -> Result<Option<TriangleModel>, std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)?;

	let header_size = std::mem::size_of::<IQMHeader>();
	let mut header = unsafe { std::mem::zeroed::<IQMHeader>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut IQMHeader as *mut u8, header_size) };

	if file.read(header_bytes)? != header_size
	{
		println!("Can't read IQM header");
		return Ok(None);
	}

	if header.magic != IQM_MAGIC
	{
		println!("File is not a valid IQM model");
		return Ok(None);
	}
	if header.version != IQM_VERSION
	{
		println!(
			"Can't load incompatible IQM model version: {}, expected {}",
			header.version, IQM_VERSION
		);
		return Ok(None);
	}

	let meshes = load_meshes(&mut file, &header)?;

	let triangles = load_triangles(&mut file, &header)?;
	let vertices = load_vertices(&mut file, &header)?;

	let bounds = load_bounds(&mut file, &header)?;

	let frames_info = bounds
		.iter()
		.map(|b| TriangleModelFrameInfo {
			bbox: BBox::from_min_max(Vec3f::from(b.bbmins), Vec3f::from(b.bbmaxs)),
		})
		.collect();

	let triangles_transformed = triangles
		.iter()
		.map(|t| {
			[
				t.vertex[0] as VertexIndex,
				t.vertex[1] as VertexIndex,
				t.vertex[2] as VertexIndex,
			]
		})
		.collect();

	// TODO - export all meshes separately.
	// Now just load single mesh.
	let single_mesh = TriangleModelMesh {
		material_name: String::new(), // TODO
		triangles: triangles_transformed,
		vertex_data: VertexData::SkeletonAnimated(vertices),
		num_frames: header.num_frames,
	};

	// Add extra shift because we use texture coordinates floor, instead of linear OpenGL interpolation as Quake III does.
	let tc_shift = -Vec2f::new(0.5, 0.5);

	Ok(Some(TriangleModel {
		frames_info,
		tc_shift,
		meshes: vec![single_mesh],
	}))
}

fn load_meshes(file: &mut std::fs::File, header: &IQMHeader) -> Result<Vec<IQMMesh>, std::io::Error>
{
	// TODO - use uninitialized memory.
	let mut meshes = vec![IQMMesh::default(); header.num_meshes as usize];
	read_chunk(file, header.ofs_meshes as u64, &mut meshes)?;

	Ok(meshes)
}

fn load_triangles(file: &mut std::fs::File, header: &IQMHeader) -> Result<Vec<IQMTriangle>, std::io::Error>
{
	// TODO - use uninitialized memory.
	let mut triangles = vec![IQMTriangle::default(); header.num_triangles as usize];
	read_chunk(file, header.ofs_triangles as u64, &mut triangles)?;

	Ok(triangles)
}

fn load_vertices(file: &mut std::fs::File, header: &IQMHeader) -> Result<Vec<SkeletonAnimatedVertex>, std::io::Error>
{
	let vertex_arrays = load_vertex_arrays(file, header)?;

	let mut vertices = vec![
		SkeletonAnimatedVertex {
			tex_coord: [0.0, 0.0],
			position: Vec3f::zero(),
			normal: Vec3f::zero(),
			bones_description: [VertexBoneDescription {
				bone_index: 0,
				weight: 0
			}; 4]
		};
		header.num_vertexes as usize
	];

	// TODO - improve this, support other attributes and types.

	for vertex_array in &vertex_arrays
	{
		match vertex_array.type_
		{
			IQM_POSITION =>
			{
				if vertex_array.size == 3 && vertex_array.format == IQM_FLOAT
				{
					// TODO - use uninitialized memory.
					let mut positions = vec![Vec3f::zero(); vertices.len()];
					read_chunk(file, vertex_array.offset as u64, &mut positions)?;
					for (v, p) in vertices.iter_mut().zip(positions.iter())
					{
						v.position = *p;
					}
				}
			},
			IQM_TEXCOORD =>
			{
				if vertex_array.size == 2 && vertex_array.format == IQM_FLOAT
				{
					// TODO - use uninitialized memory.
					let mut tex_coords = vec![[0.0, 0.0]; vertices.len()];
					read_chunk(file, vertex_array.offset as u64, &mut tex_coords)?;
					for (v, tc) in vertices.iter_mut().zip(tex_coords.iter())
					{
						v.tex_coord = *tc;
					}
				}
			},
			IQM_NORMAL =>
			{
				if vertex_array.size == 3 && vertex_array.format == IQM_FLOAT
				{
					// TODO - use uninitialized memory.
					let mut normals = vec![Vec3f::zero(); vertices.len()];
					read_chunk(file, vertex_array.offset as u64, &mut normals)?;
					for (v, n) in vertices.iter_mut().zip(normals.iter())
					{
						v.normal = *n;
					}
				}
			},
			IQM_TANGENT =>
			{},
			IQM_BLENDINDEXES =>
			{
				if vertex_array.size == 4 && vertex_array.format == IQM_UBYTE
				{
					let zero_indexes: [u8; 4] = [0, 0, 0, 0];
					// TODO - use uninitialized memory.
					let mut indexes = vec![zero_indexes; vertices.len()];
					read_chunk(file, vertex_array.offset as u64, &mut indexes)?;
					for (v, index) in vertices.iter_mut().zip(indexes.iter())
					{
						for i in 0 .. 4
						{
							v.bones_description[i].bone_index = index[i];
						}
					}
				}
			},
			IQM_BLENDWEIGHTS =>
			{
				if vertex_array.size == 4 && vertex_array.format == IQM_UBYTE
				{
					let zero_weights: [u8; 4] = [0, 0, 0, 0];
					// TODO - use uninitialized memory.
					let mut weights = vec![zero_weights; vertices.len()];
					read_chunk(file, vertex_array.offset as u64, &mut weights)?;
					for (v, weight) in vertices.iter_mut().zip(weights.iter())
					{
						for i in 0 .. 4
						{
							v.bones_description[i].weight = weight[i];
						}
					}
				}
			},
			_ =>
			{},
		}
	}

	Ok(vertices)
}

fn load_vertex_arrays(file: &mut std::fs::File, header: &IQMHeader) -> Result<Vec<IQMVertexArray>, std::io::Error>
{
	// TODO - use uninitialized memory.
	let mut vertex_arrays = vec![IQMVertexArray::default(); header.num_vertexarrays as usize];
	read_chunk(file, header.ofs_vertexarrays as u64, &mut vertex_arrays)?;

	Ok(vertex_arrays)
}

fn load_bounds(file: &mut std::fs::File, header: &IQMHeader) -> Result<Vec<IQMBounds>, std::io::Error>
{
	// TODO - use uninitialized memory.
	let mut bounds = vec![IQMBounds::default(); header.num_frames as usize];
	read_chunk(file, header.ofs_bounds as u64, &mut bounds)?;

	Ok(bounds)
}

#[repr(C)]
#[derive(Debug)]
struct IQMHeader
{
	magic: [u8; 16],
	version: u32,
	filesize: u32,
	flags: u32,
	num_text: u32,
	ofs_text: u32,
	num_meshes: u32,
	ofs_meshes: u32,
	num_vertexarrays: u32,
	num_vertexes: u32,
	ofs_vertexarrays: u32,
	num_triangles: u32,
	ofs_triangles: u32,
	ofs_adjacency: u32,
	num_joints: u32,
	ofs_joints: u32,
	num_poses: u32,
	ofs_poses: u32,
	num_anims: u32,
	ofs_anims: u32,
	num_frames: u32,
	num_framechannels: u32,
	ofs_frames: u32,
	ofs_bounds: u32,
	num_comment: u32,
	ofs_comment: u32,
	num_extensions: u32,
	ofs_extensions: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMMesh
{
	name: u32,
	material: u32,
	first_vertex: u32,
	num_vertexes: u32,
	first_triangle: u32,
	num_triangles: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMBounds
{
	bbmins: [f32; 3],
	bbmaxs: [f32; 3],
	xyradius: f32,
	radius: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMTriangle
{
	vertex: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct IQMVertexArray
{
	type_: u32,
	flags: u32,
	format: u32,
	size: u32,
	offset: u32,
}

// const IQM_BYTE: u32 = 0;
const IQM_UBYTE: u32 = 1;
// const IQM_SHORT: u32 = 2;
// const IQM_USHORT: u32 = 3;
// const IQM_INT: u32 = 4;
// const IQM_UINT: u32 = 5;
// const IQM_HALF: u32 = 6;
const IQM_FLOAT: u32 = 7;
// const IQM_DOUBLE: u32 = 8;

const IQM_POSITION: u32 = 0;
const IQM_TEXCOORD: u32 = 1;
const IQM_NORMAL: u32 = 2;
const IQM_TANGENT: u32 = 3;
const IQM_BLENDINDEXES: u32 = 4;
const IQM_BLENDWEIGHTS: u32 = 5;
// const IQM_COLOR: u32 = 6;

const IQM_MAGIC: [u8; 16] = [
	'I' as u8, 'N' as u8, 'T' as u8, 'E' as u8, 'R' as u8, 'Q' as u8, 'U' as u8, 'A' as u8, 'K' as u8, 'E' as u8,
	'M' as u8, 'O' as u8, 'D' as u8, 'E' as u8, 'L' as u8, 0,
];
const IQM_VERSION: u32 = 2;
