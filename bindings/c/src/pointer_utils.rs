// Helper methods for dereferencing raw pointers and writing to slices

/// Dereference a raw pointer to a mutable reference
pub(crate) fn deref_mut<'a, T>(ptr: *mut T) -> &'a mut T {
    unsafe { &mut *ptr }
}

/// Dereference a raw pointer to an immutable reference
pub(crate) fn deref_const<'a, T>(ptr: *const T) -> &'a T {
    unsafe { &*ptr }
}

/// Dereference a raw pointer to a pointer to a mutable slice of mutable slices
pub(crate) fn ptr_ptr_to_slice_slice_mut<'a, T>(
    ptr_ptr: *mut *mut T,
    outer_len: usize,
) -> &'a mut [&'a mut T] {
    if outer_len == 0 {
        return &mut [];
    }
    unsafe { std::slice::from_raw_parts_mut(ptr_ptr as *mut &mut T, outer_len) }
}

/// Dereference a raw pointer to a pointer to a vector of slices
pub(crate) fn ptr_ptr_to_vec_slice_const<'a, const INNER_LEN: usize>(
    ptr_ptr: *const *const u8,
    outer_len: usize,
) -> Vec<&'a [u8; INNER_LEN]> {
    if outer_len == 0 {
        return Vec::new();
    }

    let vec_slice: &[*const u8] = unsafe { std::slice::from_raw_parts(ptr_ptr, outer_len) };

    let mut result: Vec<&[u8; INNER_LEN]> = Vec::with_capacity(outer_len);

    // Convert each inner pointer to a slice of u8
    for ptr in vec_slice.iter() {
        let slice = create_array_ref::<INNER_LEN, _>(*ptr);
        result.push(slice);
    }

    result
}

/// Write `data` to a slice starting at `ptr`
pub(crate) fn write_to_slice<T: Copy>(ptr: *mut T, data: &[T]) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, data.len()) };
    slice.copy_from_slice(data);
}

/// Write `data` to a 2D slice starting at `ptr`
// TODO: the data parameter might be too complicated, investigate simplifying it
pub(crate) fn write_to_2d_slice<T: Copy, const N: usize>(
    ptr: *mut *mut T,
    data: [impl AsRef<[T]>; N],
) {
    let out_cells = ptr_ptr_to_slice_slice_mut(ptr, N);

    for (out_cell, result) in out_cells.iter_mut().zip(data) {
        write_to_slice(*out_cell, result.as_ref());
    }
}

/// Constructs a array_ref from a pointer and a length.
///
/// If the length is 0, an empty slice is returned regardless of the pointer.
pub(crate) fn create_array_ref<'a, const LEN: usize, T>(ptr: *const T) -> &'a [T; LEN] {
    let slice = create_slice_view(ptr, LEN);
    slice.try_into().expect("item should have length {LEN}")
}
pub(crate) fn create_slice_view<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}
