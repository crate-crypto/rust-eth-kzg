use crate::pointer_utils::{create_slice_view_with_null, deref_const, deref_mut};
use crate::{verification_result_to_bool_cresult, CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_CELL, BYTES_PER_COMMITMENT};

// TODO: remove this method and call verify_cell_kzg_proof_batch directly
// TODO: Add an if statement in that method to call verify_cell_kzg_proof if the batch size is 1

pub(crate) fn _verify_cell_kzg_proof(
    ctx: *const PeerDASContext,

    cell: *const u8,

    commitment: *const u8,

    cell_id: u64,

    proof: *const u8,

    verified: *mut bool,
) -> Result<(), CResult> {
    // Dereference the input pointers
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .verifier_ctx();
    let cell = create_slice_view_with_null(cell, BYTES_PER_CELL)
        .map_err(|_| CResult::with_error("could not dereference cell"))?;
    let commitment = create_slice_view_with_null(commitment, BYTES_PER_COMMITMENT)
        .map_err(|_| CResult::with_error("could not dereference commitment"))?;
    let proof = create_slice_view_with_null(proof, BYTES_PER_COMMITMENT)
        .map_err(|_| CResult::with_error("could not dereference proof"))?;

    // Computation
    //
    let verification_result = ctx.verify_cell_kzg_proof(commitment, cell_id, cell, proof);

    // Write to output
    let verified = deref_mut(verified).map_err(|_| {
        CResult::with_error("could not dereference pointer to the output verified flag")
    })?;
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}
