use c_peerdas_kzg::PeerDASContext;
use jni::objects::JByteArray;
use jni::objects::JClass;
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_peerDASContextNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let context = PeerDASContext::new();

    Box::into_raw(Box::new(context)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_peerDASContextDestroy(
    _env: JNIEnv,
    _class: JClass,
    ctx_ptr: jlong,
) {
    let _boxed_prover_context = Box::from_raw(ctx_ptr as *mut PeerDASContext);
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_computeCells<'local>(
    env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = &mut *(ctx_ptr as *mut PeerDASContext);
    let blob = env.convert_byte_array(blob).unwrap();
    let cells = ctx.prover_ctx().unwrap().compute_cells(&blob).unwrap();

    let flattened_cells = cells
        .iter()
        .flat_map(|cell| cell.iter())
        .copied()
        .collect::<Vec<u8>>();

    return env.byte_array_from_slice(&flattened_cells).unwrap();
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_computeCellsAndKZGProofs<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = &mut *(ctx_ptr as *mut PeerDASContext);

    let blob = env.convert_byte_array(blob).unwrap();

    let (cells, proofs) = ctx
        .prover_ctx()
        .unwrap()
        .compute_cells_and_kzg_proofs(&blob)
        .unwrap();

    let flattened_proofs_and_cells: Vec<u8> = cells
        .into_iter()
        .zip(proofs.into_iter())
        .flat_map(|(cell, proof)| cell.into_iter().chain(proof.into_iter()))
        .collect();

    return env
        .byte_array_from_slice(&flattened_proofs_and_cells)
        .unwrap();
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_blobToKZGCommitment<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = &mut *(ctx_ptr as *mut PeerDASContext);
    let blob = env.convert_byte_array(blob).unwrap();
    let commitment = ctx
        .prover_ctx()
        .unwrap()
        .blob_to_kzg_commitment(&blob)
        .unwrap();

    return env.byte_array_from_slice(&commitment).unwrap();
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_verifyCellKZGProof<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    commitment_bytes: JByteArray<'local>,
    cell_id: jlong,
    cell: JByteArray<'local>,
    proof_bytes: JByteArray<'local>,
) -> jboolean {
    let ctx = &mut *(ctx_ptr as *mut PeerDASContext);

    let commitment_bytes = env.convert_byte_array(&commitment_bytes).unwrap();
    let cell_id = cell_id as u64;
    let cell = env.convert_byte_array(&cell).unwrap();
    let proof_bytes = env.convert_byte_array(&proof_bytes).unwrap();

    return jboolean::from(
        ctx.verifier_ctx()
            .unwrap()
            .verify_cell_kzg_proof(&commitment_bytes, cell_id, &cell, &proof_bytes)
            .is_ok(),
    );
}
