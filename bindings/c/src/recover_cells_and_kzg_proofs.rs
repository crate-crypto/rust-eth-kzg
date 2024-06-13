use crate::pointer_utils::{
    create_slice_view, deref_const, ptr_ptr_to_vec_slice_const, write_to_2d_slice,
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
    assert!(ctx.is_null() == false, "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx).prover_ctx();
    let cells = ptr_ptr_to_vec_slice_const(cells, cells_length as usize, BYTES_PER_CELL);
    let cell_ids = create_slice_view(cell_ids, cell_ids_length as usize);

    // Computation
    //
    let (recovered_cells, recovered_proofs) = ctx
        .recover_cells_and_proofs(cell_ids.to_vec(), cells)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // Write to output
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_cells, recovered_cells);
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_proofs, recovered_proofs);

    Ok(())
}
