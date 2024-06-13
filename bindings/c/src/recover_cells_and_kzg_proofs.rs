use crate::pointer_utils::{
    create_slice_view_with_null, deref_const, dereference_to_vec_of_slices_const,
    write_to_slice_slice_with_null,
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
    // Dereference the input pointers
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();
    let cells = dereference_to_vec_of_slices_const(cells, cells_length as usize, BYTES_PER_CELL)
        .map_err(|_| CResult::with_error("could not dereference cells"))?;
    let cell_ids = create_slice_view_with_null(cell_ids, cell_ids_length as usize)
        .map_err(|_| CResult::with_error("could not dereference cell_ids"))?;

    // Computation
    //
    let (recovered_cells, recovered_proofs) = ctx
        .recover_cells_and_proofs(cell_ids.to_vec(), cells)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // Write to output
    write_to_slice_slice_with_null::<_, CELLS_PER_EXT_BLOB>(out_cells, recovered_cells)
        .map_err(|_| CResult::with_error("could not write cells to output"))?;

    write_to_slice_slice_with_null::<_, CELLS_PER_EXT_BLOB>(out_proofs, recovered_proofs)
        .map_err(|_| CResult::with_error("could not write proofs to output"))?;

    Ok(())
}
