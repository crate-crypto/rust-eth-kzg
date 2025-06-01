use rust_eth_kzg::constants::{BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT};

use crate::{
    pointer_utils::{create_array_ref, deref_const, deref_mut},
    verification_result_to_bool_cresult, CResult, DASContext,
};

pub(crate) fn _verify_kzg_proof(
    ctx: *const DASContext,
    commitment: *const u8,
    z: *const u8,
    y: *const u8,
    proof: *const u8,
    verified: *mut bool,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
    let commitment = create_array_ref::<BYTES_PER_COMMITMENT, _>(commitment);
    let z = create_array_ref::<BYTES_PER_FIELD_ELEMENT, _>(z);
    let y = create_array_ref::<BYTES_PER_FIELD_ELEMENT, _>(y);
    let proof = create_array_ref::<BYTES_PER_COMMITMENT, _>(proof);
    let verified = deref_mut(verified);

    // Computation
    //
    let verification_result = ctx.verify_kzg_proof(commitment, *z, *y, proof);

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}
