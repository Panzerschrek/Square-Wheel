use super::triangle_model::*;
use common::math_types::*;
use std::io::{Read, Seek};

pub fn load_model_md3(file_path: &std::path::Path) -> Result<Option<TriangleModel>, std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)?;

	let header_size = std::mem::size_of::<Md3Header>();
	let mut header = unsafe { std::mem::zeroed::<Md3Header>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut Md3Header as *mut u8, header_size) };

	if file.read(header_bytes)? != header_size
	{
		println!("Can't read Md3 header");
		return Ok(None);
	}

	if header.ident != MD3_ID
	{
		println!("File is not a valid MD3 model");
		return Ok(None);
	}
	if header.version != MD3_VERSION
	{
		println!(
			"Can't load incompatible MD3 model version: {}, expected {}",
			header.version, MD3_VERSION
		);
		return Ok(None);
	}

	let mut frames_src = vec![Md3Frame::default(); header.num_frames as usize];
	read_chunk(&mut file, header.lump_frameinfo as u64, &mut frames_src)?;

	let mut tags_src = vec![
		Md3Tag {
			name: [0; MAX_QPATH],
			origin: [0.0; 3],
			rotation_matrix: [0.0; 9]
		};
		header.num_tags as usize
	];
	read_chunk(&mut file, header.lump_tags as u64, &mut tags_src)?;

	file.seek(std::io::SeekFrom::Start(header.lump_meshes as u64))?;
	let mut meshes = Vec::with_capacity(header.num_meshes as usize);
	let mut offset = header.lump_meshes as u64;
	for _i in 0 .. header.num_meshes
	{
		let mesh_header_size = std::mem::size_of::<Md3Mesh>();
		let mut mesh_header = unsafe { std::mem::zeroed::<Md3Mesh>() };
		let mesh_header_bytes =
			unsafe { std::slice::from_raw_parts_mut((&mut mesh_header) as *mut Md3Mesh as *mut u8, mesh_header_size) };

		if file.read(mesh_header_bytes)? != mesh_header_size
		{
			println!("Can't read Md3 mesh header");
			break;
		}

		if let Some(mesh) = load_md3_mesh(&mesh_header, offset, &mut file)?
		{
			meshes.push(mesh);
		}

		offset += mesh_header.lump_end as u64;
		file.seek(std::io::SeekFrom::Start(offset as u64))?;
	}

	Ok(Some(TriangleModel { meshes }))
}

fn load_md3_mesh(
	src_mesh: &Md3Mesh,
	mesh_offset: u64,
	file: &mut std::fs::File,
) -> Result<Option<TriangleModelMesh>, std::io::Error>
{
	if src_mesh.ident != MD3_ID
	{
		println!("Mesh is not a valid MD3 model");
		return Ok(None);
	}

	let mut triangles_src = vec![Md3Triangle::default(); src_mesh.num_triangles as usize];
	read_chunk(file, src_mesh.lump_triangles as u64 + mesh_offset, &mut triangles_src)?;

	let mut tex_coords_src = vec![Md3TexCoord::default(); src_mesh.num_vertices as usize];
	read_chunk(file, src_mesh.lump_texcoords as u64 + mesh_offset, &mut tex_coords_src)?;

	let mut frames_src = vec![Md3Vertex::default(); (src_mesh.num_vertices * src_mesh.num_frames) as usize];
	read_chunk(file, src_mesh.lump_framevertices as u64 + mesh_offset, &mut frames_src)?;

	let mut shaders_src = vec![
		Md3Shader {
			name: [0; MAX_QPATH],
			index: 0
		};
		src_mesh.num_shaders as usize
	];
	read_chunk(file, src_mesh.lump_shaders as u64 + mesh_offset, &mut shaders_src)?;

	let triangles = triangles_src
		.iter()
		.map(|x| [x[0] as VertexIndex, x[1] as VertexIndex, x[2] as VertexIndex])
		.collect();

	let vertex_data_constant = tex_coords_src
		.iter()
		.map(|&tex_coord| VertexAnimatedVertexConstant { tex_coord })
		.collect();

	// TODO - transform coordinates properly.
	let vertex_data_variable = frames_src
		.iter()
		.map(|v| VertexAnimatedVertexVariable {
			position: Vec3f::new(v.origin[0] as f32, v.origin[1] as f32, v.origin[2] as f32) * MD3_COORD_SCALE,
			normal: decompress_normal(v.normal_pitch_yaw),
		})
		.collect();

	Ok(Some(TriangleModelMesh {
		material_name: String::new(/*TODO*/),
		triangles,
		num_frames: src_mesh.num_frames,
		vertex_data_constant,
		vertex_data_variable,
	}))
}

fn read_chunk<T: Copy>(file: &mut std::fs::File, offset: u64, dst: &mut [T]) -> Result<(), std::io::Error>
{
	file.seek(std::io::SeekFrom::Start(offset as u64))?;

	if dst.is_empty()
	{
		return Ok(());
	}

	let bytes = unsafe {
		std::slice::from_raw_parts_mut((&mut dst[0]) as *mut T as *mut u8, std::mem::size_of::<T>() * dst.len())
	};

	file.read_exact(bytes)?;

	Ok(())
}

fn decompress_normal(normal_pitch_yaw: i16) -> Vec3f
{
	let scale = std::f32::consts::TAU / 256.0;
	let pitch = (normal_pitch_yaw & 255) as f32 * scale;
	let yaw = ((normal_pitch_yaw >> 8) & 255) as f32 * scale;

	let pitch_sin = pitch.sin();
	Vec3f::new(pitch_sin * yaw.cos(), pitch_sin * yaw.sin(), pitch.cos())
}

#[repr(C)]
struct Md3Header
{
	ident: [u8; 4],
	version: u32,
	name: [u8; MAX_QPATH],
	flags: u32,

	num_frames: u32,
	num_tags: u32,
	num_meshes: u32,
	num_skins: u32,

	lump_frameinfo: u32,
	lump_tags: u32,
	lump_meshes: u32,
	lump_end: u32,
}

#[repr(C)]
struct Md3Mesh
{
	ident: [u8; 4],
	name: [u8; MAX_QPATH],
	flags: u32,

	num_frames: u32,
	num_shaders: u32,
	num_vertices: u32,
	num_triangles: u32,

	lump_triangles: u32,
	lump_shaders: u32,
	lump_texcoords: u32,
	lump_framevertices: u32,
	lump_end: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Md3Shader
{
	name: [u8; MAX_QPATH],
	index: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Md3Tag
{
	name: [u8; MAX_QPATH],
	origin: [f32; 3],
	rotation_matrix: [f32; 9],
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Md3Frame
{
	mins: [f32; 3],
	maxs: [f32; 3],
	origin: [f32; 3],
	radius: f32,
	name: [u8; 16],
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Md3Vertex
{
	origin: [i16; 3],
	normal_pitch_yaw: i16,
}

type Md3Triangle = [u32; 3];
type Md3TexCoord = [f32; 2];

const MAX_QPATH: usize = 64;
const MD3_ID: [u8; 4] = ['I' as u8, 'D' as u8, 'P' as u8, '3' as u8];
const MD3_VERSION: u32 = 15;
const MD3_COORD_SCALE: f32 = 1.0 / 64.0;
