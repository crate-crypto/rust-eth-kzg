use eip7594::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::pointer_utils::{create_slice_view, deref_const, deref_mut, write_to_slice};
use crate::{CResult, PeerDASContext};

pub(crate) fn _blob_to_kzg_commitment(
    ctx: *const PeerDASContext,
    blob_length: u64,
    blob: *const u8,
    out: *mut u8,
) -> Result<(), CResult> {
    // Pointer checks
    //
    let ctx = deref_const(ctx)
        .map_err(|_| CResult::with_error("context has a null ptr"))?
        .prover_ctx();
    let blob = deref_const(blob)
        .map_err(|_| CResult::with_error("could not dereference pointer to blob"))?;
    let out = deref_mut(out)
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
