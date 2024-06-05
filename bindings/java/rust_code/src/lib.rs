use c_peerdas_kzg::CResultStatus;
use c_peerdas_kzg::PeerDASContext;
use c_peerdas_kzg::BYTES_PER_COMMITMENT;
use jni::objects::{JByteArray, JClass, JObject, JValue};
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;

// Note: These methods will use the c crate instead of directly calling Rust.
// This reduces the attack surface for all of the bindings.
// The c crate is a thin wrapper around the KZG Rust API.

fn construct_error_message(msg_prefix: &str, msg_body: *mut i8) -> String {
    unsafe {
        // Check if msg is null
        let msg_body = msg_body.as_mut();
        let msg_body = match msg_body {
            None => return msg_prefix.to_string(),
            Some(msg) => msg,
        };

        // Concatenate the prefix and the body
        let error_message = msg_prefix.to_string()
            + ": "
            + std::ffi::CStr::from_ptr(msg_body)
                .to_string_lossy()
                .into_owned()
                .as_str();

        // free the error message
        c_peerdas_kzg::free_error_message(msg_body);

        error_message
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_peerDASContextNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    c_peerdas_kzg::peerdas_context_new() as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_peerDASContextDestroy(
    _env: JNIEnv,
    _class: JClass,
    ctx_ptr: jlong,
) {
    c_peerdas_kzg::peerdas_context_free(ctx_ptr as *mut PeerDASContext);
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_computeCells<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = ctx_ptr as *const PeerDASContext;
    let blob = env.convert_byte_array(blob).unwrap();

    let mut out_cells = vec![0u8; c_peerdas_kzg::NUM_BYTES_CELLS as usize];

    let result = c_peerdas_kzg::compute_cells(
        ctx,
        blob.len() as u64,
        blob.as_ptr(),
        out_cells.as_mut_ptr(),
    );

    if let CResultStatus::Err = result.status {
        let err_msg =
            construct_error_message("Failed to compute `compute_cells`", result.error_msg);
        env.throw_new("java/lang/IllegalArgumentException", err_msg)
            .expect("Failed to throw exception for `compute_cells`");
        return JByteArray::default();
    }

    return env.byte_array_from_slice(&out_cells).unwrap();
}

#[repr(C)]
pub struct CellsAndProofs {
    pub cells: [u8; c_peerdas_kzg::NUM_BYTES_CELLS as usize],
    pub proofs: [u8; c_peerdas_kzg::NUM_BYTES_PROOFS as usize],
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_computeCellsAndKZGProofs<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JObject<'local> {
    let ctx = ctx_ptr as *const PeerDASContext;
    let blob = env.convert_byte_array(blob).unwrap();

    let mut cells_and_proofs = CellsAndProofs {
        cells: [0u8; c_peerdas_kzg::NUM_BYTES_CELLS as usize],
        proofs: [0u8; c_peerdas_kzg::NUM_BYTES_PROOFS as usize],
    };

    let result = c_peerdas_kzg::compute_cells_and_kzg_proofs(
        ctx,
        blob.len() as u64,
        blob.as_ptr(),
        cells_and_proofs.cells.as_mut_ptr(),
        cells_and_proofs.proofs.as_mut_ptr(),
    );

    if let CResultStatus::Err = result.status {
        let err_msg = construct_error_message(
            "Failed to compute `compute_cells_and_kzg_proofs`",
            result.error_msg,
        );
        env.throw_new("java/lang/IllegalArgumentException", err_msg)
            .expect("Failed to throw exception for `compute_cells_and_kzg_proofs`");
        return JObject::default();
    }

    // Create a new instance of the CellsAndProofs class in Java
    let cells_and_proofs_class = env
        .find_class("ethereum/cryptography/CellsAndProofs")
        .unwrap();
    let cells_and_proofs_obj = env
        .new_object(
            cells_and_proofs_class,
            "([B[B)V",
            &[
                JValue::Object(&env.byte_array_from_slice(&cells_and_proofs.cells).unwrap()),
                JValue::Object(&env.byte_array_from_slice(&cells_and_proofs.proofs).unwrap()),
            ],
        )
        .unwrap();
    cells_and_proofs_obj
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_blobToKZGCommitment<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = ctx_ptr as *const PeerDASContext;
    let blob = env.convert_byte_array(blob).unwrap();

    let mut out = vec![0u8; BYTES_PER_COMMITMENT as usize];

    let result = c_peerdas_kzg::blob_to_kzg_commitment(
        ctx,
        blob.len() as u64,
        blob.as_ptr(),
        out.as_mut_ptr(),
    );

    if let CResultStatus::Err = result.status {
        let err_msg = construct_error_message(
            "Failed to compute `blob_to_kzg_commitment`",
            result.error_msg,
        );
        env.throw_new("java/lang/IllegalArgumentException", err_msg)
            .expect("Failed to throw exception for `blob_to_kzg_commitment`");
        return JByteArray::default();
    }

    return env.byte_array_from_slice(&out).unwrap();
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_verifyCellKZGProof<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    commitment_bytes: JByteArray<'local>,
    cell_id: jlong,
    cell: JByteArray<'local>,
    proof_bytes: JByteArray<'local>,
) -> jboolean {
    let ctx = ctx_ptr as *const PeerDASContext;

    let commitment_bytes = env.convert_byte_array(&commitment_bytes).unwrap();
    let cell_id = cell_id as u64;
    let cell = env.convert_byte_array(&cell).unwrap();
    let proof_bytes = env.convert_byte_array(&proof_bytes).unwrap();

    let mut verified = false;
    let verified_ptr: *mut bool = (&mut verified) as *mut bool;

    let result = c_peerdas_kzg::verify_cell_kzg_proof(
        ctx,
        cell.len() as u64,
        cell.as_ptr(),
        commitment_bytes.len() as u64,
        commitment_bytes.as_ptr(),
        cell_id,
        proof_bytes.len() as u64,
        proof_bytes.as_ptr(),
        verified_ptr,
    );

    if let CResultStatus::Err = result.status {
        let err_msg = construct_error_message(
            "Failed to compute `verify_cell_kzg_proof`",
            result.error_msg,
        );
        env.throw_new("java/lang/IllegalArgumentException", err_msg)
            .expect("Failed to throw exception for `verify_cell_kzg_proof`");
        return jboolean::default();
    }

    return jboolean::from(verified);
}
