use crate::pointer_utils::{
    create_slice_view_with_null, deref_const, write_to_slice_slice_with_null,
};
use crate::{CResult, PeerDASContext};
use eip7594::constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB};

pub(crate) fn _compute_cells_and_kzg_proofs(
    ctx: *const PeerDASContext,
    blob: *const u8,
    out_cells: *mut *mut u8,
    out_proofs: *mut *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();
    let blob = create_slice_view_with_null(blob, BYTES_PER_BLOB)
        .map_err(|_| CResult::with_error("could not dereference pointer to blob"))?;

    // Computation
    //
    let (cells, proofs) = ctx
        .compute_cells_and_kzg_proofs(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    // Write to output
    write_to_slice_slice_with_null::<_, CELLS_PER_EXT_BLOB>(out_cells, cells)
        .map_err(|_| CResult::with_error("could not write cells to output"))?;

    write_to_slice_slice_with_null::<_, CELLS_PER_EXT_BLOB>(out_proofs, proofs)
        .map_err(|_| CResult::with_error("could not write proofs to output"))?;

    Ok(())
}
