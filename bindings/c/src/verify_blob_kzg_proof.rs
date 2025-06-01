use rust_eth_kzg::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::{
    pointer_utils::{create_array_ref, deref_const, deref_mut},
    verification_result_to_bool_cresult, CResult, DASContext,
};

pub(crate) fn _verify_blob_kzg_proof(
    ctx: *const DASContext,
    blob: *const u8,
    commitment: *const u8,
    proof: *const u8,
    verified: *mut bool,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
    let blob = create_array_ref::<BYTES_PER_BLOB, _>(blob);
    let commitment = create_array_ref::<BYTES_PER_COMMITMENT, _>(commitment);
    let proof = create_array_ref::<BYTES_PER_COMMITMENT, _>(proof);
    let verified = deref_mut(verified);

    // Computation
    //
    let verification_result = ctx.verify_blob_kzg_proof(blob, commitment, proof);

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}