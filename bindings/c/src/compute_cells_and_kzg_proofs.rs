use crate::pointer_utils::{
    create_slice_view, deref_const, deref_mut, dereference_to_vec_of_slices, write_to_slice,
};
use crate::{CResult, PeerDASContext, NUM_BYTES_CELLS, NUM_BYTES_PROOFS};
use eip7594::constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB};

pub(crate) fn _compute_cells_and_kzg_proofs(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out_cells: *mut u8,
    out_proofs: *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();
    let blob = deref_const(blob)
        .map_err(|_| CResult::with_error("could not dereference pointer to blob"))?;
    let out_cells = deref_mut(out_cells)
        .map_err(|_| CResult::with_error("could not dereference pointer to the output cells"))?;
    let out_proofs = deref_mut(out_proofs)
        .map_err(|_| CResult::with_error("could not dereference pointer to the output proofs"))?;

    // Length checks
    //
    if blob_length != BYTES_PER_BLOB as u64 {
        return Err(CResult::with_error(&format!(
            "Invalid blob length. Expected: {}, Got: {}",
            BYTES_PER_BLOB, blob_length
        )));
    }

    // Convert immutable references to slices
    //
    let blob = create_slice_view(blob, blob_length as usize);

    // Computation
    //
    let (cells, proofs) = ctx
        .compute_cells_and_kzg_proofs(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // TODO: This is not consistent with the node way of returning cells and proofs.
    // TODO: This may be fine, because node lives at a higher level and has richer features due to napi
    let cells_flattened: Vec<_> = cells
        .iter()
        .flat_map(|cell| cell.into_iter())
        .copied()
        .collect();
    assert_eq!(
        cells_flattened.len() as u64,
        NUM_BYTES_CELLS,
        "This is a library bug. cells_flattened.len() != num_bytes_cells(), {} != {}",
        cells_flattened.len(),
        NUM_BYTES_CELLS
    );

    let proofs_flattened: Vec<_> = proofs
        .iter()
        .flat_map(|proof| proof.iter())
        .copied()
        .collect();
    assert_eq!(
        proofs_flattened.len() as u64,
        NUM_BYTES_PROOFS,
        "This is a library bug. proofs_flattened.len() != num_bytes_proofs(), {} != {}",
        proofs_flattened.len(),
        NUM_BYTES_PROOFS
    );

    // Write to output
    write_to_slice(out_cells, &cells_flattened);
    write_to_slice(out_proofs, &proofs_flattened);

    Ok(())
}

pub(crate) fn _compute_cells_and_kzg_proofs_deflattened(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();
    let blob = deref_const(blob)
        .map_err(|_| CResult::with_error("could not dereference pointer to blob"))?;
    let out_cells = dereference_to_vec_of_slices(out_cells, CELLS_PER_EXT_BLOB)
        .map_err(|_| CResult::with_error("could not dereference pointer to the output"))?;
    let out_proofs = dereference_to_vec_of_slices(out_proofs, CELLS_PER_EXT_BLOB)
        .map_err(|_| CResult::with_error("could not dereference pointer to the output"))?;

    // Length checks
    //
    if blob_length != BYTES_PER_BLOB as u64 {
        return Err(CResult::with_error(&format!(
            "Invalid blob length. Expected: {}, Got: {}",
            BYTES_PER_BLOB, blob_length
        )));
    }

    // Convert immutable references to slices
    //
    let blob = create_slice_view(blob, blob_length as usize);

    // Computation
    //
    let (cells, proofs) = ctx
        .compute_cells_and_kzg_proofs(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // Write to output
    assert_eq!(out_cells.len(), cells.len());
    assert_eq!(out_proofs.len(), proofs.len());

    for (out_cell, result) in out_cells.into_iter().zip(cells) {
        write_to_slice(out_cell, &result);
    }
    for (out_proof, result) in out_proofs.into_iter().zip(proofs) {
        write_to_slice(out_proof, &result);
    }

    Ok(())
}
