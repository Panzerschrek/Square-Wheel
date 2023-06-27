use std::io::{Read, Seek};

pub fn read_chunk<T: Copy, F: Read + Seek>(reader: &mut F, offset: u64, dst: &mut [T]) -> Result<(), std::io::Error>
{
	reader.seek(std::io::SeekFrom::Start(offset as u64))?;

	if dst.is_empty()
	{
		return Ok(());
	}

	let bytes = unsafe {
		std::slice::from_raw_parts_mut((&mut dst[0]) as *mut T as *mut u8, std::mem::size_of::<T>() * dst.len())
	};

	reader.read_exact(bytes)?;

	Ok(())
}

pub fn read_vector<T: Copy, F: Read + Seek>(
	reader: &mut F,
	offset: u64,
	num_elements: u32,
) -> Result<Vec<T>, std::io::Error>
{
	// TODO - use uninitiaized memory instead.
	let mut result = unsafe { vec![std::mem::zeroed::<T>(); num_elements as usize] };

	read_chunk(reader, offset, &mut result)?;

	Ok(result)
}
