use c_eth_kzg::DASContext;
use jni::{
    objects::{JByteArray, JClass, JLongArray, JObject, JObjectArray, JValue},
    sys::{jboolean, jlong},
    JNIEnv,
};

mod errors;
use errors::Error;

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_DASContextNew(
    _env: JNIEnv,
    _class: JClass,
    use_precomp: jboolean,
) -> jlong {
    let use_precomp = use_precomp != 0;
    c_eth_kzg::eth_kzg_das_context_new(use_precomp) as jlong
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_DASContextDestroy(
    _env: JNIEnv,
    _class: JClass,
    ctx_ptr: jlong,
) {
    c_eth_kzg::eth_kzg_das_context_free(ctx_ptr as *mut DASContext);
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_computeCellsAndKZGProofs<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JObject<'local> {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };
    match compute_cells_and_kzg_proofs(&mut env, ctx, blob) {
        Ok(cells_and_proofs) => cells_and_proofs,
        Err(err) => {
            throw_on_error(&mut env, err, "computeCellsAndKZGProofs");
            JObject::default()
        }
    }
}
fn compute_cells_and_kzg_proofs<'local>(
    env: &mut JNIEnv<'local>,
    ctx: &DASContext,
    blob: JByteArray<'local>,
) -> Result<JObject<'local>, Error> {
    let blob = env.convert_byte_array(blob)?;
    let blob = slice_to_array_ref(&blob, "blob")?;

    let (cells, proofs) = ctx.compute_cells_and_kzg_proofs(blob)?;
    let cells = cells.map(|cell| *cell);
    cells_and_proofs_to_jobject(env, &cells, &proofs)
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_computeCells<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JObject<'local> {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };
    match compute_cells(&mut env, ctx, blob) {
        Ok(cells) => cells,
        Err(err) => {
            throw_on_error(&mut env, err, "computeCells");
            JObject::default()
        }
    }
}
fn compute_cells<'local>(
    env: &mut JNIEnv<'local>,
    ctx: &DASContext,
    blob: JByteArray<'local>,
) -> Result<JObject<'local>, Error> {
    let blob = env.convert_byte_array(blob)?;
    let blob = slice_to_array_ref(&blob, "blob")?;

    let cells = ctx.compute_cells(blob)?;
    let cells = cells.map(|cell| *cell);
    cells_to_jobject(env, &cells)
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_blobToKZGCommitment<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };
    match blob_to_kzg_commitment(&env, ctx, blob) {
        Ok(commitment) => commitment,
        Err(err) => {
            throw_on_error(&mut env, err, "blobToKZGCommitment");
            JByteArray::default()
        }
    }
}
fn blob_to_kzg_commitment<'local>(
    env: &JNIEnv<'local>,
    ctx: &DASContext,
    blob: JByteArray<'local>,
) -> Result<JByteArray<'local>, Error> {
    let blob = env.convert_byte_array(blob)?;
    let blob = slice_to_array_ref(&blob, "blob")?;

    let commitment = ctx.blob_to_kzg_commitment(blob)?;
    env.byte_array_from_slice(&commitment).map_err(Error::from)
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_verifyCellKZGProofBatch<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    commitment: JObjectArray<'local>,
    cell_indices: JLongArray,
    cells: JObjectArray<'local>,
    proofs: JObjectArray<'local>,
) -> jboolean {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };

    match verify_cell_kzg_proof_batch(&mut env, ctx, &commitment, cell_indices, &cells, &proofs) {
        Ok(result) => result,
        Err(err) => {
            throw_on_error(&mut env, err, "verifyCellKZGProofBatch");
            jboolean::default()
        }
    }
}
fn verify_cell_kzg_proof_batch<'local>(
    env: &mut JNIEnv,
    ctx: &DASContext,
    commitment: &JObjectArray<'local>,
    cell_indices: JLongArray,
    cells: &JObjectArray<'local>,
    proofs: &JObjectArray<'local>,
) -> Result<jboolean, Error> {
    let commitment = jobject_array_to_2d_byte_array(env, commitment)?;
    let cell_indices = jlongarray_to_vec_u64(env, cell_indices)?;
    let cells = jobject_array_to_2d_byte_array(env, cells)?;
    let proofs = jobject_array_to_2d_byte_array(env, proofs)?;

    let cells: Vec<_> = cells
        .iter()
        .map(|cell| slice_to_array_ref(cell, "cell"))
        .collect::<Result<_, _>>()?;
    let commitments: Vec<_> = commitment
        .iter()
        .map(|commitment| slice_to_array_ref(commitment, "commitment"))
        .collect::<Result<_, _>>()?;
    let proofs: Vec<_> = proofs
        .iter()
        .map(|proof| slice_to_array_ref(proof, "proof"))
        .collect::<Result<_, _>>()?;

    match ctx.verify_cell_kzg_proof_batch(commitments, &cell_indices, cells, proofs) {
        Ok(()) => Ok(jboolean::from(true)),
        Err(x) if x.is_proof_invalid() => Ok(jboolean::from(false)),
        Err(err) => Err(Error::Cryptography(err)),
    }
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_recoverCellsAndKZGProofs<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    cell_ids: JLongArray,
    cells: JObjectArray<'local>,
) -> JObject<'local> {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };

    match recover_cells_and_kzg_proofs(&mut env, ctx, cell_ids, &cells) {
        Ok(cells_and_proofs) => cells_and_proofs,
        Err(err) => {
            throw_on_error(&mut env, err, "recoverCellsAndKZGProofs");
            JObject::default()
        }
    }
}
fn recover_cells_and_kzg_proofs<'local>(
    env: &mut JNIEnv<'local>,
    ctx: &DASContext,
    cell_ids: JLongArray,
    cells: &JObjectArray<'local>,
) -> Result<JObject<'local>, Error> {
    let cell_ids = jlongarray_to_vec_u64(env, cell_ids)?;
    let cells = jobject_array_to_2d_byte_array(env, cells)?;
    let cells: Vec<_> = cells
        .iter()
        .map(|cell| slice_to_array_ref(cell, "cell"))
        .collect::<Result<_, _>>()?;

    let (recovered_cells, recovered_proofs) = ctx.recover_cells_and_kzg_proofs(cell_ids, cells)?;
    let recovered_cells = recovered_cells.map(|cell| *cell);
    cells_and_proofs_to_jobject(env, &recovered_cells, &recovered_proofs)
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_computeKzgProof<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
    z: JByteArray<'local>,
) -> JObjectArray<'local> {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };
    match compute_kzg_proof(&mut env, ctx, blob, z) {
        Ok(result) => result,
        Err(err) => {
            throw_on_error(&mut env, err, "computeKzgProof");
            JObjectArray::default()
        }
    }
}
fn compute_kzg_proof<'local>(
    env: &mut JNIEnv<'local>,
    ctx: &DASContext,
    blob: JByteArray<'local>,
    z: JByteArray<'local>,
) -> Result<JObjectArray<'local>, Error> {
    let blob = env.convert_byte_array(blob)?;
    let blob = slice_to_array_ref(&blob, "blob")?;

    let z = env.convert_byte_array(z)?;
    let z = slice_to_array_ref(&z, "z")?;

    let (proof, y) = ctx.compute_kzg_proof(blob, *z)?;

    // Create a 2D byte array with proof and y
    let byte_array_class = env.find_class("[B")?;
    let result_array = env.new_object_array(2, byte_array_class, env.new_byte_array(0)?)?;

    let proof_array = env.byte_array_from_slice(&proof)?;
    let y_array = env.byte_array_from_slice(&y)?;

    env.set_object_array_element(&result_array, 0, proof_array)?;
    env.set_object_array_element(&result_array, 1, y_array)?;

    Ok(result_array)
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_computeBlobKzgProof<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
    commitment: JByteArray<'local>,
) -> JByteArray<'local> {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };
    match compute_blob_kzg_proof(&env, ctx, blob, commitment) {
        Ok(proof) => proof,
        Err(err) => {
            throw_on_error(&mut env, err, "computeBlobKzgProof");
            JByteArray::default()
        }
    }
}
fn compute_blob_kzg_proof<'local>(
    env: &JNIEnv<'local>,
    ctx: &DASContext,
    blob: JByteArray<'local>,
    commitment: JByteArray<'local>,
) -> Result<JByteArray<'local>, Error> {
    let blob = env.convert_byte_array(blob)?;
    let blob = slice_to_array_ref(&blob, "blob")?;

    let commitment = env.convert_byte_array(commitment)?;
    let commitment = slice_to_array_ref(&commitment, "commitment")?;

    let proof = ctx.compute_blob_kzg_proof(blob, commitment)?;
    env.byte_array_from_slice(&proof).map_err(Error::from)
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_verifyKzgProof<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    commitment: JByteArray<'local>,
    z: JByteArray<'local>,
    y: JByteArray<'local>,
    proof: JByteArray<'local>,
) -> jboolean {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };

    match verify_kzg_proof(&mut env, ctx, commitment, z, y, proof) {
        Ok(result) => result,
        Err(err) => {
            throw_on_error(&mut env, err, "verifyKzgProof");
            jboolean::default()
        }
    }
}
fn verify_kzg_proof<'local>(
    env: &mut JNIEnv,
    ctx: &DASContext,
    commitment: JByteArray<'local>,
    z: JByteArray<'local>,
    y: JByteArray<'local>,
    proof: JByteArray<'local>,
) -> Result<jboolean, Error> {
    let commitment = env.convert_byte_array(commitment)?;
    let commitment = slice_to_array_ref(&commitment, "commitment")?;

    let z = env.convert_byte_array(z)?;
    let z = slice_to_array_ref(&z, "z")?;

    let y = env.convert_byte_array(y)?;
    let y = slice_to_array_ref(&y, "y")?;

    let proof = env.convert_byte_array(proof)?;
    let proof = slice_to_array_ref(&proof, "proof")?;

    match ctx.verify_kzg_proof(commitment, *z, *y, proof) {
        Ok(()) => Ok(jboolean::from(true)),
        Err(x) if x.is_proof_invalid() => Ok(jboolean::from(false)),
        Err(err) => Err(Error::Cryptography(err)),
    }
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_verifyBlobKzgProof<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blob: JByteArray<'local>,
    commitment: JByteArray<'local>,
    proof: JByteArray<'local>,
) -> jboolean {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };

    match verify_blob_kzg_proof(&mut env, ctx, blob, commitment, proof) {
        Ok(result) => result,
        Err(err) => {
            throw_on_error(&mut env, err, "verifyBlobKzgProof");
            jboolean::default()
        }
    }
}
fn verify_blob_kzg_proof<'local>(
    env: &mut JNIEnv,
    ctx: &DASContext,
    blob: JByteArray<'local>,
    commitment: JByteArray<'local>,
    proof: JByteArray<'local>,
) -> Result<jboolean, Error> {
    let blob = env.convert_byte_array(blob)?;
    let blob = slice_to_array_ref(&blob, "blob")?;

    let commitment = env.convert_byte_array(commitment)?;
    let commitment = slice_to_array_ref(&commitment, "commitment")?;

    let proof = env.convert_byte_array(proof)?;
    let proof = slice_to_array_ref(&proof, "proof")?;

    match ctx.verify_blob_kzg_proof(blob, commitment, proof) {
        Ok(()) => Ok(jboolean::from(true)),
        Err(x) if x.is_proof_invalid() => Ok(jboolean::from(false)),
        Err(err) => Err(Error::Cryptography(err)),
    }
}

#[no_mangle]
pub extern "system" fn Java_ethereum_cryptography_LibEthKZG_verifyBlobKzgProofBatch<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    ctx_ptr: jlong,
    blobs: JObjectArray<'local>,
    commitments: JObjectArray<'local>,
    proofs: JObjectArray<'local>,
) -> jboolean {
    let ctx = unsafe { &*(ctx_ptr as *const DASContext) };

    match verify_blob_kzg_proof_batch(&mut env, ctx, &blobs, &commitments, &proofs) {
        Ok(result) => result,
        Err(err) => {
            throw_on_error(&mut env, err, "verifyBlobKzgProofBatch");
            jboolean::default()
        }
    }
}
fn verify_blob_kzg_proof_batch<'local>(
    env: &mut JNIEnv,
    ctx: &DASContext,
    blobs: &JObjectArray<'local>,
    commitments: &JObjectArray<'local>,
    proofs: &JObjectArray<'local>,
) -> Result<jboolean, Error> {
    let blobs = jobject_array_to_2d_byte_array(env, blobs)?;
    let commitments = jobject_array_to_2d_byte_array(env, commitments)?;
    let proofs = jobject_array_to_2d_byte_array(env, proofs)?;

    let blobs: Vec<_> = blobs
        .iter()
        .map(|blob| slice_to_array_ref(blob, "blob"))
        .collect::<Result<_, _>>()?;
    let commitments: Vec<_> = commitments
        .iter()
        .map(|commitment| slice_to_array_ref(commitment, "commitment"))
        .collect::<Result<_, _>>()?;
    let proofs: Vec<_> = proofs
        .iter()
        .map(|proof| slice_to_array_ref(proof, "proof"))
        .collect::<Result<_, _>>()?;

    match ctx.verify_blob_kzg_proof_batch(blobs, commitments, proofs) {
        Ok(()) => Ok(jboolean::from(true)),
        Err(x) if x.is_proof_invalid() => Ok(jboolean::from(false)),
        Err(err) => Err(Error::Cryptography(err)),
    }
}

/// Converts a JLongArray to a Vec<u64>
fn jlongarray_to_vec_u64(env: &JNIEnv, array: JLongArray) -> Result<Vec<u64>, Error> {
    // Step 1: Get the length of the JLongArray
    let array_length = env.get_array_length(&array)?;

    // Step 2: Create a buffer to hold the jlong elements (these are i64s)
    let mut buffer: Vec<i64> = vec![0; array_length as usize];

    // Step 3: Get the elements from the JLongArray
    env.get_long_array_region(array, 0, &mut buffer)?;

    // Step 4: Convert the Vec<i64> to Vec<u64>
    Ok(buffer.into_iter().map(|x| x as u64).collect())
}

/// Converts a JObjectArray to a Vec<Vec<u8>>
fn jobject_array_to_2d_byte_array(
    env: &mut JNIEnv,
    array: &JObjectArray,
) -> Result<Vec<Vec<u8>>, Error> {
    // Get the length of the outer array
    let outer_len = env.get_array_length(array)?;

    let mut result = Vec::with_capacity(outer_len as usize);

    for i in 0..outer_len {
        // Get each inner array (JByteArray)
        let inner_array_obj = env.get_object_array_element(array, i)?;
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

/// Converts a Vec<Vec<u8>> to a JObject that represents a CellsAndProofs object in Java
fn cells_and_proofs_to_jobject<'local>(
    env: &mut JNIEnv<'local>,
    cells: &[impl AsRef<[u8]>],
    proofs: &[impl AsRef<[u8]>],
) -> Result<JObject<'local>, Error> {
    // Create a new instance of the CellsAndProofs class in Java
    let cells_and_proofs_class = env.find_class("ethereum/cryptography/CellsAndProofs")?;

    let cell_byte_array_class = env.find_class("[B")?;
    let proof_byte_array_class = env.find_class("[B")?;

    // Create 2D array for cells
    let cells_array = env.new_object_array(
        cells.len() as i32,
        cell_byte_array_class,
        env.new_byte_array(0)?,
    )?;

    for (i, cell) in cells.iter().enumerate() {
        let cell_array = env.byte_array_from_slice(cell.as_ref())?;
        env.set_object_array_element(&cells_array, i as i32, cell_array)?;
    }

    // Create 2D array for proofs
    let proofs_array = env.new_object_array(
        proofs.len() as i32,
        proof_byte_array_class,
        env.new_byte_array(0)?,
    )?;

    for (i, proof) in proofs.iter().enumerate() {
        let proof_array = env.byte_array_from_slice(proof.as_ref())?;
        env.set_object_array_element(&proofs_array, i as i32, proof_array)?;
    }

    // Create the CellsAndProofs object
    let cells_and_proofs_obj = env.new_object(
        cells_and_proofs_class,
        "([[B[[B)V",
        &[JValue::Object(&cells_array), JValue::Object(&proofs_array)],
    )?;

    Ok(cells_and_proofs_obj)
}

fn cells_to_jobject<'local>(
    env: &mut JNIEnv<'local>,
    cells: &[impl AsRef<[u8]>],
) -> Result<JObject<'local>, Error> {
    // Create a new instance of the Cells class in Java
    let cells_class = env.find_class("ethereum/cryptography/Cells")?;

    let cell_byte_array_class = env.find_class("[B")?;

    // Create 2D array for cells
    let cells_array = env.new_object_array(
        cells.len() as i32,
        cell_byte_array_class,
        env.new_byte_array(0)?,
    )?;

    for (i, cell) in cells.iter().enumerate() {
        let cell_array = env.byte_array_from_slice(cell.as_ref())?;
        env.set_object_array_element(&cells_array, i as i32, cell_array)?;
    }

    // Create the Cells object
    let cells_obj = env.new_object(cells_class, "([[B)V", &[JValue::Object(&cells_array)])?;

    Ok(cells_obj)
}
/// Throws an exception in Java
fn throw_on_error(env: &mut JNIEnv, err: Error, func_name: &'static str) {
    let reason = match err {
        Error::Jni(err) => format!("{err:?}"),
        Error::IncorrectSize {
            expected,
            got,
            name,
        } => format!("{name} is not the correct size. expected: {expected}\ngot: {got}"),
        Error::Cryptography(err) => format!("{err:?}"),
    };
    let msg = format!("function {func_name} has thrown an exception, with reason: {reason}");
    env.throw_new("java/lang/IllegalArgumentException", msg)
        .expect("Failed to throw exception");
}

/// Convert a slice into a reference to an array
///
/// This is needed as the API for rust library does
/// not accept slices.
fn slice_to_array_ref<'a, const N: usize>(
    slice: &'a [u8],
    name: &'static str,
) -> Result<&'a [u8; N], Error> {
    slice.try_into().map_err(|_| Error::IncorrectSize {
        expected: N,
        got: slice.len(),
        name,
    })
}
