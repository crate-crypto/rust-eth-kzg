use crate::CResultStatus;

// Helper methods for dereferencing raw pointers and writing to slices
//
pub(crate) fn deref_mut<'a, T>(ptr: *mut T) -> Result<&'a mut T, CResultStatus> {
    unsafe { ptr.as_mut().map_or(Err(CResultStatus::Err), |p| Ok(p)) }
}
pub(crate) fn deref_const<'a, T>(ptr: *const T) -> Result<&'a T, CResultStatus> {
    unsafe { ptr.as_ref().map_or(Err(CResultStatus::Err), |p| Ok(p)) }
}
pub(crate) fn dereference_to_vec_of_slices<'a>(
    vec_vec: *mut *mut u8,
    outer_len: usize,
) -> Result<Vec<&'a mut u8>, CResultStatus> {
    // Ensure we are working with raw pointers, thus we need to use unsafe
    unsafe {
        // Convert the outer pointer to a slice of raw pointers
        if vec_vec.is_null() {
            return Err(CResultStatus::Err);
        }
        let vec_slice: &mut [*mut u8] = std::slice::from_raw_parts_mut(vec_vec, outer_len);

        // Convert each inner pointer to a slice of u8
        let mut result: Vec<&mut u8> = Vec::with_capacity(outer_len);
        for ptr in vec_slice.into_iter() {
            result.push(deref_mut(*ptr)?);
        }

        Ok(result)
    }
}
pub(crate) fn dereference_to_vec_of_slices_const<'a>(
    vec_vec: *const *const u8,
    outer_len: usize,
    inner_len: usize,
) -> Result<Vec<&'a [u8]>, CResultStatus> {
    // Ensure we are working with raw pointers, thus we need to use unsafe
    unsafe {
        // Convert the outer pointer to a slice of raw pointers
        if vec_vec.is_null() {
            return Err(CResultStatus::Err);
        }
        let vec_slice: &[*const u8] = std::slice::from_raw_parts(vec_vec, outer_len);

        // Convert each inner pointer to a slice of u8
        let mut result: Vec<&[u8]> = Vec::with_capacity(outer_len);
        for ptr in vec_slice.into_iter() {
            result.push(create_slice_view(deref_const(*ptr)?, inner_len));
        }

        Ok(result)
    }
}

// TODO: We could return the number of bytes written to the C function so they can check if the length is correct.
pub(crate) fn write_to_slice<T: Copy>(ptr: &mut T, data: &[T]) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, data.len()) };
    slice.copy_from_slice(data);
}
// Note: If `ptr` points to a slice that is more than `len`
// This method will not panic and will instead truncate the memory region.
pub(crate) fn create_slice_view<'a, T>(ptr: &T, len: usize) -> &'a [T] {
    unsafe { std::slice::from_raw_parts(ptr, len) }
}
