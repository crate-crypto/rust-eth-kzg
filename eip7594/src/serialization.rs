use crate::constants::{BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT};
use bls12_381::{G1Projective, Scalar};

pub(crate) fn hex_to_bytes(hex_str: &str) -> Vec<u8> {
    hex::decode(hex_str).expect("malformed hex string")
}

pub(crate) fn deserialize_blob_to_scalars(blob_bytes: &[u8]) -> Vec<Scalar> {
    assert!(
        blob_bytes.len() % BYTES_PER_FIELD_ELEMENT == 0,
        "expected bytes to be a multiple of {BYTES_PER_FIELD_ELEMENT}",
    );

    let bytes32s = blob_bytes.chunks_exact(BYTES_PER_FIELD_ELEMENT);

    let mut scalars = Vec::with_capacity(bytes32s.len());
    for bytes32 in bytes32s {
        scalars.push(deserialize_scalar(bytes32))
    }
    scalars
}

pub(crate) fn deserialize_scalar(scalar_bytes: &[u8]) -> Scalar {
    let bytes32 : [u8;BYTES_PER_FIELD_ELEMENT]= scalar_bytes.try_into().expect("infallible: expected blob chunks to be exactly {SCALAR_SERIALIZED_SIZE} bytes, since blob was a multiple of {SCALAR_SERIALIZED_SIZE");
    // convert the CtOption into Option
    let option_scalar: Option<Scalar> = Scalar::from_bytes_be(&bytes32).into();
    option_scalar.expect("Result: blob chunk is not a valid scalar")
}

pub(crate) fn deserialize_compressed_g1(point_bytes: &[u8]) -> G1Projective {
    let point_bytes: [u8; BYTES_PER_G1_POINT] =
        point_bytes.try_into().expect("point should be 48 bytes");
    G1Projective::from_compressed(&point_bytes).expect("cannot deserialize point")
}
