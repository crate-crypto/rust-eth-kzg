use crate::pointer_utils::{create_slice_view, deref_const, deref_mut};
use crate::{verification_result_to_bool_cresult, CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_CELL, BYTES_PER_COMMITMENT};

// TODO: remove this method and call verify_cell_kzg_proof_batch directly
// TODO: Add an if statement in that method to call verify_cell_kzg_proof if the batch size is 1

pub(crate) fn _verify_cell_kzg_proof(
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
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .verifier_ctx();
    let cell = deref_const(cell).map_err(|_| CResult::with_error("could not dereference cell"))?;
    let commitment = deref_const(commitment)
        .map_err(|_| CResult::with_error("could not dereference commitment"))?;
    let proof =
        deref_const(proof).map_err(|_| CResult::with_error("could not dereference proof"))?;
    let verified = deref_mut(verified).map_err(|_| {
        CResult::with_error("could not dereference pointer to the output verified flag")
    })?;

    // Length checks
    //
    if cell_length != BYTES_PER_CELL as u64
        || commitment_length != BYTES_PER_COMMITMENT as u64
        || proof_length != BYTES_PER_COMMITMENT as u64
    {
        return Err(CResult::with_error(&format!(
            "Invalid length. Expected: cell: {}, commitment: {}, proof: {}, Got: cell: {}, commitment: {}, proof: {}",
            BYTES_PER_CELL, BYTES_PER_COMMITMENT, BYTES_PER_COMMITMENT, cell_length, commitment_length, proof_length
        )));
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
