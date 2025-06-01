use rust_eth_kzg::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::{
    pointer_utils::{create_array_ref, deref_const, write_to_slice},
    CResult, DASContext,
};

pub(crate) fn _compute_blob_kzg_proof(
    ctx: *const DASContext,
    blob: *const u8,
    commitment: *const u8,
    out_proof: *mut u8,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
    let blob = create_array_ref::<BYTES_PER_BLOB, _>(blob);
    let commitment = create_array_ref::<BYTES_PER_COMMITMENT, _>(commitment);

    // Computation
    //
    let proof = ctx
        .compute_blob_kzg_proof(blob, commitment)
        .map_err(|err| CResult::with_error(&format!("{err:?}")))?;

    assert!(
        proof.len() == BYTES_PER_COMMITMENT,
        "This is a library bug. proof.len() != BYTES_PER_COMMITMENT, {} != {}",
        proof.len(),
        BYTES_PER_COMMITMENT
    );

    // Write output to slice
    //
    write_to_slice(out_proof, &proof);

    Ok(())
}
