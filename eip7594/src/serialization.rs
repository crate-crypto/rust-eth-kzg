use crate::{
    constants::{
        BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT, CELLS_PER_EXT_BLOB,
        FIELD_ELEMENTS_PER_CELL,
    },
    Cell, KZGProof,
};
use bls12_381::{G1Point, Scalar};

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
        None => Err(SerializationError::CouldNotDeserializeScalar {
            bytes: scalar_bytes.to_vec(),
        }),
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

/// Converts a a set of scalars (evaluations) to the `Cell` type
pub(crate) fn evaluation_sets_to_cells<T: AsRef<[Scalar]>>(
    evaluations: impl Iterator<Item = T>,
) -> [Cell; CELLS_PER_EXT_BLOB] {
    let cells: Vec<Cell> = evaluations
        .map(|eval| serialize_scalars_to_cell(eval.as_ref()))
        .map(|cell| {
            cell.into_boxed_slice()
                .try_into()
                .expect("infallible: Vec<u8> should have length equal to BYTES_PER_CELL")
        })
        .collect();

    cells
        .try_into()
        .unwrap_or_else(|_| panic!("expected {} number of cells", CELLS_PER_EXT_BLOB))
}

pub(crate) fn serialize_cells_and_proofs(
    evaluation_sets: Vec<Vec<Scalar>>,
    proofs: Vec<G1Point>,
) -> ([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]) {
    // Serialize the evaluation sets into `Cell`s.
    let cells = evaluation_sets_to_cells(evaluation_sets.into_iter());

    // Serialize the proofs into `KZGProof` objects.
    let proofs: Vec<_> = proofs.iter().map(serialize_g1_compressed).collect();
    let proofs: [KZGProof; CELLS_PER_EXT_BLOB] = proofs
        .try_into()
        .unwrap_or_else(|_| panic!("expected {} number of proofs", CELLS_PER_EXT_BLOB));

    (cells, proofs)
}
