use crate::pointer_utils::{create_slice_view, deref_const, write_to_2d_slice};
use crate::{CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB};

pub(crate) fn _compute_cells_and_kzg_proofs(
    ctx: *const PeerDASContext,
    blob: *const u8,
    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Pointer checks
    //
    let ctx = deref_const(ctx).prover_ctx();
    let blob = create_slice_view(blob, BYTES_PER_BLOB);

    // Computation
    //
    let (cells, proofs) = ctx
        .compute_cells_and_kzg_proofs(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // Write to output
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_cells, cells);
    write_to_2d_slice::<_, CELLS_PER_EXT_BLOB>(out_proofs, proofs);

    Ok(())
}
