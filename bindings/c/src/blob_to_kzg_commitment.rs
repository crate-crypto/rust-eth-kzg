use eip7594::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::pointer_utils::{create_slice_view_with_null, deref_const, write_to_slice_with_null};
use crate::{CResult, PeerDASContext};

pub(crate) fn _blob_to_kzg_commitment(
    ctx: *const PeerDASContext,
    blob: *const u8,
    out: *mut u8,
) -> Result<(), CResult> {
    // Dereference the input pointers
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();

    let blob = create_slice_view_with_null(blob, BYTES_PER_BLOB)
        .map_err(|_| CResult::with_error("could not dereference pointer to blob"))?;

    // Computation
    //
    let commitment = ctx
        .blob_to_kzg_commitment(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    assert!(
        commitment.len() == BYTES_PER_COMMITMENT as usize,
        "This is a library bug. commitment.len() != BYTES_PER_COMMITMENT, {} != {}",
        commitment.len(),
        BYTES_PER_COMMITMENT
    );

    // Write output to slice
    //
    write_to_slice_with_null(out, &commitment)
        .map_err(|_| CResult::with_error("could not write commitment to output"))?;

    Ok(())
}
