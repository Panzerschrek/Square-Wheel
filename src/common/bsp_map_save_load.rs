use super::bsp_map_compact::*;
use std::{
	io::{Read, Seek, Write},
	path::Path,
};

// Just write raw bytes of map structs into file.
// This is fine until we use plain structs and load this file on machined with same bytes order.

pub fn save_map(bsp_map: &BSPMap, file_path: &Path) -> Result<(), std::io::Error>
{
	let mut header = BspMapHeader {
		id: BSP_MAP_ID,
		version: BSP_MAP_VERSION,
		lumps: unsafe { std::mem::zeroed() },
	};

	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path)?;

	// Write header first time to advance current file position.
	let header_bytes = unsafe {
		std::slice::from_raw_parts(
			(&header) as *const BspMapHeader as *const u8,
			std::mem::size_of::<BspMapHeader>(),
		)
	};
	file.write(header_bytes)?;

	let mut offset = header_bytes.len();

	write_lump(&bsp_map.nodes, &mut file, &mut header.lumps[LUMP_NODES], &mut offset)?;
	write_lump(&bsp_map.leafs, &mut file, &mut header.lumps[LUMP_LEAFS], &mut offset)?;
	write_lump(
		&bsp_map.polygons,
		&mut file,
		&mut header.lumps[LUMP_POLYGONS],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.portals,
		&mut file,
		&mut header.lumps[LUMP_PORTALS],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.leafs_portals,
		&mut file,
		&mut header.lumps[LUMP_LEAFS_PORTALS],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.vertices,
		&mut file,
		&mut header.lumps[LUMP_VERTICES],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.textures,
		&mut file,
		&mut header.lumps[LUMP_TEXTURES],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.submodels,
		&mut file,
		&mut header.lumps[LUMP_SUBMODELS],
		&mut offset,
	)?;

	// Write header again to update lumps headers.
	file.seek(std::io::SeekFrom::Start(0))?;
	file.write(header_bytes)?;
	file.sync_data()?;

	Ok(())
}

pub fn load_map(file_path: &Path) -> Result<Option<BSPMap>, std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)?;

	let header_size = std::mem::size_of::<BspMapHeader>();
	let mut header = unsafe { std::mem::zeroed::<BspMapHeader>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut BspMapHeader as *mut u8, header_size) };

	if file.read(header_bytes)? != header_size
	{
		// TODO - print some message?
		return Ok(None);
	}

	let map = BSPMap {
		nodes: read_lump(&mut file, &header.lumps[LUMP_NODES])?,
		leafs: read_lump(&mut file, &header.lumps[LUMP_LEAFS])?,
		polygons: read_lump(&mut file, &header.lumps[LUMP_POLYGONS])?,
		portals: read_lump(&mut file, &header.lumps[LUMP_PORTALS])?,
		leafs_portals: read_lump(&mut file, &header.lumps[LUMP_LEAFS_PORTALS])?,
		vertices: read_lump(&mut file, &header.lumps[LUMP_VERTICES])?,
		textures: read_lump(&mut file, &header.lumps[LUMP_TEXTURES])?,
		submodels: read_lump(&mut file, &header.lumps[LUMP_SUBMODELS])?,
	};

	Ok(Some(map))
}

#[repr(C)]
struct BspMapHeader
{
	id: [u8; 4],
	version: u32,
	lumps: [Lump; MAX_LUMPS],
}

#[repr(C)]
struct Lump
{
	offset: u32,
	element_size: u32,
	element_count: u32,
}

const BSP_MAP_ID: [u8; 4] = ['S' as u8, 'q' as u8, 'w' as u8, 'M' as u8];
const BSP_MAP_VERSION: u32 = 1; // Change each time when format is changed!

const MAX_LUMPS: usize = 16;

const LUMP_NODES: usize = 0;
const LUMP_LEAFS: usize = 1;
const LUMP_POLYGONS: usize = 2;
const LUMP_PORTALS: usize = 3;
const LUMP_LEAFS_PORTALS: usize = 4;
const LUMP_VERTICES: usize = 5;
const LUMP_TEXTURES: usize = 6;
const LUMP_SUBMODELS: usize = 7;

// Returns number of bytes written.
fn write_lump<T>(
	data: &[T],
	file: &mut std::fs::File,
	lump: &mut Lump,
	offset: &mut usize,
) -> Result<(), std::io::Error>
{
	if data.is_empty()
	{
		return Ok(());
	}

	let bytes = unsafe {
		std::slice::from_raw_parts(
			(&data[0]) as *const T as *const u8,
			std::mem::size_of::<T>() * data.len(),
		)
	};
	file.write_all(bytes)?;

	lump.offset = (*offset) as u32;
	lump.element_size = std::mem::size_of::<T>() as u32;
	lump.element_count = data.len() as u32;
	*offset += bytes.len();

	Ok(())
}

fn read_lump<T: Copy>(file: &mut std::fs::File, lump: &Lump) -> Result<Vec<T>, std::io::Error>
{
	let mut result = vec![unsafe { std::mem::zeroed::<T>() }; lump.element_count as usize];
	if result.is_empty()
	{
		return Ok(result);
	}

	// TODO - what if seek fails?
	file.seek(std::io::SeekFrom::Start(lump.offset as u64))?;

	let bytes = unsafe {
		std::slice::from_raw_parts_mut(
			(&mut result[0]) as *mut T as *mut u8,
			std::mem::size_of::<T>() * result.len(),
		)
	};
	file.read_exact(bytes)?;

	Ok(result)
}
