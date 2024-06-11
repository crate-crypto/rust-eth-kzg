use crate::pointer_utils::{create_slice_view, deref_const, deref_mut};
use crate::{verification_result_to_bool_cresult, CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_CELL, BYTES_PER_COMMITMENT};

pub(crate) fn _verify_cell_kzg_proof_batch(
    ctx: *const PeerDASContext,

    row_commitments_length: u64,
    row_commitments: *const u8,

    row_indices_length: u64,
    row_indices: *const u64,

    column_indices_length: u64,
    column_indices: *const u64,

    cells_length: u64,
    cells: *const u8,

    proofs_length: u64,
    proofs: *const u8,

    verified: *mut bool,
) -> Result<(), CResult> {
    // When the arrays are empty in the caller language, the pointer might be null
    // This was witnessed in csharp.
    // For now, we will check for an empty batch size and return early, for both optimization purposes
    // and for safety.
    // TODO: we could make it so that the client needs to worry about making sure the ptr is not nil
    // TODO: This is an easy guarantee to put in languages and does not add a lot of code.
    //
    // TODO: We should also keep the null pointer checks so that we never have UB if the library is used incorrectly.
    if cells_length == 0 {
        unsafe { *verified = true };
        return Ok(());
    }

    // Pointer checks
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .verifier_ctx();
    let row_commitments = deref_const(row_commitments)
        .map_err(|_| CResult::with_error("could not dereference row_commitments"))?;
    let row_indices = deref_const(row_indices)
        .map_err(|_| CResult::with_error("could not dereference row_indices"))?;
    let column_indices = deref_const(column_indices)
        .map_err(|_| CResult::with_error("could not dereference column_indices"))?;
    let cells =
        deref_const(cells).map_err(|_| CResult::with_error("could not dereference cells"))?;
    let proofs =
        deref_const(proofs).map_err(|_| CResult::with_error("could not dereference proofs"))?;
    let verified = deref_mut(verified).map_err(|_| {
        CResult::with_error("could not dereference pointer to the output verified flag")
    })?;

    // Length checks
    //
    let num_cells = (cells_length / BYTES_PER_CELL as u64) as usize;
    let num_commitments = (row_commitments_length / BYTES_PER_COMMITMENT as u64) as usize;

    if (row_commitments_length as usize) != (BYTES_PER_COMMITMENT * num_commitments) {
        return Err(CResult::with_error(&format!(
            "Invalid row_commitments length. Expected: {}, Got: {}",
            BYTES_PER_COMMITMENT * num_commitments,
            row_commitments_length
        )));
    }

    if (cells_length as usize) != (num_cells * BYTES_PER_CELL) {
        return Err(CResult::with_error(&format!(
            "Invalid cells length. Expected: {}, Got: {}",
            num_cells * BYTES_PER_CELL,
            cells_length
        )));
    }

    if (proofs_length as usize) != (BYTES_PER_COMMITMENT * num_cells) {
        return Err(CResult::with_error(&format!(
            "Invalid proofs length. Expected: {}, Got: {}",
            BYTES_PER_COMMITMENT * num_cells,
            proofs_length
        )));
    }

    if (row_indices_length as usize) != num_cells {
        return Err(CResult::with_error(&format!(
            "Invalid row_indices length. Expected: {}, Got: {}",
            num_cells, row_indices_length
        )));
    }

    if (column_indices_length as usize) != num_cells {
        return Err(CResult::with_error(&format!(
            "Invalid column_indices length. Expected: {}, Got: {}",
            num_cells, column_indices_length
        )));
    }

    // Convert immutable references to slices
    //
    let row_commitments = create_slice_view(row_commitments, row_commitments_length as usize);
    let cells = create_slice_view(cells, cells_length as usize);
    let proofs = create_slice_view(proofs, proofs_length as usize);
    let row_indices = create_slice_view(row_indices, num_cells as usize);
    let column_indices = create_slice_view(column_indices, num_cells as usize);

    // Computation
    //
    let row_commitments: Vec<_> = row_commitments
        .chunks_exact(BYTES_PER_COMMITMENT as usize)
        .collect();
    let cells = cells.chunks_exact(BYTES_PER_CELL as usize).collect();
    let proofs = proofs.chunks_exact(BYTES_PER_COMMITMENT as usize).collect();

    let verification_result = ctx.verify_cell_kzg_proof_batch(
        row_commitments,
        // TODO: conversion to a vector should not be needed
        row_indices.to_vec(),
        column_indices.to_vec(),
        cells,
        proofs,
    );

    // Write to output
    let proof_is_valid = verification_result_to_bool_cresult(verification_result)?;
    *verified = proof_is_valid;

    Ok(())
}
