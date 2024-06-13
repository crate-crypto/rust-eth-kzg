use crate::CResultStatus;

// Helper methods for dereferencing raw pointers and writing to slices
//
pub(crate) fn deref_mut<'a, T>(ptr: *mut T) -> Result<&'a mut T, CResultStatus> {
    unsafe { ptr.as_mut().map_or(Err(CResultStatus::Err), |p| Ok(p)) }
}
pub(crate) fn deref_const<'a, T>(ptr: *const T) -> Result<&'a T, CResultStatus> {
    unsafe { ptr.as_ref().map_or(Err(CResultStatus::Err), |p| Ok(p)) }
}
pub(crate) fn dereference_to_vec_of_slices<'a, T>(
    vec_vec: *mut *mut T,
    outer_len: usize,
) -> Result<Vec<&'a mut T>, CResultStatus> {
    if outer_len == 0 {
        return Ok(Vec::new());
    }
    // Ensure we are working with raw pointers, thus we need to use unsafe
    unsafe {
        // Convert the outer pointer to a slice of raw pointers
        if vec_vec.is_null() {
            return Err(CResultStatus::Err);
        }
        let vec_slice: &mut [*mut T] = std::slice::from_raw_parts_mut(vec_vec, outer_len);

        // Convert each inner pointer to a slice of T
        let mut result: Vec<&mut T> = Vec::with_capacity(outer_len);
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
    if outer_len == 0 {
        return Ok(Vec::new());
    }

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
            result.push(create_slice_view_with_null(*ptr, inner_len)?);
        }

        Ok(result)
    }
}

pub(crate) fn write_to_slice_with_null<T: Copy>(
    ptr: *mut T,
    data: &[T],
) -> Result<(), CResultStatus> {
    let ptr = deref_mut(ptr)?;
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, data.len()) };
    slice.copy_from_slice(data);
    Ok(())
}
// TODO: the data parameter might be too complicated, investigate simplifying it
pub(crate) fn write_to_slice_slice_with_null<T: Copy, const N: usize>(
    ptr: *mut *mut T,
    data: [impl AsRef<[T]>; N],
) -> Result<(), CResultStatus> {
    let out_cells = dereference_to_vec_of_slices(ptr, N)?;

    for (out_cell, result) in out_cells.into_iter().zip(data) {
        write_to_slice_with_null(out_cell, result.as_ref())?;
    }

    Ok(())
}

pub(crate) fn create_slice_view_with_null<'a, T>(
    ptr: *const T,
    len: usize,
) -> Result<&'a [T], CResultStatus> {
    if len == 0 {
        Ok(&[])
    } else {
        unsafe { Ok(std::slice::from_raw_parts(ptr, len)) }
    }
}
