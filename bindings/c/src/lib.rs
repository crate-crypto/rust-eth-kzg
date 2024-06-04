extern crate eip7594;

use eip7594::constants::{BYTES_PER_BLOB, BYTES_PER_CELL};
pub use eip7594::constants::{
    BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB,
};
use eip7594::prover::ProverContext as eip7594_ProverContext;
use eip7594::verifier::{VerifierContext as eip7594_VerifierContext, VerifierError};

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

A note on safety and API:

- It is also the callers responsibility to ensure that pointers are properly aligned. We do not check that *ctx PeerDASContext
   points to a PeerDASContext, we simply deref.

- It is the callers responsibility to ensure that the pointers that get passed in point to the minimum number of bytes required, to dereference them safely.
    - If the pointers, point to region of memory that is less than the minimum number of bytes required, then this method will read from random memory.
    - If the pointers point to a region of memory that is more than the minimum number of bytes required, then this method will essentially truncate the memory region.

- For a particular instance, the length of the some parameters like blobs will always be the same.
  This means we do not need to pass the length in as a parameter, but we do so, so that we can check the users expectations on
  the expected length.

  The alternative is to have the code calling the FFI API to check the length of the blob before calling this method.
  However this is not ideal, because every language called via the FFI API will need to repeat the same checks.
*/

pub struct PeerDASContext {
    prover_ctx: Option<eip7594_ProverContext>,
    verifier_ctx: Option<eip7594_VerifierContext>,
}

impl PeerDASContext {
    pub fn new() -> Self {
        PeerDASContext::with_setting(CContextSetting::Both)
    }

    pub fn with_setting(setting: CContextSetting) -> Self {
        match setting {
            CContextSetting::ProvingOnly => PeerDASContext {
                prover_ctx: Some(eip7594_ProverContext::new()),
                verifier_ctx: None,
            },
            CContextSetting::VerifyOnly => PeerDASContext {
                prover_ctx: None,
                verifier_ctx: Some(eip7594_VerifierContext::new()),
            },
            CContextSetting::Both => PeerDASContext {
                prover_ctx: Some(eip7594_ProverContext::new()),
                verifier_ctx: Some(eip7594_VerifierContext::new()),
            },
        }
    }

    pub fn prover_ctx(&self) -> Option<&eip7594_ProverContext> {
        self.prover_ctx.as_ref()
    }

    pub fn verifier_ctx(&self) -> Option<&eip7594_VerifierContext> {
        self.verifier_ctx.as_ref()
    }
}

#[no_mangle]
pub extern "C" fn peerdas_context_new() -> *mut PeerDASContext {
    let ctx = Box::new(PeerDASContext::new());
    Box::into_raw(ctx)
}
#[no_mangle]
pub extern "C" fn peerdas_context_new_with_setting(
    setting: CContextSetting,
) -> *mut PeerDASContext {
    let ctx = Box::new(PeerDASContext::with_setting(setting));
    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn peerdas_context_free(ctx: *mut PeerDASContext) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ctx);
    }
}

/// The settings object for the Context.
/// This is used to indicate if the context is for proving only, verifying only or both.
#[repr(C)]
pub enum CContextSetting {
    ProvingOnly,
    VerifyOnly,
    Both,
}

/// The Result of each FFI function call.
/// This is used to indicate if the function call was successful or not.
#[repr(C)]
pub enum CResult {
    Ok,
    Err,
}

// Helper methods for dereferencing raw pointers and writing to slices
//
fn deref_mut<'a, T>(ptr: *mut T) -> Result<&'a mut T, CResult> {
    unsafe { ptr.as_mut().map_or(Err(CResult::Err), |p| Ok(p)) }
}
fn deref_const<'a, T>(ptr: *const T) -> Result<&'a T, CResult> {
    unsafe { ptr.as_ref().map_or(Err(CResult::Err), |p| Ok(p)) }
}
// TODO: We could return the number of bytes written to the C function so they can check if the length is correct.
fn write_to_slice<T: Copy>(ptr: &mut T, data: &[T]) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, data.len()) };
    slice.copy_from_slice(data);
}
// Note: If `ptr` points to a slice that is more than `len`
// This method will not panic and will instead truncate the memory region.
fn create_slice_view<'a, T>(ptr: &T, len: usize) -> &'a [T] {
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
#[no_mangle]
pub extern "C" fn blob_to_kzg_commitment(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out: *mut u8,
) -> CResult {
    match _blob_to_kzg_commitment(ctx, blob_length, blob, out) {
        Ok(_) => CResult::Ok,
        Err(err) => return err,
    }
}
fn _blob_to_kzg_commitment(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out: *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)?;
    let ctx = ctx.prover_ctx().ok_or(CResult::Err)?;
    let blob = deref_const(blob)?;
    let out = deref_mut(out)?;

    // Length checks
    //
    if blob_length != BYTES_PER_BLOB as u64 {
        return Err(CResult::Err);
    }

    // Convert immutable references to slices
    //
    let blob = create_slice_view(blob, blob_length as usize);

    // Computation
    //
    let commitment = ctx.blob_to_kzg_commitment(blob).map_err(|_| CResult::Err)?;
    assert!(
        commitment.len() == BYTES_PER_COMMITMENT as usize,
        "This is a library bug. commitment.len() != BYTES_PER_COMMITMENT, {} != {}",
        commitment.len(),
        BYTES_PER_COMMITMENT
    );

    // Write output to slice
    //
    write_to_slice(out, &commitment);

    Ok(())
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `NUM_BYTES_CELLS` bytes.
#[no_mangle]
pub extern "C" fn compute_cells(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out_cells: *mut u8,
) -> CResult {
    match _compute_cells(ctx, blob_length, blob, out_cells) {
        Ok(_) => return CResult::Ok,
        Err(err) => return err,
    }
}

fn _compute_cells(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out_cells: *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)?;
    let ctx = ctx.prover_ctx().ok_or(CResult::Err)?;
    let blob = deref_const(blob)?;
    let out_cells = deref_mut(out_cells)?;

    // Length checks
    //
    if blob_length != BYTES_PER_BLOB as u64 {
        return Err(CResult::Err);
    }

    // Convert immutable references to slices
    //
    let blob = create_slice_view(blob, blob_length as usize);

    // Computation
    //
    let cells = ctx.compute_cells(blob).map_err(|_| CResult::Err)?;

    let cells_flattened: Vec<_> = cells
        .iter()
        .flat_map(|cell| cell.into_iter())
        .copied()
        .collect();

    assert_eq!(
        cells_flattened.len() as u64,
        NUM_BYTES_CELLS,
        "This is a library bug. cells_flattened.len() != num_bytes_cells(), {} != {}",
        cells_flattened.len(),
        NUM_BYTES_CELLS
    );

    // Write to output
    write_to_slice(out_cells, &cells_flattened);

    Ok(())
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `NUM_BYTES_CELLS` bytes.
/// - The caller must ensure that `out_proofs` points to a region of memory that is at least `NUM_BYTES_PROOFS` bytes.
#[no_mangle]
pub extern "C" fn compute_cells_and_kzg_proofs(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out_cells: *mut u8,
    out_proofs: *mut u8,
) -> CResult {
    match _compute_cells_and_kzg_proofs(ctx, blob_length, blob, out_cells, out_proofs) {
        Ok(_) => return CResult::Ok,
        Err(err) => return err,
    }
}
fn _compute_cells_and_kzg_proofs(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out_cells: *mut u8,
    out_proofs: *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)?;
    let ctx = ctx.prover_ctx().ok_or(CResult::Err)?;
    let blob = deref_const(blob)?;
    let out_cells = deref_mut(out_cells)?;
    let out_proofs = deref_mut(out_proofs)?;

    // Length checks
    //
    if blob_length != BYTES_PER_BLOB as u64 {
        return Err(CResult::Err);
    }

    // Convert immutable references to slices
    //
    let blob = create_slice_view(blob, blob_length as usize);

    // Computation
    //
    let (cells, proofs) = ctx
        .compute_cells_and_kzg_proofs(blob)
        .map_err(|_| CResult::Err)?;

    // TODO: This is not consistent with the node way of returning cells and proofs.
    // TODO: This may be fine, because node lives at a higher level and has richer features due to napi
    let cells_flattened: Vec<_> = cells
        .iter()
        .flat_map(|cell| cell.into_iter())
        .copied()
        .collect();
    assert_eq!(
        cells_flattened.len() as u64,
        NUM_BYTES_CELLS,
        "This is a library bug. cells_flattened.len() != num_bytes_cells(), {} != {}",
        cells_flattened.len(),
        NUM_BYTES_CELLS
    );

    let proofs_flattened: Vec<_> = proofs
        .iter()
        .flat_map(|proof| proof.iter())
        .copied()
        .collect();
    assert_eq!(
        proofs_flattened.len() as u64,
        NUM_BYTES_PROOFS,
        "This is a library bug. proofs_flattened.len() != num_bytes_proofs(), {} != {}",
        proofs_flattened.len(),
        NUM_BYTES_PROOFS
    );

    // Write to output
    write_to_slice(out_cells, &cells_flattened);
    write_to_slice(out_proofs, &proofs_flattened);

    Ok(())
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `cell` points to a region of memory that is at least `BYTES_PER_CELL` bytes.
/// - The caller must ensure that `commitment` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `proof` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `verified` points to a region of memory that is at least 1 byte.
//
// TODO: Can we create a new structure that allows us to hold a pointer+length? example struct Slice {ptr : *const u8, len: u64}
#[no_mangle]
pub extern "C" fn verify_cell_kzg_proof(
    ctx: *const PeerDASContext,
    cell_length: u64,
    cell: *const u8,
    commitment_length: u64,
    commitment: *const u8,
    cell_id: u64,
    proof_length: u64,
    proof: *const u8,
    verified: *mut bool,
) -> CResult {
    match _verify_cell_kzg_proof(
        ctx,
        cell_length,
        cell,
        commitment_length,
        commitment,
        cell_id,
        proof_length,
        proof,
        verified,
    ) {
        Ok(_) => return CResult::Ok,
        Err(err) => return err,
    }
}

fn _verify_cell_kzg_proof(
    ctx: *const PeerDASContext,
    cell_length: u64,
    cell: *const u8,
    commitment_length: u64,
    commitment: *const u8,
    cell_id: u64,
    proof_length: u64,
    proof: *const u8,
    verified: *mut bool,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)?;
    let ctx = ctx.verifier_ctx().ok_or(CResult::Err)?;
    let cell = deref_const(cell)?;
    let commitment = deref_const(commitment)?;
    let proof = deref_const(proof)?;
    let verified = deref_mut(verified)?;

    // Length checks
    //
    if cell_length != BYTES_PER_CELL as u64
        || commitment_length != BYTES_PER_COMMITMENT as u64
        || proof_length != BYTES_PER_COMMITMENT as u64
    {
        return Err(CResult::Err);
    }

    // Convert immutable references to slices
    //
    let cell = create_slice_view(cell, cell_length as usize);
    let commitment = create_slice_view(commitment, commitment_length as usize);
    let proof = create_slice_view(proof, proof_length as usize);

    // Computation
    //
    let verification_result = ctx.verify_cell_kzg_proof(commitment, cell_id, cell, proof);

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}

// Note: The underlying cryptography library, uses a Result enum to indicate a proof failed.
// Because from the callers perspective, as long as the verification procedure is invalid, it doesn't matter why it is invalid.
// We need to unwrap it here because the FFI API is not rich enough to distinguish this.
fn verification_result_to_bool_cresult(
    verification_result: Result<(), VerifierError>,
) -> Result<bool, CResult> {
    match verification_result {
        Ok(_) => Ok(true),
        Err(VerifierError::InvalidProof) => Ok(false),
        Err(_) => Err(CResult::Err),
    }
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `row_commitments` points to a region of memory that is at least `row_commitments_length` bytes.
/// - The caller must ensure that `row_indices` points to a region of memory that is at least `num_cells` bytes.
/// - The caller must ensure that `column_indices` points to a region of memory that is at least `num_cells` bytes.
/// - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` bytes.
/// - The caller must ensure that `proofs` points to a region of memory that is at least `num_cells * BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `verified` points to a region of memory that is at least 1 byte.
///
/// Note: cells, proofs and row_commitments are expected to be contiguous in memory.
/// ie they have been concatenated together
#[no_mangle]
pub extern "C" fn verify_cell_kzg_proof_batch(
    ctx: *const PeerDASContext,
    row_commitments_length: u64,
    row_commitments: *const u8,
    row_indices_length: u64,
    row_indices: *const u64,
    column_indices_length: u64,
    column_indices: *const u64,
    cells_length: u64,
    cells: *const u8,
    proofs_length: u64,
    proofs: *const u8,
    verified: *mut bool,
) -> CResult {
    match _verify_cell_kzg_proof_batch(
        ctx,
        row_commitments_length,
        row_commitments,
        row_indices_length,
        row_indices,
        column_indices_length,
        column_indices,
        cells_length,
        cells,
        proofs_length,
        proofs,
        verified,
    ) {
        Ok(_) => return CResult::Ok,
        Err(err) => return err,
    }
}
fn _verify_cell_kzg_proof_batch(
    ctx: *const PeerDASContext,
    row_commitments_length: u64,
    row_commitments: *const u8,
    row_indices_length: u64,
    row_indices: *const u64,
    column_indices_length: u64,
    column_indices: *const u64,
    cells_length: u64,
    cells: *const u8,
    proofs_length: u64,
    proofs: *const u8,
    verified: *mut bool,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)?;
    let ctx = ctx.verifier_ctx().ok_or(CResult::Err)?;
    let row_commitments = deref_const(row_commitments)?;
    let row_indices = deref_const(row_indices)?;
    let column_indices = deref_const(column_indices)?;
    let cells = deref_const(cells)?;
    let proofs = deref_const(proofs)?;
    let verified = deref_mut(verified)?;

    // Length checks
    //
    let num_cells = (cells_length / BYTES_PER_CELL as u64) as usize;
    let num_commitments = (row_commitments_length / BYTES_PER_COMMITMENT as u64) as usize;

    if (row_commitments_length as usize) != (BYTES_PER_COMMITMENT * num_commitments) {
        return Err(CResult::Err);
    }

    if (cells_length as usize) != (num_cells * BYTES_PER_CELL) {
        return Err(CResult::Err);
    }

    if (proofs_length as usize) != (BYTES_PER_COMMITMENT * num_cells) {
        return Err(CResult::Err);
    }

    if (row_indices_length as usize) != num_cells {
        return Err(CResult::Err);
    }

    if (column_indices_length as usize) != num_cells {
        return Err(CResult::Err);
    }

    // Convert immutable references to slices
    //
    let row_commitments = create_slice_view(row_commitments, row_commitments_length as usize);
    let cells = create_slice_view(cells, cells_length as usize);
    let proofs = create_slice_view(proofs, proofs_length as usize);
    let row_indices = create_slice_view(row_indices, num_cells as usize);
    let column_indices = create_slice_view(column_indices, num_cells as usize);

    // Computation
    //
    let row_commitments: Vec<_> = row_commitments
        .chunks_exact(BYTES_PER_COMMITMENT as usize)
        .collect();
    let cells = cells.chunks_exact(BYTES_PER_CELL as usize).collect();
    let proofs = proofs.chunks_exact(BYTES_PER_COMMITMENT as usize).collect();

    let verification_result = ctx.verify_cell_kzg_proof_batch(
        row_commitments,
        // TODO: conversion to a vector should not be needed
        row_indices.to_vec(),
        column_indices.to_vec(),
        cells,
        proofs,
    );

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}

/// Safety:
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `cell_ids` points to a region of memory that is at least `num_cells` bytes.
/// - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `NUM_BYTES_CELLS` bytes.
#[no_mangle]
pub extern "C" fn recover_all_cells(
    ctx: *const PeerDASContext,
    cells_length: u64,
    cells: *const u8,
    cell_ids_length: u64,
    cell_ids: *const u64,
    out_cells: *mut u8,
) -> CResult {
    match _recover_all_cells(
        ctx,
        cells_length,
        cells,
        cell_ids_length,
        cell_ids,
        out_cells,
    ) {
        Ok(_) => return CResult::Ok,
        Err(err) => return err,
    }
}
fn _recover_all_cells(
    ctx: *const PeerDASContext,
    cells_length: u64,
    cells: *const u8,
    cell_ids_length: u64,
    cell_ids: *const u64,
    out_cells: *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)?;
    let ctx = ctx.verifier_ctx().ok_or(CResult::Err)?;
    let cells = deref_const(cells)?;
    let cell_ids = deref_const(cell_ids)?;
    let out_cells = deref_mut(out_cells)?;

    // Length checks
    //
    if cells_length % (BYTES_PER_CELL as u64) != 0 {
        return Err(CResult::Err);
    }
    let num_cells = cells_length as usize / BYTES_PER_CELL as usize;
    if cell_ids_length != num_cells as u64 {
        return Err(CResult::Err);
    }

    // Convert immutable references to slices
    //
    let cells = create_slice_view(cells, cells_length as usize);
    let cell_ids = create_slice_view(cell_ids, cell_ids_length as usize);

    // Computation
    //
    let cells: Vec<_> = cells.chunks_exact(BYTES_PER_CELL as usize).collect();

    let recovered_cells = ctx
        .recover_all_cells(cell_ids.to_vec(), cells)
        .map_err(|_| CResult::Err)?;

    let cells_flattened: Vec<_> = recovered_cells
        .iter()
        .flat_map(|cell| cell.into_iter())
        .copied()
        .collect();

    assert_eq!(
        cells_flattened.len() as u64,
        NUM_BYTES_CELLS,
        "This is a library bug. cells_flattened.len() != num_bytes_cells(), {} != {}",
        cells_flattened.len(),
        NUM_BYTES_CELLS
    );

    // Write to output
    write_to_slice(out_cells, &cells_flattened);

    Ok(())
}

#[cfg(test)]
mod tests {
    use eip7594::constants::{
        BYTES_PER_CELL, BYTES_PER_COMMITMENT, CELLS_PER_EXT_BLOB, NUM_PROOFS,
    };

    use crate::{NUM_BYTES_CELLS, NUM_BYTES_PROOFS};

    #[test]
    fn test_num_bytes_proof_constant() {
        assert_eq!(BYTES_PER_COMMITMENT * NUM_PROOFS, NUM_BYTES_PROOFS as usize);
    }

    #[test]
    fn test_num_bytes_cell_constant() {
        assert_eq!(
            BYTES_PER_CELL * CELLS_PER_EXT_BLOB,
            NUM_BYTES_CELLS as usize
        );
    }
}

#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    fn prover_context_alloc_free() {
        let ctx = peerdas_context_new();
        peerdas_context_free(ctx);
    }

    #[test]
    fn prover_context_blob_to_kzg_commitment() {
        let ctx = peerdas_context_new();
        let blob = vec![0u8; BYTES_PER_BLOB];
        let mut out = vec![0u8; BYTES_PER_COMMITMENT];
        blob_to_kzg_commitment(ctx, blob.len() as u64, blob.as_ptr(), out.as_mut_ptr());
    }

    #[test]
    fn prover_context_compute_cells_and_kzg_proofs() {
        let ctx = peerdas_context_new();
        let blob = vec![0u8; BYTES_PER_BLOB];
        let mut out_cells = vec![0u8; NUM_BYTES_CELLS as usize];
        let mut out_proofs = vec![0u8; NUM_BYTES_PROOFS as usize];
        compute_cells_and_kzg_proofs(
            ctx,
            blob.len() as u64,
            blob.as_ptr(),
            out_cells.as_mut_ptr(),
            out_proofs.as_mut_ptr(),
        );
    }
}
