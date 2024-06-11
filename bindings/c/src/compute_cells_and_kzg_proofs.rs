use crate::pointer_utils::{
    create_slice_view, deref_const, dereference_to_vec_of_slices, write_to_slice,
};
use crate::{CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB};

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
