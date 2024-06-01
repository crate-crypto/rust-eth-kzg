extern crate eip7594;

use eip7594::constants::BYTES_PER_BLOB;
pub use eip7594::constants::{
    BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB,
};
use eip7594::prover::ProverContext as eip7594_ProverContext;
use eip7594::verifier::VerifierContext as eip7594_VerifierContext;

/// The total number of bytes needed to represent all of the proofs
/// we generate for a blob.
///
/// Note: We have a test to ensure that this stays in sync with the
/// constants in the eip7594 crate.
/// Unfortunately, cbindgen doesn't allow us to use those constants directly.
pub const NUM_BYTES_PROOFS: u64 = 6144;
/// The number of bytes needed to represent all of the cells
/// we generate for a blob.
///
/// Note: We have a test to ensure that this stays in sync with the
/// constants in the eip7594 crate.
/// Unfortunately, cbindgen doesn't allow us to use those constants directly.
pub const NUM_BYTES_CELLS: u64 = 262144;

/*

A note on safety:

- It is the callers responsibility to ensure that the pointers that get passed in point to the minimum number of bytes required, to dereference them safely.
    - If the pointers, point to region of memory that is less than the minimum number of bytes required, then this method will read from random memory.
    - If the pointers point to a region of memory that is more than the minimum number of bytes required, then this method will essentially truncate the memory region.

*/

// We re-define the structs so that they can be generated in the c-code as
// opaque structs.
// TODO: try type aliasing
pub struct ProverContext(eip7594_ProverContext);
pub struct VerifierContext(eip7594_VerifierContext);

#[no_mangle]
pub extern "C" fn prover_context_new() -> *mut ProverContext {
    let ctx = Box::new(ProverContext(eip7594_ProverContext::new()));
    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn prover_context_free(ctx: *mut ProverContext) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ctx);
    }
}

#[repr(C)]
pub enum CResult {
    Ok,
    Err,
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
#[no_mangle]
pub extern "C" fn blob_to_kzg_commitment(
    ctx: *const ProverContext,
    blob: *const u8,
    out: *mut u8,
) -> CResult {
    if ctx.is_null() || blob.is_null() || out.is_null() {
        // TODO: We have ommited the error handling for null pointers at the moment.
        // TODO: Likely will panic in this case.
        return CResult::Err;
    }

    let (blob, ctx) = unsafe {
        // Note: If `blob` points to a slice that is more than the number of bytes for a blob
        // This method will not panic and will instead truncate the memory region.
        let blob_slice = std::slice::from_raw_parts(blob, BYTES_PER_BLOB);
        let ctx_ref = &*ctx;

        (blob_slice, ctx_ref)
    };
    let commitment = match ctx.0.blob_to_kzg_commitment(blob) {
        Ok(commitment) => commitment,
        Err(_) => return CResult::Err,
    };

    unsafe {
        let commitment_data_slice = std::slice::from_raw_parts_mut(out, BYTES_PER_COMMITMENT);
        commitment_data_slice.copy_from_slice(&commitment);
    }

    CResult::Ok
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `NUM_BYTES_CELLS` bytes.
#[no_mangle]
pub extern "C" fn compute_cells(
    ctx: *const ProverContext,
    blob: *const u8,
    out_cells: *mut u8,
) -> CResult {
    // Check if pointers are null
    if ctx.is_null() || blob.is_null() || out_cells.is_null() {
        return CResult::Err;
    }

    let (blob, ctx) = unsafe {
        let blob_slice = std::slice::from_raw_parts(blob, BYTES_PER_BLOB);
        let ctx_ref = &*ctx;

        (blob_slice, ctx_ref)
    };

    let cells = match ctx.0.compute_cells(blob) {
        Ok(cells) => cells,
        Err(_) => return CResult::Err,
    };

    let cells_flattened: Vec<_> = cells
        .iter()
        .flat_map(|cell| cell.into_iter())
        .copied()
        .collect();

    // Check that these are the correct sizes because callers will use these
    // methods to allocate the output arrays.
    assert_eq!(
        cells_flattened.len() as u64,
        NUM_BYTES_CELLS,
        "This is a library bug. cells_flattened.len() != num_bytes_cells(), {} != {}",
        cells_flattened.len(),
        NUM_BYTES_CELLS
    );

    unsafe {
        let cells_data_slice = std::slice::from_raw_parts_mut(out_cells, cells_flattened.len());
        cells_data_slice.copy_from_slice(&cells_flattened);
    }

    CResult::Ok
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `NUM_BYTES_CELLS` bytes.
/// - The caller must ensure that `out_proofs` points to a region of memory that is at least `NUM_BYTES_PROOFS` bytes.
#[no_mangle]
pub extern "C" fn compute_cells_and_kzg_proofs(
    ctx: *const ProverContext,
    blob: *const u8,
    out_cells: *mut u8,
    out_proofs: *mut u8,
) -> CResult {
    // Check if pointers are null
    if ctx.is_null() || blob.is_null() || out_cells.is_null() || out_proofs.is_null() {
        return CResult::Err;
    }

    let (blob, ctx) = unsafe {
        let blob_slice = std::slice::from_raw_parts(blob, BYTES_PER_BLOB);
        let ctx_ref = &*ctx;

        (blob_slice, ctx_ref)
    };

    let (cells, proofs) = match ctx.0.compute_cells_and_kzg_proofs(blob) {
        Ok(cells_and_proofs) => cells_and_proofs,
        Err(_) => return CResult::Err,
    };

    // TODO: This is not consistent with the node way of returning cells and proofs.
    // TODO: This may be fine, because node lives at a higher level and has richer features due to napi
    let cells_flattened: Vec<_> = cells
        .iter()
        .flat_map(|cell| cell.into_iter())
        .copied()
        .collect();

    let proofs_flattened: Vec<_> = proofs
        .iter()
        .flat_map(|proof| proof.iter())
        .copied()
        .collect();

    // Check that these are the correct sizes because callers will use these
    // methods to allocate the output arrays.
    assert_eq!(
        cells_flattened.len() as u64,
        NUM_BYTES_CELLS,
        "This is a library bug. cells_flattened.len() != num_bytes_cells(), {} != {}",
        cells_flattened.len(),
        NUM_BYTES_CELLS
    );
    assert_eq!(
        proofs_flattened.len() as u64,
        NUM_BYTES_PROOFS,
        "This is a library bug. proofs_flattened.len() != num_bytes_proofs(), {} != {}",
        proofs_flattened.len(),
        NUM_BYTES_PROOFS
    );

    unsafe {
        let cells_data_slice = std::slice::from_raw_parts_mut(out_cells, cells_flattened.len());
        cells_data_slice.copy_from_slice(&cells_flattened);

        let proofs_data_slice = std::slice::from_raw_parts_mut(out_proofs, proofs_flattened.len());
        proofs_data_slice.copy_from_slice(&proofs_flattened);
    }

    CResult::Ok
}

#[no_mangle]
pub extern "C" fn verifier_context_new() -> *mut VerifierContext {
    let ctx = Box::new(VerifierContext(eip7594_VerifierContext::new()));
    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn verifier_context_free(ctx: *mut VerifierContext) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ctx);
    }
}

#[cfg(test)]
mod tests {
    use eip7594::constants::{
        BYTES_PER_CELL, BYTES_PER_COMMITMENT, CELLS_PER_EXT_BLOB, NUM_PROOFS,
    };

    #[test]
    fn test_num_bytes_proof_constant() {
        assert_eq!(BYTES_PER_COMMITMENT * NUM_PROOFS, 6144);
    }

    #[test]
    fn test_num_bytes_cell_constant() {
        assert_eq!(BYTES_PER_CELL * CELLS_PER_EXT_BLOB, 262144);
    }
}

#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    fn prover_context_alloc_free() {
        let ctx = prover_context_new();
        prover_context_free(ctx);
    }

    #[test]
    fn prover_context_blob_to_kzg_commitment() {
        let ctx = prover_context_new();
        let blob = vec![0u8; BYTES_PER_BLOB];
        let mut out = vec![0u8; BYTES_PER_COMMITMENT];
        blob_to_kzg_commitment(ctx, blob.as_ptr(), out.as_mut_ptr());
    }

    #[test]
    fn prover_context_compute_cells_and_kzg_proofs() {
        let ctx = prover_context_new();
        let blob = vec![0u8; BYTES_PER_BLOB];
        let mut out_cells = vec![0u8; NUM_BYTES_CELLS as usize];
        let mut out_proofs = vec![0u8; NUM_BYTES_PROOFS as usize];
        compute_cells_and_kzg_proofs(
            ctx,
            blob.as_ptr(),
            out_cells.as_mut_ptr(),
            out_proofs.as_mut_ptr(),
        );
    }
}
