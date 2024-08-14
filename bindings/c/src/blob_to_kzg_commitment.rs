use rust_eth_kzg::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};

use crate::pointer_utils::{create_array_ref, deref_const, write_to_slice};
use crate::{CResult, DASContext};

pub(crate) fn _blob_to_kzg_commitment(
    ctx: *const DASContext,
    blob: *const u8,
    out: *mut u8,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
    let blob = create_array_ref::<BYTES_PER_BLOB, _>(blob);

    // Computation
    //
    let commitment = ctx
        .blob_to_kzg_commitment(blob)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    assert!(
        commitment.len() == BYTES_PER_COMMITMENT,
        "This is a library bug. commitment.len() != BYTES_PER_COMMITMENT, {} != {}",
        commitment.len(),
        BYTES_PER_COMMITMENT
    );

    // Write output to slice
    //
    write_to_slice(out, &commitment);

    Ok(())
}
