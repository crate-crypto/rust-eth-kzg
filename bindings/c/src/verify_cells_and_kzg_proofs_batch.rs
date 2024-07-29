use crate::pointer_utils::{create_slice_view, deref_const, deref_mut, ptr_ptr_to_vec_slice_const};
use crate::{verification_result_to_bool_cresult, CResult, DASContext};
use rust_eth_kzg::constants::{BYTES_PER_CELL, BYTES_PER_COMMITMENT};

#[allow(clippy::too_many_arguments)]
pub(crate) fn _verify_cell_kzg_proof_batch(
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
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // When the arrays are empty in the caller language, the pointer might be null
    // This was witnessed in csharp.
    // For now, we will check for an empty batch size and return early, for both optimization purposes
    // and for safety.
    // TODO: we could make it so that the client needs to worry about making sure the ptr is not nil
    // TODO: This is an easy guarantee to put in languages and does not add a lot of code.
    //
    // TODO: We should also keep the null pointer checks so that we never have UB if the library is used incorrectly.
    //
    // TODO: We should be able to remove this and have the code just return an empty slice if the length is 0.
    // TODO: then remove the length checks and have it handled by the underlying library.
    if cells_length == 0 {
        unsafe { *verified = true };
        return Ok(());
    }

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx).inner();
    let commitments = ptr_ptr_to_vec_slice_const::<BYTES_PER_COMMITMENT>(
        commitments,
        commitments_length as usize,
    );
    let cell_indices = create_slice_view(cell_indices, cell_indices_length as usize);
    let cells = ptr_ptr_to_vec_slice_const::<BYTES_PER_CELL>(cells, cells_length as usize);
    let proofs = ptr_ptr_to_vec_slice_const::<BYTES_PER_COMMITMENT>(proofs, proofs_length as usize);
    let verified = deref_mut(verified);

    // Computation
    //
    let verification_result =
        ctx.verify_cell_kzg_proof_batch(commitments, cell_indices.to_vec(), cells, proofs);

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}
