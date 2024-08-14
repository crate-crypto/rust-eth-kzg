use crate::pointer_utils::{create_array_ref, deref_const, write_to_2d_slice};
use crate::{CResult, DASContext};
use rust_eth_kzg::constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB};

pub(crate) fn _compute_cells_and_kzg_proofs(
    ctx: *const DASContext,
    blob: *const u8,
    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Pointer checks
    //
    let ctx = deref_const(ctx);
    let blob = create_array_ref::<BYTES_PER_BLOB, _>(blob);

    // Computation
    //
    let (cells, proofs) = ctx
        .compute_cells_and_kzg_proofs(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;
    let cells_unboxed = cells.map(|cell| cell.to_vec());

    // Write to output
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_cells, cells_unboxed);
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_proofs, proofs);

    Ok(())
}
