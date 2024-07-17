use crate::pointer_utils::{
    create_slice_view, deref_const, ptr_ptr_to_vec_slice_const, write_to_2d_slice,
};
use crate::{CResult, PeerDASContext};
use rust_eth_kzg::constants::{BYTES_PER_CELL, CELLS_PER_EXT_BLOB};

pub(crate) fn _recover_cells_and_proofs(
    ctx: *const PeerDASContext,
    cells_length: u64,
    cells: *const *const u8,
    cell_indices_length: u64,
    cell_indices: *const u64,
    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx).inner();
    let cells = ptr_ptr_to_vec_slice_const::<BYTES_PER_CELL>(cells, cells_length as usize);
    let cell_indices = create_slice_view(cell_indices, cell_indices_length as usize);

    // Computation
    //
    let (recovered_cells, recovered_proofs) = ctx
        .recover_cells_and_proofs(cell_indices.to_vec(), cells)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;
    let recovered_cells_unboxed = recovered_cells.map(|cell| cell.to_vec());

    // Write to output
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_cells, recovered_cells_unboxed);
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_proofs, recovered_proofs);

    Ok(())
}
