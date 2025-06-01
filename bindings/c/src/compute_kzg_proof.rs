use rust_eth_kzg::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT};

use crate::{
    pointer_utils::{create_array_ref, deref_const, write_to_slice},
    CResult, DASContext,
};

pub(crate) fn _compute_kzg_proof(
    ctx: *const DASContext,
    blob: *const u8,
    z: *const u8,
    out_proof: *mut u8,
    out_y: *mut u8,
) -> Result<(), CResult> {
    assert!(!ctx.is_null(), "context pointer is null");

    // Dereference the input pointers
    //
    let ctx = deref_const(ctx);
    let blob = create_array_ref::<BYTES_PER_BLOB, _>(blob);
    let z = create_array_ref::<BYTES_PER_FIELD_ELEMENT, _>(z);

    // Computation
    //
    let (proof, y) = ctx
        .compute_kzg_proof(blob, *z)
        .map_err(|err| CResult::with_error(&format!("{:?}", err)))?;

    assert!(
        proof.len() == BYTES_PER_COMMITMENT,
        "This is a library bug. proof.len() != BYTES_PER_COMMITMENT, {} != {}",
        proof.len(),
        BYTES_PER_COMMITMENT
    );

    assert!(
        y.len() == BYTES_PER_FIELD_ELEMENT,
        "This is a library bug. y.len() != BYTES_PER_FIELD_ELEMENT, {} != {}",
        y.len(),
        BYTES_PER_FIELD_ELEMENT
    );

    // Write output to slices
    //
    write_to_slice(out_proof, &proof);
    write_to_slice(out_y, &y);

    Ok(())
}
