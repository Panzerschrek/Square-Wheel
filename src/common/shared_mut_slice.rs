// Wrapper class for sharing mutable slice across several threads.
// It's programmer responcibility to make sure concurrent access for same slice elements is not actually happens.

#[derive(Copy, Clone)]
pub struct SharedMutSlice<T>
{
	ptr: *mut T,
	len: usize,
}

unsafe impl<T> Send for SharedMutSlice<T> {}
unsafe impl<T> Sync for SharedMutSlice<T> {}

impl<T> SharedMutSlice<T>
{
	pub fn new(slice: &mut [T]) -> Self
	{
		Self {
			ptr: slice.as_mut_ptr(),
			len: slice.len(),
		}
	}

	#[allow(clippy::mut_from_ref)]
	pub unsafe fn get(&self) -> &mut [T]
	{
		std::slice::from_raw_parts_mut(self.ptr, self.len)
	}
}
