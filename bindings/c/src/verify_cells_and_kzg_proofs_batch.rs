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
    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
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
        ctx.verify_cell_kzg_proof_batch(commitments, cell_indices, cells, proofs);

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}
