use super::triangle_model::*;
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

	file.seek(std::io::SeekFrom::Start(header.lump_meshes as u64))?;
	let mut meshes = Vec::with_capacity(header.num_meshes as usize);
	for i in 0 .. header.num_meshes
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

		if let Some(mesh) = load_md3_mesh(&mesh_header, &mut file)?
		{
			meshes.push(mesh);
		}
	}

	Ok(Some(TriangleModel { meshes }))
}

fn load_md3_mesh(src_mesh: &Md3Mesh, file: &mut std::fs::File) -> Result<Option<TriangleModelMesh>, std::io::Error>
{
	if src_mesh.ident != MD3_ID
	{
		println!("Mesh is not a valid MD3 model");
		return Ok(None);
	}

	Ok(None)
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

	lump_elements: u32,
	lump_shaders: u32,
	lump_texcoords: u32,
	lump_framevertices: u32,
	lump_end: u32,
}

#[repr(C)]
struct Md3Shader
{
	name: [u8; MAX_QPATH],
	index: u32,
}

#[repr(C)]
struct Md3Tag
{
	name: [u8; MAX_QPATH],
	origin: [f32; 3],
	rotation_matrix: [f32; 9],
}

#[repr(C)]
struct Md3Frame
{
	mins: [f32; 3],
	maxs: [f32; 3],
	origin: [f32; 3],
	radius: f32,
	name: [u8; 16],
}

#[repr(C)]
struct Md3Vertex
{
	origin: [i16; 3],
	normal_pitch_yaw: i16,
}

const MAX_QPATH: usize = 64;
const MD3_ID: [u8; 4] = ['I' as u8, 'D' as u8, 'P' as u8, '3' as u8];
const MD3_VERSION: u32 = 15;
