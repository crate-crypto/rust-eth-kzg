use eip7594::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::pointer_utils::{create_slice_view, deref_const, write_to_slice};
use crate::{CResult, PeerDASContext};

pub(crate) fn _blob_to_kzg_commitment(
    ctx: *const PeerDASContext,
    blob: *const u8,
    out: *mut u8,
) -> Result<(), CResult> {
    assert!(ctx.is_null() == false, "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx).prover_ctx();
    let blob = create_slice_view(blob, BYTES_PER_BLOB);

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
    write_to_slice(out, &commitment);

    Ok(())
}