use bls12_381::{G1Point, Scalar};

use crate::constants::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT};
pub use crate::errors::SerializationError;

fn deserialize_bytes_to_scalars(bytes: &[u8]) -> Result<Vec<Scalar>, SerializationError> {
    // Check that the bytes are a multiple of the scalar size
    if bytes.len() % BYTES_PER_FIELD_ELEMENT != 0 {
        return Err(SerializationError::ScalarHasInvalidLength {
            length: bytes.len(),
            bytes: bytes.to_vec(),
        });
    }

    let bytes32s = bytes.chunks_exact(BYTES_PER_FIELD_ELEMENT);

    let mut scalars = Vec::with_capacity(bytes32s.len());
    for bytes32 in bytes32s {
        scalars.push(deserialize_bytes_to_scalar(bytes32)?);
    }
    Ok(scalars)
}

pub(crate) fn deserialize_blob_to_scalars(
    blob_bytes: &[u8],
) -> Result<Vec<Scalar>, SerializationError> {
    if blob_bytes.len() != BYTES_PER_BLOB {
        return Err(SerializationError::BlobHasInvalidLength {
            length: blob_bytes.len(),
            bytes: blob_bytes.to_vec(),
        });
    }
    deserialize_bytes_to_scalars(blob_bytes)
}

pub(crate) fn deserialize_bytes_to_scalar(
    scalar_bytes: &[u8],
) -> Result<Scalar, SerializationError> {
    let bytes32 = scalar_bytes.try_into().expect("infallible: expected blob chunks to be exactly {SCALAR_SERIALIZED_SIZE} bytes, since blob was a multiple of {SCALAR_SERIALIZED_SIZE");

    // Convert the CtOption into Option
    let option_scalar: Option<Scalar> = Scalar::from_bytes_be(bytes32).into();
    option_scalar.map_or_else(
        || {
            Err(SerializationError::CouldNotDeserializeScalar {
                bytes: scalar_bytes.to_vec(),
            })
        },
        Ok,
    )
}

pub(crate) fn deserialize_compressed_g1(point_bytes: &[u8]) -> Result<G1Point, SerializationError> {
    let Ok(point_bytes) = point_bytes.try_into() else {
        return Err(SerializationError::G1PointHasInvalidLength {
            length: point_bytes.len(),
            bytes: point_bytes.to_vec(),
        });
    };

    let opt_g1: Option<G1Point> = Option::from(G1Point::from_compressed(point_bytes));
    opt_g1.ok_or_else(|| SerializationError::CouldNotDeserializeG1Point {
        bytes: point_bytes.to_vec(),
    })
}
pub(crate) fn serialize_g1_compressed(point: &G1Point) -> [u8; BYTES_PER_G1_POINT] {
    point.to_compressed()
}
