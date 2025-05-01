mod blob_to_kzg_commitment;
use blob_to_kzg_commitment::_blob_to_kzg_commitment;

mod compute_cells_and_kzg_proofs;
use compute_cells_and_kzg_proofs::_compute_cells_and_kzg_proofs;

mod verify_cells_and_kzg_proofs_batch;
use rust_eth_kzg::constants::RECOMMENDED_PRECOMP_WIDTH;
use rust_eth_kzg::ThreadCount;
use verify_cells_and_kzg_proofs_batch::_verify_cell_kzg_proof_batch;

mod recover_cells_and_kzg_proofs;
use recover_cells_and_kzg_proofs::_recover_cells_and_proofs;

pub(crate) mod pointer_utils;

pub use rust_eth_kzg::constants::{
    BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT,
    CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB,
};
pub use rust_eth_kzg::Error;
use std::ops::Deref;

/*
 * Note: All methods in this file have been prefixed with `eth_kzg`.
 * This is so that when they are imported into languages such as nim,
 * they will have a separate namespace to other c libraries.
 *
 * ie Nim will take two c libraries and put their methods in the same
 * namespace.
 */

// This is a wrapper around the DASContext from the eip7594 library.
// We need to wrap it as some bindgen tools cannot pick up items
// not defined in this file.
#[derive(Default)]
pub struct DASContext {
    inner: rust_eth_kzg::DASContext,
}

impl DASContext {
    pub fn inner(&self) -> &rust_eth_kzg::DASContext {
        &self.inner
    }
}

impl Deref for DASContext {
    type Target = rust_eth_kzg::DASContext;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Create a new DASContext and return a pointer to it.
///
/// `num_threads`: set to `0`` to indicate that the library should pick a sensible default.
///
/// # Memory faults
///
/// To avoid memory leaks, one should ensure that the pointer is freed after use
/// by calling `eth_kzg_das_context_free`.
#[no_mangle]
pub extern "C" fn eth_kzg_das_context_new(use_precomp: bool, num_threads: u32) -> *mut DASContext {
    let use_precomp = if use_precomp {
        rust_eth_kzg::UsePrecomp::Yes {
            width: RECOMMENDED_PRECOMP_WIDTH,
        }
    } else {
        rust_eth_kzg::UsePrecomp::No
    };

    let ctx = Box::new(DASContext {
        inner: rust_eth_kzg::DASContext::with_threads(
            &rust_eth_kzg::TrustedSetup::default(),
            ThreadCount::Multi(num_threads as usize),
            use_precomp,
        ),
    });
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
///   a pointer that was not created by `eth_kzg_das_context_new`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn eth_kzg_das_context_free(ctx: *mut DASContext) {
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
    ///   This can be done by calling `eth_kzg_free_error_message`.
    ///
    /// # Memory faults
    ///
    /// - If this method is called twice on the same pointer, it will result in a double-free.
    pub fn with_error(error_msg: &str) -> Self {
        let error_msg =
            std::ffi::CString::new(error_msg).expect("Unable to convert error to CString");
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
pub unsafe extern "C" fn eth_kzg_free_error_message(c_message: *mut std::os::raw::c_char) {
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
/// - The caller must ensure that the pointers are valid.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
///
/// # Undefined behavior
///
/// - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
///   If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
#[no_mangle]
#[must_use]
pub extern "C" fn eth_kzg_blob_to_kzg_commitment(
    ctx: *const DASContext,

    blob: *const u8,

    out: *mut u8,
) -> CResult {
    match _blob_to_kzg_commitment(ctx, blob, out) {
        Ok(_) => CResult::with_ok(),
        Err(err) => err,
    }
}

/// Computes the cells and KZG proofs for a given blob.
///
/// # Safety
///
/// - The caller must ensure that the pointers are valid. If pointers are null.
/// - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` elements
///   and that each element is at least `BYTES_PER_CELL` bytes.
/// - The caller must ensure that `out_proofs` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` elements
///   and that each element is at least `BYTES_PER_COMMITMENT` bytes.
///
/// # Undefined behavior
///
/// - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
///   If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
#[no_mangle]
#[must_use]
pub extern "C" fn eth_kzg_compute_cells_and_kzg_proofs(
    ctx: *const DASContext,

    blob: *const u8,

    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> CResult {
    match _compute_cells_and_kzg_proofs(ctx, blob, out_cells, out_proofs) {
        Ok(_) => CResult::with_ok(),
        Err(err) => err,
    }
}

// The underlying cryptography library, uses a Result enum to indicate a proof failed verification.
//
// From the callers perspective, as long as the verification procedure is invalid, it doesn't matter why it is invalid.
// We need to unwrap it here because the FFI API is not rich enough to distinguish invalid proof vs invalid input.
fn verification_result_to_bool_cresult(
    verification_result: Result<(), Error>,
) -> Result<bool, CResult> {
    match verification_result {
        Ok(_) => Ok(true),
        Err(x) if x.invalid_proof() => Ok(false),
        Err(err) => Err(CResult::with_error(&format!("{:?}", err))),
    }
}

/// Verifies a batch of cells and their KZG proofs.
///
/// # Safety
///
/// - If the length parameter for a pointer is set to zero, then this implementation will not check if its pointer is
///   null. This is because the caller might have passed in a null pointer, if the length is zero. Instead an empty slice
///   will be created.
///
/// - The caller must ensure that the pointers are valid.
/// - The caller must ensure that `commitments` points to a region of memory that is at least `commitments_length` commitments
///   and that each commitment is at least `BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `row_indices` points to a region of memory that is at least `num_cells` elements
///   and that each element is 8 bytes.
/// - The caller must ensure that `cell_indices` points to a region of memory that is at least `num_cells` elements
///   and that each element is 8 bytes.
/// - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` proof and
///   that each cell is at least `BYTES_PER_CELL` bytes
/// - The caller must ensure that `proofs` points to a region of memory that is at least `proofs_length` proofs
///   and that each proof is at least `BYTES_PER_COMMITMENT` bytes.
/// - The caller must ensure that `verified` points to a region of memory that is at least 1 byte.
///
/// # Undefined behavior
///
/// - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
///   If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
#[no_mangle]
#[must_use]
pub extern "C" fn eth_kzg_verify_cell_kzg_proof_batch(
    ctx: *const DASContext,

    commitments_length: u64,
    commitments: *const *const u8,

    cell_indices_length: u64,
    cell_indices: *const u64,

    cells_length: u64,
    cells: *const *const u8,

    proofs_length: u64,
    proofs: *const *const u8,

    verified: *mut bool,
) -> CResult {
    match _verify_cell_kzg_proof_batch(
        ctx,
        commitments_length,
        commitments,
        cell_indices_length,
        cell_indices,
        cells_length,
        cells,
        proofs_length,
        proofs,
        verified,
    ) {
        Ok(_) => CResult::with_ok(),
        Err(err) => err,
    }
}

/// Recovers all cells and their KZG proofs from the given cell indices and cells
///
/// # Safety
///
///  - If the length parameter for a pointer is set to zero, then this implementation will not check if its pointer is
///    null. This is because the caller might have passed in a null pointer, if the length is zero. Instead an empty slice will be created.
///
/// - The caller must ensure that the pointers are valid.
/// - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` cells
///   and that each cell is at least `BYTES_PER_CELL` bytes.
/// - The caller must ensure that `cell_indices` points to a region of memory that is at least `cell_indices_length` cell indices
///   and that each cell id is 8 bytes.
/// - The caller must ensure that `out_cells` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` cells
///   and that each cell is at least `BYTES_PER_CELL` bytes.
/// - The caller must ensure that `out_proofs` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` proofs
///   and that each proof is at least `BYTES_PER_COMMITMENT` bytes.
///
/// # Undefined behavior
///
/// - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
///   If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
#[no_mangle]
#[must_use]
pub extern "C" fn eth_kzg_recover_cells_and_proofs(
    ctx: *const DASContext,

    cells_length: u64,
    cells: *const *const u8,

    cell_indices_length: u64,
    cell_indices: *const u64,

    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> CResult {
    match _recover_cells_and_proofs(
        ctx,
        cells_length,
        cells,
        cell_indices_length,
        cell_indices,
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
pub extern "C" fn eth_kzg_constant_bytes_per_cell() -> u64 {
    BYTES_PER_CELL as u64
}
#[no_mangle]
pub extern "C" fn eth_kzg_constant_bytes_per_proof() -> u64 {
    BYTES_PER_COMMITMENT as u64
}
#[no_mangle]
pub extern "C" fn eth_kzg_constant_cells_per_ext_blob() -> u64 {
    CELLS_PER_EXT_BLOB as u64
}
