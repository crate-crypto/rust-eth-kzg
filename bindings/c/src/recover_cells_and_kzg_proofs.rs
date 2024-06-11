use crate::pointer_utils::{
    create_slice_view, deref_const, dereference_to_vec_of_slices,
    dereference_to_vec_of_slices_const, write_to_slice,
};
use crate::{CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_CELL, CELLS_PER_EXT_BLOB};

pub(crate) fn _recover_all_cells_and_proofs(
    ctx: *const PeerDASContext,
    cells_length: u64,
    cells: *const *const u8,
    cell_ids_length: u64,
    cell_ids: *const u64,
    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> Result<(), CResult> {
    if cells_length == 0 {
        return Err(CResult::with_error("Not enough cells for recovery"));
    }

    // Pointer checks
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();
    let num_cells = cells_length;
    let cells = dereference_to_vec_of_slices_const(cells, num_cells as usize, BYTES_PER_CELL)
        .map_err(|_| CResult::with_error("could not dereference cells"))?;
    let cell_ids =
        deref_const(cell_ids).map_err(|_| CResult::with_error("could not dereference cell_ids"))?;
    let out_cells = dereference_to_vec_of_slices(out_cells, CELLS_PER_EXT_BLOB)
        .map_err(|_| CResult::with_error("could not dereference pointer to the output"))?;
    let out_proofs = dereference_to_vec_of_slices(out_proofs, CELLS_PER_EXT_BLOB)
        .map_err(|_| CResult::with_error("could not dereference pointer to the output"))?;

    // Length checks
    //
    let num_cells = cells_length as usize;
    if cell_ids_length != num_cells as u64 {
        return Err(CResult::with_error(&format!(
            "Invalid cell_ids length. Expected: {}, Got: {}",
            num_cells, cell_ids_length
        )));
    }

    // Convert immutable references to slices
    //
    let cell_ids = create_slice_view(cell_ids, cell_ids_length as usize);

    // Computation
    //
    let (recovered_cells, recovered_proofs) = ctx
        .recover_cells_and_proofs(cell_ids.to_vec(), cells)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // Write to output
    assert_eq!(out_cells.len(), recovered_cells.len());
    assert_eq!(out_proofs.len(), recovered_proofs.len());

    for (out_cell, result) in out_cells.into_iter().zip(recovered_cells) {
        write_to_slice(out_cell, &result);
    }
    for (out_proof, result) in out_proofs.into_iter().zip(recovered_proofs) {
        write_to_slice(out_proof, &result);
    }

    Ok(())
}
