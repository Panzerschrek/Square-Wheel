use std::io::{Read, Seek};

pub fn read_chunk<T: Copy>(file: &mut std::fs::File, offset: u64, dst: &mut [T]) -> Result<(), std::io::Error>
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
