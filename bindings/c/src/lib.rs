extern crate eip7594;

mod blob_to_kzg_commitment;
use blob_to_kzg_commitment::_blob_to_kzg_commitment;

mod compute_cells_and_kzg_proofs;
use compute_cells_and_kzg_proofs::_compute_cells_and_kzg_proofs;

mod verify_cells_and_kzg_proofs;
use verify_cells_and_kzg_proofs::_verify_cell_kzg_proof;

mod verify_cells_and_kzg_proofs_batch;
use verify_cells_and_kzg_proofs_batch::_verify_cell_kzg_proof_batch;

mod recover_cells_and_kzg_proofs;
use recover_cells_and_kzg_proofs::_recover_all_cells_and_proofs;

pub(crate) mod pointer_utils;

pub use eip7594::constants::{
    BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT,
    CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB,
};
use eip7594::prover::ProverContext as eip7594_ProverContext;
use eip7594::verifier::{VerifierContext as eip7594_VerifierContext, VerifierError};

// TODO: Perhaps we remove undefined behavior or safety header since violating safety usually means ub

/*

A note on safety and API:

- It is also the callers responsibility to ensure that pointers are properly aligned. We do not check that *ctx PeerDASContext
   points to a PeerDASContext, we simply deref.

- TODO(put this above every function) It is the callers responsibility to ensure that pointers to pointers point to the same type of data.
    - We could make this our responsibility, but then we would need to pass in all sizes for every element in the 2d array.

- It is the callers responsibility to ensure that the pointers that get passed in point to the minimum number of bytes required, to dereference them safely.
    - If the pointers, point to region of memory that is less than the minimum number of bytes required, then this method will read from random memory.
    - If the pointers point to a region of memory that is more than the minimum number of bytes required, then this method will essentially truncate the memory region.

- For a particular instance, the length of the some parameters like blobs will always be the same.
  This means we do not need to pass the length in as a parameter, but we do so, so that we can check the users expectations on
  the expected length.

  The alternative is to have the code calling the FFI API to check the length of the blob before calling this method.
  However this is not ideal, because every language called via the FFI API will need to repeat the same checks.
*/

/// The context that will be used to create and verify proofs.
pub struct PeerDASContext {
    prover_ctx: eip7594_ProverContext,
    verifier_ctx: eip7594_VerifierContext,
}

impl PeerDASContext {
    pub fn new() -> Self {
        PeerDASContext {
            prover_ctx: eip7594_ProverContext::new(),
            verifier_ctx: eip7594_VerifierContext::new(),
        }
    }

    pub fn prover_ctx(&self) -> &eip7594_ProverContext {
        &self.prover_ctx
    }

    pub fn verifier_ctx(&self) -> &eip7594_VerifierContext {
        &self.verifier_ctx
    }
}

/// Create a new PeerDASContext and return a pointer to it.
///
/// # Memory faults
///
/// To avoid memory leaks, one should ensure that the pointer is freed after use
/// by calling `peerdas_context_free`.
#[no_mangle]
pub extern "C" fn peerdas_context_new() -> *mut PeerDASContext {
    let ctx = Box::new(PeerDASContext::new());
    Box::into_raw(ctx)
}

/// # Safety
///
/// - The caller must ensure that the pointer is valid. If the pointer is null, this method will return early.
/// - The caller should also avoid a double-free by setting the pointer to null after calling this method.
///
/// # Memory faults
///
/// - If this method is called twice on the same pointer, it will result in a double-free.
///
/// # Undefined behavior
///
/// - Since the `ctx` is created in Rust, we can only get undefined behavior, if the caller passes in
/// a pointer that was not created by `peerdas_context_new`.
#[no_mangle]
pub extern "C" fn peerdas_context_free(ctx: *mut PeerDASContext) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ctx);
    }
}

/// A C-style enum to indicate whether a function call was a success or not.
#[repr(C)]
pub enum CResultStatus {
    Ok,
    Err,
}

/// A C-style struct to represent the success result of a function call.
///
/// This includes the status of the call and an error message, if the status was an error.
#[repr(C)]
pub struct CResult {
    pub status: CResultStatus,
    pub error_msg: *mut std::os::raw::c_char,
}

impl CResult {
    /// Create a new CResult with an error message.
    ///
    /// # Memory leaks
    ///
    /// - Ownership of the error message is transferred to the caller.
    ///   The caller is responsible for freeing the memory allocated for the error message.
    ///   This can be done by calling `free_error_message`.
    ///
    /// # Memory faults
    ///
    /// - If this method is called twice on the same pointer, it will result in a double-free.
    pub fn with_error(error_msg: &str) -> Self {
        let error_msg = std::ffi::CString::new(error_msg).unwrap();
        CResult {
            status: CResultStatus::Err,
            error_msg: error_msg.into_raw(),
        }
    }

    /// Creates a new CResult with an Ok status indicating a function has returned successfully.
    pub fn with_ok() -> Self {
        CResult {
            status: CResultStatus::Ok,
            error_msg: std::ptr::null_mut(),
        }
    }
}

/// Free the memory allocated for the error message.
///
/// # Safety
///
/// - The caller must ensure that the pointer is valid. If the pointer is null, this method will return early.
/// - The caller should also avoid a double-free by setting the pointer to null after calling this method.
#[no_mangle]
pub extern "C" fn free_error_message(c_message: *mut std::os::raw::c_char) {
    // check if the pointer is null
    if c_message.is_null() {
        return;
    }
    // Safety: Deallocate the memory allocated for the C-style string
    unsafe {
        let _ = std::ffi::CString::from_raw(c_message);
    };
}

/// Compute a commitment from a Blob
///
/// # Safety
///
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `blob_len` bytes.
/// - The caller must ensure that `out` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
#[no_mangle]
#[must_use]
pub extern "C" fn blob_to_kzg_commitment(
    ctx: *const PeerDASContext,

    blob_length: u64,
    blob: *const u8,

    out: *mut u8,
) -> CResult {
    match _blob_to_kzg_commitment(ctx, blob_length, blob, out) {
        Ok(_) => CResult::with_ok(),
        Err(err) => return err,
    }
}

/// Computes the cells and KZG proofs for a given blob.
///
/// Safety:
///
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `blob` points to a region of memory that is at least `blob_len` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `NUM_BYTES_CELLS` bytes.
/// - The caller must ensure that `out_proofs` points to a region of memory that is at least `NUM_BYTES_PROOFS` bytes.
#[no_mangle]
#[must_use]
pub extern "C" fn compute_cells_and_kzg_proofs(
    ctx: *const PeerDASContext,

    blob_length: u64,
    blob: *const u8,

    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> CResult {
    match _compute_cells_and_kzg_proofs(ctx, blob_length, blob, out_cells, out_proofs) {
        Ok(_) => return CResult::with_ok(),
        Err(err) => return err,
    }
}

/// Safety:
///
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `cell` points to a region of memory that is at least `cell_length` bytes.
/// - The caller must ensure that `commitment` points to a region of memory that is at least `commitment_length` bytes.
/// - The caller must ensure that `proof` points to a region of memory that is at least `proof_length` bytes.
/// - The caller must ensure that `verified` points to a region of memory that is at least 1 byte.
//
// TODO: Can we create a new structure that allows us to hold a pointer+length? example struct Slice {ptr : *const u8, len: u64}
#[no_mangle]
#[must_use]
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
        Ok(_) => return CResult::with_ok(),
        Err(err) => return err,
    }
}

// The underlying cryptography library, uses a Result enum to indicate a proof failed verification.

// Because from the callers perspective, as long as the verification procedure is invalid, it doesn't matter why it is invalid.
// We need to unwrap it here because the FFI API is not rich enough to distinguish this case.
fn verification_result_to_bool_cresult(
    verification_result: Result<(), VerifierError>,
) -> Result<bool, CResult> {
    match verification_result {
        Ok(_) => Ok(true),
        Err(VerifierError::InvalidProof) => Ok(false),
        Err(err) => Err(CResult::with_error(&format!("{:?}", err))),
    }
}

/// Verifies a batch of cells and their KZG proofs.
///
/// # Safety
///
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `row_commitments` points to a region of memory that is at least `row_commitments_length` commitments
///   and that each commitment is at least `BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `row_indices` points to a region of memory that is at least `num_cells` elements.
/// - The caller must ensure that `column_indices` points to a region of memory that is at least `num_cells` elements.
/// - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` proof and
///   that each cell is at least `BYTES_PER_CELL` bytes
/// - The caller must ensure that `proofs` points to a region of memory that is at least `proofs_length` proofs
/// and that each proof is at least `BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `verified` points to a region of memory that is at least 1 byte.
#[no_mangle]
#[must_use]
pub extern "C" fn verify_cell_kzg_proof_batch(
    ctx: *const PeerDASContext,

    row_commitments_length: u64,
    row_commitments: *const *const u8,

    row_indices_length: u64,
    row_indices: *const u64,

    column_indices_length: u64,
    column_indices: *const u64,

    cells_length: u64,
    cells: *const *const u8,

    proofs_length: u64,
    proofs: *const *const u8,

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
        Ok(_) => return CResult::with_ok(),
        Err(err) => return err,
    }
}

/// Recovers all cells and their KZG proofs from the given cell ids and cells
///
/// # Safety
/// - The caller must ensure that the pointers are valid. If pointers are null, this method will return an error.
/// - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` cells
/// and that each cell is at least `BYTES_PER_CELL` bytes.
/// - The caller must ensure that `cell_ids` points to a region of memory that is at least `cell_ids_length` cell ids.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` cells
/// and that each cell is at least `BYTES_PER_CELL` bytes.
/// - The caller must ensure that `out_proofs` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` proofs
///   and that each proof is at least `BYTES_PER_COMMITMENT` bytes.
#[no_mangle]
#[must_use]
pub extern "C" fn recover_cells_and_proofs(
    ctx: *const PeerDASContext,

    cells_length: u64,
    cells: *const *const u8,

    cell_ids_length: u64,
    cell_ids: *const u64,

    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> CResult {
    match _recover_all_cells_and_proofs(
        ctx,
        cells_length,
        cells,
        cell_ids_length,
        cell_ids,
        out_cells,
        out_proofs,
    ) {
        Ok(_) => CResult::with_ok(),
        Err(err) => err,
    }
}

// Expose the constants to the C API so that languages that have to define them
// manually can use them in tests.
#[no_mangle]
pub extern "C" fn constant_bytes_per_cell() -> u64 {
    BYTES_PER_CELL as u64
}
#[no_mangle]
pub extern "C" fn constant_bytes_per_proof() -> u64 {
    BYTES_PER_COMMITMENT as u64
}
#[no_mangle]
pub extern "C" fn constant_cells_per_ext_blob() -> u64 {
    CELLS_PER_EXT_BLOB as u64
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
}
