// Helper methods for dereferencing raw pointers and writing to slices
//
pub(crate) fn deref_mut<'a, T>(ptr: *mut T) -> &'a mut T {
    unsafe { &mut *ptr }
}
pub(crate) fn deref_const<'a, T>(ptr: *const T) -> &'a T {
    unsafe { &*ptr }
}

pub(crate) fn deref_vec_of_slices<'a, T>(vec_vec: *mut *mut T, outer_len: usize) -> Vec<&'a mut T> {
    if outer_len == 0 {
        return Vec::new();
    }

    // Ensure we are working with raw pointers, thus we need to use unsafe
    unsafe {
        // Convert the outer pointer to a slice of raw pointers
        let vec_slice: &mut [*mut T] = std::slice::from_raw_parts_mut(vec_vec, outer_len);

        // Convert each inner pointer to a slice of T
        let mut result: Vec<&mut T> = Vec::with_capacity(outer_len);
        for ptr in vec_slice.into_iter() {
            result.push(deref_mut(*ptr));
        }

        result
    }
}
pub(crate) fn deref_to_vec_of_slices_const<'a>(
    vec_vec: *const *const u8,
    outer_len: usize,
    inner_len: usize,
) -> Vec<&'a [u8]> {
    if outer_len == 0 {
        return Vec::new();
    }

    // Ensure we are working with raw pointers, thus we need to use unsafe
    unsafe {
        let vec_slice: &[*const u8] = std::slice::from_raw_parts(vec_vec, outer_len);

        // Convert each inner pointer to a slice of u8
        let mut result: Vec<&[u8]> = Vec::with_capacity(outer_len);
        for ptr in vec_slice.into_iter() {
            result.push(create_slice_view(*ptr, inner_len));
        }

        result
    }
}

pub(crate) fn write_to_slice<T: Copy>(ptr: *mut T, data: &[T]) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, data.len()) };
    slice.copy_from_slice(data);
}
// TODO: the data parameter might be too complicated, investigate simplifying it
pub(crate) fn write_to_2d_slice<T: Copy, const N: usize>(
    ptr: *mut *mut T,
    data: [impl AsRef<[T]>; N],
) {
    let out_cells = deref_vec_of_slices(ptr, N);

    for (out_cell, result) in out_cells.into_iter().zip(data) {
        write_to_slice(out_cell, result.as_ref());
    }
}
pub(crate) fn create_slice_view<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}
