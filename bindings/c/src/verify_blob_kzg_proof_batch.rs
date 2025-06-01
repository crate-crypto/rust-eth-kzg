use rust_eth_kzg::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::{
    pointer_utils::{deref_const, deref_mut, ptr_ptr_to_vec_slice_const},
    verification_result_to_bool_cresult, CResult, DASContext,
};

pub(crate) fn _verify_blob_kzg_proof_batch(
    ctx: *const DASContext,
    blobs_length: u64,
    blobs: *const *const u8,
    commitments_length: u64,
    commitments: *const *const u8,
    proofs_length: u64,
    proofs: *const *const u8,
    verified: *mut bool,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
    let blobs = ptr_ptr_to_vec_slice_const::<BYTES_PER_BLOB>(blobs, blobs_length as usize);
    let commitments = ptr_ptr_to_vec_slice_const::<BYTES_PER_COMMITMENT>(
        commitments,
        commitments_length as usize,
    );
    let proofs = ptr_ptr_to_vec_slice_const::<BYTES_PER_COMMITMENT>(proofs, proofs_length as usize);
    let verified = deref_mut(verified);

    // Computation - now all parameters use reference types consistently
    //
    let verification_result = ctx.verify_blob_kzg_proof_batch(blobs, commitments, proofs);

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}