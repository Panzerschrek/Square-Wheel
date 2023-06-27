use super::{bsp_map_compact::*, lightmap_compression::*};
use std::{
	io::{Read, Seek, Write},
	path::{Path, PathBuf},
};

pub const BSP_MAP_FILE_EXTENSION: &str = "sqwm";

pub fn normalize_bsp_map_file_path(mut file_path: PathBuf) -> PathBuf
{
	if file_path.extension().is_some()
	{
		return file_path;
	}

	file_path.set_extension(BSP_MAP_FILE_EXTENSION);
	file_path
}

pub fn save_map(bsp_map: &BSPMap, file_path: &Path) -> Result<(), std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path)?;
	save_map_into_writer(bsp_map, &mut file)?;
	file.sync_data()?;
	Ok(())
}

pub fn save_map_into_writer<F: Write + Seek>(bsp_map: &BSPMap, writer: &mut F) -> Result<(), std::io::Error>
{
	// Just write raw bytes of map structs into file.
	// This is fine until we use plain structs and load this file on machined with same bytes order.

	let mut header = BspMapHeader {
		id: BSP_MAP_ID,
		version: BSP_MAP_VERSION,
		lumps: unsafe { std::mem::zeroed() },
	};

	// Write header first time to advance current file position.
	let header_bytes = unsafe {
		std::slice::from_raw_parts(
			(&header) as *const BspMapHeader as *const u8,
			std::mem::size_of::<BspMapHeader>(),
		)
	};
	writer.write_all(header_bytes)?;

	let mut offset = header_bytes.len();

	write_lump(&bsp_map.nodes, writer, &mut header.lumps[LUMP_NODES], &mut offset)?;
	write_lump(&bsp_map.leafs, writer, &mut header.lumps[LUMP_LEAFS], &mut offset)?;
	write_lump(&bsp_map.polygons, writer, &mut header.lumps[LUMP_POLYGONS], &mut offset)?;
	write_lump(&bsp_map.portals, writer, &mut header.lumps[LUMP_PORTALS], &mut offset)?;
	write_lump(
		&bsp_map.leafs_portals,
		writer,
		&mut header.lumps[LUMP_LEAFS_PORTALS],
		&mut offset,
	)?;
	write_lump(&bsp_map.vertices, writer, &mut header.lumps[LUMP_VERTICES], &mut offset)?;
	write_lump(&bsp_map.textures, writer, &mut header.lumps[LUMP_TEXTURES], &mut offset)?;
	write_lump(
		&bsp_map.submodels,
		writer,
		&mut header.lumps[LUMP_SUBMODELS],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.submodels_bsp_nodes,
		writer,
		&mut header.lumps[LUMP_SUBMODELS_BSP_NODES],
		&mut offset,
	)?;
	write_lump(&bsp_map.entities, writer, &mut header.lumps[LUMP_ENTITIES], &mut offset)?;
	write_lump(
		&bsp_map.key_value_pairs,
		writer,
		&mut header.lumps[LUMP_KEY_VALUE_PAIRS],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.strings_data,
		writer,
		&mut header.lumps[LUMP_STRINGS_DATA],
		&mut offset,
	)?;
	write_lump(
		&bsp_map
			.lightmaps_data
			.iter()
			.map(LightmapElementCompressed::compress)
			.collect::<Vec<_>>(),
		writer,
		&mut header.lumps[LUMP_LIGHTMAPS_DATA],
		&mut offset,
	)?;
	write_lump(
		&bsp_map
			.directional_lightmaps_data
			.iter()
			.map(DirectionalLightmapElementCompressed::compress)
			.collect::<Vec<_>>(),
		writer,
		&mut header.lumps[LUMP_DIRECTIONAL_LIGHTMAPS_DATA],
		&mut offset,
	)?;
	write_single_element_lump(
		&bsp_map.light_grid_header,
		writer,
		&mut header.lumps[LUMP_LIGHT_GRID_HEADER],
		&mut offset,
	)?;
	write_lump(
		&bsp_map.light_grid_columns,
		writer,
		&mut header.lumps[LUMP_LIGHT_GRID_COLUMNS],
		&mut offset,
	)?;
	write_lump(
		&bsp_map
			.light_grid_samples
			.iter()
			.map(LightGridElementCompressed::compress)
			.collect::<Vec<_>>(),
		writer,
		&mut header.lumps[LUMP_LIGHT_GRID_SAMPLES],
		&mut offset,
	)?;

	// Write header again to update lumps headers.
	writer.seek(std::io::SeekFrom::Start(0))?;
	writer.write_all(header_bytes)?;

	Ok(())
}

pub fn load_map(file_path: &Path) -> Result<Option<BSPMap>, std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)?;

	load_map_from_reader(&mut file)
}

pub fn load_map_from_reader<F: Read + Seek>(reader: &mut F) -> Result<Option<BSPMap>, std::io::Error>
{
	let header_size = std::mem::size_of::<BspMapHeader>();
	let mut header = unsafe { std::mem::zeroed::<BspMapHeader>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut BspMapHeader as *mut u8, header_size) };

	if reader.read(header_bytes)? != header_size
	{
		println!("Can't read BSP map header");
		return Ok(None);
	}

	if header.id != BSP_MAP_ID
	{
		println!("File is not a valid BSP map");
		return Ok(None);
	}
	if header.version != BSP_MAP_VERSION
	{
		println!(
			"Can't load incompatible map version: {}, expected {}",
			header.version, BSP_MAP_VERSION
		);
		return Ok(None);
	}

	let map = BSPMap {
		nodes: read_lump(reader, &header.lumps[LUMP_NODES])?,
		leafs: read_lump(reader, &header.lumps[LUMP_LEAFS])?,
		polygons: read_lump(reader, &header.lumps[LUMP_POLYGONS])?,
		portals: read_lump(reader, &header.lumps[LUMP_PORTALS])?,
		leafs_portals: read_lump(reader, &header.lumps[LUMP_LEAFS_PORTALS])?,
		vertices: read_lump(reader, &header.lumps[LUMP_VERTICES])?,
		textures: read_lump(reader, &header.lumps[LUMP_TEXTURES])?,
		submodels: read_lump(reader, &header.lumps[LUMP_SUBMODELS])?,
		submodels_bsp_nodes: read_lump(reader, &header.lumps[LUMP_SUBMODELS_BSP_NODES])?,
		entities: read_lump(reader, &header.lumps[LUMP_ENTITIES])?,
		key_value_pairs: read_lump(reader, &header.lumps[LUMP_KEY_VALUE_PAIRS])?,
		strings_data: read_lump(reader, &header.lumps[LUMP_STRINGS_DATA])?,
		lightmaps_data: read_lump(reader, &header.lumps[LUMP_LIGHTMAPS_DATA])?
			.iter()
			.map(LightmapElementCompressed::decompress)
			.collect(),
		directional_lightmaps_data: read_lump(reader, &header.lumps[LUMP_DIRECTIONAL_LIGHTMAPS_DATA])?
			.iter()
			.map(DirectionalLightmapElementCompressed::decompress)
			.collect(),
		light_grid_header: read_single_element_lump(reader, &header.lumps[LUMP_LIGHT_GRID_HEADER])?,
		light_grid_columns: read_lump(reader, &header.lumps[LUMP_LIGHT_GRID_COLUMNS])?,
		light_grid_samples: read_lump(reader, &header.lumps[LUMP_LIGHT_GRID_SAMPLES])?
			.iter()
			.map(LightGridElementCompressed::decompress)
			.collect(),
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

const BSP_MAP_ID: [u8; 4] = *b"SqwM";
const BSP_MAP_VERSION: u32 = 11; // Change each time when format is changed!

const MAX_LUMPS: usize = 32;

const LUMP_NODES: usize = 0;
const LUMP_LEAFS: usize = 1;
const LUMP_POLYGONS: usize = 2;
const LUMP_PORTALS: usize = 3;
const LUMP_LEAFS_PORTALS: usize = 4;
const LUMP_VERTICES: usize = 5;
const LUMP_TEXTURES: usize = 6;
const LUMP_SUBMODELS: usize = 7;
const LUMP_SUBMODELS_BSP_NODES: usize = 8;
const LUMP_ENTITIES: usize = 9;
const LUMP_KEY_VALUE_PAIRS: usize = 10;
const LUMP_STRINGS_DATA: usize = 11;
const LUMP_LIGHTMAPS_DATA: usize = 12;
const LUMP_DIRECTIONAL_LIGHTMAPS_DATA: usize = 13;
const LUMP_LIGHT_GRID_HEADER: usize = 14;
const LUMP_LIGHT_GRID_COLUMNS: usize = 15;
const LUMP_LIGHT_GRID_SAMPLES: usize = 16;

fn write_lump<T, F: Write>(
	data: &[T],
	writer: &mut F,
	lump: &mut Lump,
	offset: &mut usize,
) -> Result<(), std::io::Error>
{
	let element_size = std::mem::size_of::<T>();

	lump.offset = (*offset) as u32;
	lump.element_size = element_size as u32;
	lump.element_count = data.len() as u32;

	if data.is_empty()
	{
		return Ok(());
	}

	let bytes = unsafe { std::slice::from_raw_parts((&data[0]) as *const T as *const u8, element_size * data.len()) };
	writer.write_all(bytes)?;

	*offset += bytes.len();

	Ok(())
}

fn read_lump<T: Copy, F: Read + Seek>(file: &mut F, lump: &Lump) -> Result<Vec<T>, std::io::Error>
{
	let element_size = std::mem::size_of::<T>();
	if lump.element_size != (element_size as u32)
	{
		// TODO - generate error?
		println!("Wrong element size: {}, expected {}", lump.element_size, element_size);
		return Ok(Vec::new());
	}

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

fn write_single_element_lump<T, F: Write>(
	element: &T,
	writer: &mut F,
	lump: &mut Lump,
	offset: &mut usize,
) -> Result<(), std::io::Error>
{
	let element_size = std::mem::size_of::<T>();

	lump.offset = (*offset) as u32;
	lump.element_size = element_size as u32;
	lump.element_count = 1;

	let bytes = unsafe { std::slice::from_raw_parts(element as *const T as *const u8, element_size) };
	writer.write_all(bytes)?;

	*offset += bytes.len();

	Ok(())
}

fn read_single_element_lump<T: Copy, F: Read + Seek>(reader: &mut F, lump: &Lump) -> Result<T, std::io::Error>
{
	let mut result = unsafe { std::mem::zeroed::<T>() };

	let element_size = std::mem::size_of::<T>();
	if lump.element_size != (element_size as u32)
	{
		// TODO - generate error?
		println!("Wrong element size: {}, expected {}", lump.element_size, element_size);
		return Ok(result);
	}

	// TODO - what if seek fails?
	reader.seek(std::io::SeekFrom::Start(lump.offset as u64))?;

	let bytes = unsafe { std::slice::from_raw_parts_mut((&mut result) as *mut T as *mut u8, std::mem::size_of::<T>()) };
	reader.read_exact(bytes)?;

	Ok(result)
}
