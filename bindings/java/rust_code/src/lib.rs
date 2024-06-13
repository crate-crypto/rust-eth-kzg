use c_peerdas_kzg::PeerDASContext;
use eip7594::verifier::VerifierError;
use jni::objects::JObjectArray;
use jni::objects::{JByteArray, JClass, JLongArray, JObject, JValue};
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;

// TODO: Convert the unwraps into Exceptions
// TODO: The java code solely uses `java/lang/IllegalArgumentException`
// TODO: swap for custom exception or use a more specific exception

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
    let ctx = &*(ctx_ptr as *const PeerDASContext);
    let blob = env.convert_byte_array(blob).unwrap();

    let (cells, proofs) = match ctx.prover_ctx().compute_cells_and_kzg_proofs(&blob) {
        Ok(cells_and_proofs) => cells_and_proofs,
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", format!("{:?}", err))
                .expect("Failed to throw exception for `compute_cells_and_kzg_proofs`");
            return JObject::default();
        }
    };

    return cells_and_proofs_to_jobject(&mut env, &cells, &proofs).unwrap();
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
    let ctx = &*(ctx_ptr as *const PeerDASContext);
    let blob = env.convert_byte_array(blob).unwrap();

    let commitment = match ctx.prover_ctx().blob_to_kzg_commitment(&blob) {
        Ok(commitment) => commitment,
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", format!("{:?}", err))
                .expect("Failed to throw exception for `blob_to_kzg_commitment`");
            return JByteArray::default();
        }
    };

    return env.byte_array_from_slice(&commitment).unwrap();
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
    let ctx = &*(ctx_ptr as *const PeerDASContext);

    let commitment_bytes = env.convert_byte_array(&commitment_bytes).unwrap();
    let cell_id = cell_id as u64;
    let cell = env.convert_byte_array(&cell).unwrap();
    let proof_bytes = env.convert_byte_array(&proof_bytes).unwrap();

    match ctx
        .verifier_ctx()
        .verify_cell_kzg_proof(&commitment_bytes, cell_id, &cell, &proof_bytes)
    {
        Ok(_) => return jboolean::from(true),
        Err(VerifierError::InvalidProof) => {
            return jboolean::from(false);
        }
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", format!("{:?}", err))
                .expect("Failed to throw exception for `verify_cell_kzg_proof`");
            return jboolean::default();
        }
    }
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
    let ctx = &*(ctx_ptr as *const PeerDASContext);

    let commitment_bytes = jobject_array_to_2d_byte_array(&mut env, commitment_bytes).unwrap();
    let row_indices = jlongarray_to_vec_u64(&env, row_indices);
    let column_indices = jlongarray_to_vec_u64(&env, column_indices);
    let cells = jobject_array_to_2d_byte_array(&mut env, cells).unwrap();
    let proofs = jobject_array_to_2d_byte_array(&mut env, proofs).unwrap();

    match ctx.verifier_ctx().verify_cell_kzg_proof_batch(
        commitment_bytes,
        row_indices,
        column_indices,
        cells.iter().map(|cell| cell.as_slice()).collect(),
        proofs,
    ) {
        Ok(_) => return jboolean::from(true),
        Err(VerifierError::InvalidProof) => {
            return jboolean::from(false);
        }
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", format!("{:?}", err))
                .expect("Failed to throw exception for `verify_cell_kzg_proof_batch`");
            return jboolean::default();
        }
    }
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
    let ctx = &*(ctx_ptr as *const PeerDASContext);

    let cell_ids = jlongarray_to_vec_u64(&env, cell_ids);

    let cells = match jobject_array_to_2d_byte_array(&mut env, cells) {
        Ok(cells) => cells,
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", &err.to_string())
                .expect("Failed to throw exception for `recover_all_cells`");
            return JObject::default();
        }
    };

    let (recovered_cells, recovered_proofs) = match ctx.prover_ctx().recover_cells_and_proofs(
        cell_ids,
        cells.iter().map(|x| x.as_slice()).collect(),
        vec![],
    ) {
        Ok(recovered_cells_and_proofs) => recovered_cells_and_proofs,
        Err(err) => {
            env.throw_new("java/lang/IllegalArgumentException", format!("{:?}", err))
                .expect("Failed to throw exception for `recover_all_cells`");
            return JObject::default();
        }
    };

    return cells_and_proofs_to_jobject(&mut env, &recovered_cells, &recovered_proofs).unwrap();
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

fn cells_and_proofs_to_jobject<'local>(
    env: &mut JNIEnv<'local>,
    cells: &[impl AsRef<[u8]>],
    proofs: &[impl AsRef<[u8]>],
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
        let cell_array = env.byte_array_from_slice(cell.as_ref()).unwrap();
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
        let proof_array = env.byte_array_from_slice(proof.as_ref()).unwrap();
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
