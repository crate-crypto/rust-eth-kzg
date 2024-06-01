use crate::constants::{
    BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT, FIELD_ELEMENTS_PER_CELL,
};
use bls12_381::{G1Point, Scalar};

#[derive(Debug)]
pub enum SerializationError {
    CouldNotDeserializeScalar { bytes: Vec<u8> },
    CouldNotDeserializeG1Point { bytes: Vec<u8> },
    ScalarHasInvalidLength { bytes: Vec<u8>, length: usize },
    BlobHasInvalidLength { bytes: Vec<u8>, length: usize },
    G1PointHasInvalidLength { bytes: Vec<u8>, length: usize },
}

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
        scalars.push(deserialize_scalar(bytes32)?)
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
pub(crate) fn deserialize_cell_to_scalars(
    cell_bytes: &[u8],
) -> Result<Vec<Scalar>, SerializationError> {
    deserialize_bytes_to_scalars(cell_bytes)
}

pub(crate) fn deserialize_scalar(scalar_bytes: &[u8]) -> Result<Scalar, SerializationError> {
    let bytes32 : [u8;BYTES_PER_FIELD_ELEMENT]= scalar_bytes.try_into().expect("infallible: expected blob chunks to be exactly {SCALAR_SERIALIZED_SIZE} bytes, since blob was a multiple of {SCALAR_SERIALIZED_SIZE");

    // convert the CtOption into Option
    let option_scalar: Option<Scalar> = Scalar::from_bytes_be(&bytes32).into();
    match option_scalar {
        Some(scalar) => Ok(scalar),
        None => {
            return Err(SerializationError::CouldNotDeserializeScalar {
                bytes: scalar_bytes.to_vec(),
            })
        }
    }
}

pub(crate) fn deserialize_compressed_g1(point_bytes: &[u8]) -> Result<G1Point, SerializationError> {
    let point_bytes: [u8; BYTES_PER_G1_POINT] = match point_bytes.try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(SerializationError::G1PointHasInvalidLength {
                length: point_bytes.len(),
                bytes: point_bytes.to_vec(),
            })
        }
    };

    let opt_g1: Option<G1Point> = Option::from(G1Point::from_compressed(&point_bytes));
    opt_g1.ok_or(SerializationError::CouldNotDeserializeG1Point {
        bytes: point_bytes.to_vec(),
    })
}
pub(crate) fn serialize_g1_compressed(point: &G1Point) -> [u8; BYTES_PER_G1_POINT] {
    point.to_compressed()
}

pub(crate) fn serialize_scalars_to_cell(scalars: &[Scalar]) -> Vec<u8> {
    assert_eq!(
        scalars.len(),
        FIELD_ELEMENTS_PER_CELL,
        "must have exactly {FIELD_ELEMENTS_PER_CELL} scalars to serialize to a cell"
    );

    let mut bytes = Vec::with_capacity(FIELD_ELEMENTS_PER_CELL * BYTES_PER_FIELD_ELEMENT);
    for scalar in scalars {
        bytes.extend_from_slice(&scalar.to_bytes_be());
    }
    bytes
}
