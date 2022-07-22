use super::triangle_model::*;
use std::io::{Read, Seek};

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

	println!("iqm: {:?}", header);
	panic!("not implemented yet!");
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

const IQM_MAGIC: [u8; 16] = [
	'I' as u8, 'N' as u8, 'T' as u8, 'E' as u8, 'R' as u8, 'Q' as u8, 'U' as u8, 'A' as u8, 'K' as u8, 'E' as u8,
	'M' as u8, 'O' as u8, 'D' as u8, 'E' as u8, 'L' as u8, 0,
];
const IQM_VERSION: u32 = 2;
