use c_peerdas_kzg::CResultStatus;
use c_peerdas_kzg::PeerDASContext;
use c_peerdas_kzg::BYTES_PER_CELL;
use c_peerdas_kzg::BYTES_PER_COMMITMENT;
use eip7594::constants::CELLS_PER_EXT_BLOB;
use jni::objects::JObjectArray;
use jni::objects::{JByteArray, JClass, JLongArray, JObject, JValue};
use jni::sys::jsize;
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;

// Note: These methods will use the c crate instead of directly calling Rust.
// This reduces the attack surface for all of the bindings.
// The c crate is a thin wrapper around the KZG Rust API.

// TODO: Convert the unwraps into Exceptions
// TODO: The java code solely uses `java/lang/IllegalArgumentException`
// TODO: swap for custom exception or use a more specific exception

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

    let mut out_cells = vec![vec![0u8; BYTES_PER_CELL]; CELLS_PER_EXT_BLOB];
    let mut out_proofs = vec![vec![0u8; BYTES_PER_COMMITMENT]; CELLS_PER_EXT_BLOB];

    let out_cells_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8_mut(&mut out_cells);
    let out_proofs_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8_mut(&mut out_proofs);

    let result = c_peerdas_kzg::compute_cells_and_kzg_proofs(
        ctx,
        blob.len() as u64,
        blob.as_ptr(),
        out_cells_ptr_ptr,
        out_proofs_ptr_ptr,
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

    return cells_and_proofs_to_jobject(&mut env, &out_cells, &out_proofs).unwrap();
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

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_verifyCellKZGProofBatch<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    commitment_bytes: JObjectArray<'local>,
    row_indices: JLongArray,
    column_indices: JLongArray,
    cells: JObjectArray<'local>,
    proofs: JObjectArray<'local>,
) -> jboolean {
    let ctx = ctx_ptr as *const PeerDASContext;

    let commitment_bytes = jobject_array_to_2d_byte_array(&mut env, commitment_bytes).unwrap();
    for commitment in &commitment_bytes {
        if commitment.len() != BYTES_PER_COMMITMENT {
            let err_msg = "All commitments must be the same length";
            env.throw_new("java/lang/IllegalArgumentException", err_msg)
                .expect("Failed to throw exception for `verify_cell_kzg_proof_batch`");
            return jboolean::default();
        }
    }
    let commitment_bytes_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8(&commitment_bytes);

    let commitment_bytes_length = commitment_bytes.len() as u64;

    let row_indices = jlongarray_to_vec_u64(&env, row_indices);
    let row_indices_length = row_indices.len() as u64;

    let column_indices = jlongarray_to_vec_u64(&env, column_indices);
    let column_indices_length = column_indices.len() as u64;

    let cells = jobject_array_to_2d_byte_array(&mut env, cells).unwrap();
    for cell in &cells {
        if cell.len() != BYTES_PER_CELL {
            let err_msg = "All cells must be the same length";
            env.throw_new("java/lang/IllegalArgumentException", err_msg)
                .expect("Failed to throw exception for `verify_cell_kzg_proof_batch`");
            return jboolean::default();
        }
    }
    let cells_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8(&cells);

    let proofs = jobject_array_to_2d_byte_array(&mut env, proofs).unwrap();
    for proof in &proofs {
        if proof.len() != BYTES_PER_COMMITMENT {
            let err_msg = "All proofs must be the same length";
            env.throw_new("java/lang/IllegalArgumentException", err_msg)
                .expect("Failed to throw exception for `verify_cell_kzg_proof_batch`");
            return jboolean::default();
        }
    }
    let proofs_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8(&proofs);

    let cells_length = cells.len() as u64;
    let proofs_length = proofs.len() as u64;

    let mut verified = false;
    let verified_ptr: *mut bool = (&mut verified) as *mut bool;

    let result = c_peerdas_kzg::verify_cell_kzg_proof_batch(
        ctx,
        commitment_bytes_length,
        commitment_bytes_ptr_ptr,
        row_indices_length,
        row_indices.as_ptr(),
        column_indices_length,
        column_indices.as_ptr(),
        cells_length,
        cells_ptr_ptr,
        proofs_length,
        proofs_ptr_ptr,
        verified_ptr,
    );

    if let CResultStatus::Err = result.status {
        let err_msg = construct_error_message(
            "Failed to compute `verify_cell_kzg_proof_batch`",
            result.error_msg,
        );
        env.throw_new("java/lang/IllegalArgumentException", err_msg)
            .expect("Failed to throw exception for `verify_cell_kzg_proof_batch`");
        return jboolean::default();
    }

    return jboolean::from(verified);
}

#[no_mangle]
pub unsafe extern "system" fn Java_ethereum_cryptography_LibPeerDASKZG_recoverCellsAndProof<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    cell_ids: JLongArray,
    cells: JObjectArray<'local>,
) -> JObject<'local> {
    let ctx = ctx_ptr as *const PeerDASContext;

    let cell_ids = jlongarray_to_vec_u64(&env, cell_ids);
    let cell_ids_length = cell_ids.len() as u64;

    let cells = match jobject_array_to_2d_byte_array(&mut env, cells) {
        Ok(cells) => cells,
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", &err.to_string())
                .expect("Failed to throw exception for `recover_all_cells`");
            return JObject::default();
        }
    };

    // check that all cells are the same length
    for cell in &cells {
        if cell.len() != BYTES_PER_CELL {
            let err_msg = "All cells must be the same length";
            env.throw_new("java/lang/IllegalArgumentException", err_msg)
                .expect("Failed to throw exception for `recover_all_cells`");
            return JObject::default();
        }
    }

    let mut out_cells = vec![vec![0u8; BYTES_PER_CELL]; CELLS_PER_EXT_BLOB];
    let mut out_proofs = vec![vec![0u8; BYTES_PER_COMMITMENT]; CELLS_PER_EXT_BLOB];

    let cells_length = cells.len() as u64;
    let cells_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8(&cells);

    let out_cells_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8_mut(&mut out_cells);
    let out_proofs_ptr_ptr = vec_vec_u8_to_ptr_ptr_u8_mut(&mut out_proofs);

    let result = c_peerdas_kzg::recover_cells_and_proofs(
        ctx,
        cells_length,
        cells_ptr_ptr,
        cell_ids_length,
        cell_ids.as_ptr(),
        out_cells_ptr_ptr,
        out_proofs_ptr_ptr,
    );

    if let CResultStatus::Err = result.status {
        let err_msg =
            construct_error_message("Failed to compute `recover_all_cells`", result.error_msg);
        env.throw_new("java/lang/IllegalArgumentException", err_msg)
            .expect("Failed to throw exception for `recover_all_cells`");
        return JObject::default();
    }

    return cells_and_proofs_to_jobject(&mut env, &out_cells, &out_proofs).unwrap();
}

fn jlongarray_to_vec_u64(env: &JNIEnv, array: JLongArray) -> Vec<u64> {
    // Step 1: Get the length of the JLongArray
    let array_length = env
        .get_array_length(&array)
        .expect("Unable to get array length");

    // Step 2: Create a buffer to hold the jlong elements (these are i64s)
    let mut buffer: Vec<i64> = vec![0; array_length as usize];

    // Step 3: Get the elements from the JLongArray
    env.get_long_array_region(array, 0, &mut buffer)
        .expect("Unable to get array region");

    // Step 4: Convert the Vec<i64> to Vec<u64>
    buffer.into_iter().map(|x| x as u64).collect()
}

fn jobject_array_to_2d_byte_array(
    env: &mut JNIEnv,
    array: JObjectArray,
) -> Result<Vec<Vec<u8>>, jni::errors::Error> {
    // Get the length of the outer array
    let outer_len = env.get_array_length(&array)?;

    let mut result = Vec::with_capacity(outer_len as usize);

    for i in 0..outer_len {
        // Get each inner array (JByteArray)
        let inner_array_obj = env.get_object_array_element(&array, i)?;
        let inner_array: JByteArray = JByteArray::from(inner_array_obj);

        // Get the length of the inner array
        let inner_len = env.get_array_length(&inner_array)?;

        // Get the elements of the inner array
        let mut buf = vec![0; inner_len as usize];
        env.get_byte_array_region(inner_array, 0, &mut buf)?;

        // Convert i8 to u8
        let buf = buf.into_iter().map(|x| x as u8).collect();

        result.push(buf);
    }

    Ok(result)
}

fn vec2d_to_jobject_array<'local>(
    env: &mut JNIEnv<'local>,
    vec: &[Vec<u8>],
) -> Result<JObjectArray<'local>, jni::errors::Error> {
    // Create a new jobjectArray with the same length as the outer vector
    let outer_len = vec.len() as jsize;
    let byte_array_class = env.find_class("[B")?;
    let mut outer_array =
        env.new_object_array(outer_len, byte_array_class, JObjectArray::default())?;

    for (i, inner_vec) in vec.into_iter().enumerate() {
        // Convert each inner Vec<u8> to a JByteArray
        let inner_array = env.byte_array_from_slice(&inner_vec)?;

        // Set the inner array into the outer array
        env.set_object_array_element(&mut outer_array, i as jsize, inner_array)?;
    }

    Ok(outer_array)
}

fn vec_vec_u8_to_ptr_ptr_u8(vec: &Vec<Vec<u8>>) -> *const *const u8 {
    // Create a Vec<*const u8> to hold the pointers to each inner Vec<u8>
    let ptrs: Vec<*const u8> = vec.iter().map(|inner_vec| inner_vec.as_ptr()).collect();

    // Get a pointer to the first element of the ptrs vector
    let ptr_to_ptrs: *const *const u8 = ptrs.as_ptr();

    // Prevent the ptrs vector from being deallocated
    std::mem::forget(ptrs);

    ptr_to_ptrs
}

fn vec_vec_u8_to_ptr_ptr_u8_mut(vec: &mut Vec<Vec<u8>>) -> *mut *mut u8 {
    // Create a Vec<*mut u8> to hold the pointers to each inner Vec<u8>
    let mut ptrs: Vec<*mut u8> = vec
        .iter_mut()
        .map(|inner_vec| inner_vec.as_mut_ptr())
        .collect();

    // Get a pointer to the first element of the ptrs vector
    let ptr_to_ptrs: *mut *mut u8 = ptrs.as_mut_ptr();

    // Prevent the ptrs vector from being deallocated
    std::mem::forget(ptrs);

    ptr_to_ptrs
}

fn cells_and_proofs_to_jobject<'local>(
    env: &mut JNIEnv<'local>,
    cells: &[Vec<u8>],
    proofs: &[Vec<u8>],
) -> Result<JObject<'local>, jni::errors::Error> {
    // Create a new instance of the CellsAndProofs class in Java
    let cells_and_proofs_class = env
        .find_class("ethereum/cryptography/CellsAndProofs")
        .unwrap();

    let cell_byte_array_class = env.find_class("[B").unwrap();
    let proof_byte_array_class = env.find_class("[B").unwrap();

    // Create 2D array for cells
    let cells_array = env
        .new_object_array(
            cells.len() as i32,
            cell_byte_array_class,
            env.new_byte_array(0).unwrap(),
        )
        .unwrap();

    for (i, cell) in cells.into_iter().enumerate() {
        let cell_array = env.byte_array_from_slice(cell).unwrap();
        env.set_object_array_element(&cells_array, i as i32, cell_array)
            .unwrap();
    }

    // Create 2D array for proofs
    let proofs_array = env
        .new_object_array(
            proofs.len() as i32,
            proof_byte_array_class,
            env.new_byte_array(0).unwrap(),
        )
        .unwrap();

    for (i, proof) in proofs.into_iter().enumerate() {
        let proof_array = env.byte_array_from_slice(proof).unwrap();
        env.set_object_array_element(&proofs_array, i as i32, proof_array)
            .unwrap();
    }

    // Create the CellsAndProofs object
    let cells_and_proofs_obj = env
        .new_object(
            cells_and_proofs_class,
            "([[B[[B)V",
            &[JValue::Object(&cells_array), JValue::Object(&proofs_array)],
        )
        .unwrap();

    Ok(cells_and_proofs_obj)
}
