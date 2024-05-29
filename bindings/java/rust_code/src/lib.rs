use eip7594::prover::ProverContext;
use eip7594::verifier::VerifierContext;

use jni::objects::JByteArray;
use jni::objects::JClass;
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_proverContextNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let prover_context = ProverContext::new();

    Box::into_raw(Box::new(prover_context)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_proverContextDestroy(
    _env: JNIEnv,
    _class: JClass,
    prover_context_ptr: jlong,
) {
    let _boxed_prover_context = Box::from_raw(prover_context_ptr as *mut ProverContext);
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_computeCells<'local>(
    env: JNIEnv<'local>,
    _class: JClass,
    prover_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let prover_ctx = &mut *(prover_ptr as *mut ProverContext);
    let blob = env.convert_byte_array(blob).unwrap();
    let cells = prover_ctx.compute_cells(&blob).unwrap();

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
    prover_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let prover_ctx = &mut *(prover_ptr as *mut ProverContext);

    let blob = env.convert_byte_array(blob).unwrap();

    let (cells, proofs) = prover_ctx.compute_cells_and_kzg_proofs(&blob).unwrap();

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
    prover_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let prover_ctx = &mut *(prover_ptr as *mut ProverContext);
    let blob = env.convert_byte_array(blob).unwrap();
    let commitment = prover_ctx.blob_to_kzg_commitment(&blob).unwrap();

    return env.byte_array_from_slice(&commitment).unwrap();
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_verifierContextNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let verifier_context = VerifierContext::new();
    Box::into_raw(Box::new(verifier_context)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_verifierContextDestroy(
    _env: JNIEnv,
    _class: JClass,
    verifier_context_ptr: jlong,
) {
    let _boxed_verifier_context = Box::from_raw(verifier_context_ptr as *mut VerifierContext);
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_verifyCellKZGProof<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass,
    verifier_ptr: jlong,
    commitment_bytes: JByteArray<'local>,
    cell_id: jlong,
    cell: JByteArray<'local>,
    proof_bytes: JByteArray<'local>,
) -> jboolean {
    let verifier_ctx = &mut *(verifier_ptr as *mut VerifierContext);

    let commitment_bytes = env.convert_byte_array(&commitment_bytes).unwrap();
    let cell_id = cell_id as u64;
    let cell = env.convert_byte_array(&cell).unwrap();
    let proof_bytes = env.convert_byte_array(&proof_bytes).unwrap();

    return jboolean::from(
        verifier_ctx
            .verify_cell_kzg_proof(&commitment_bytes, cell_id, &cell, &proof_bytes)
            .is_ok(),
    );
}
