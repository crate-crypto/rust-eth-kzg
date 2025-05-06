use bls12_381::{G1Point, Scalar};

pub use crate::errors::SerializationError;
use crate::{
    constants::{
        BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT,
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_CELL,
    },
    Cell, KZGProof,
};

fn deserialize_bytes_to_scalars(bytes: &[u8]) -> Result<Vec<Scalar>, SerializationError> {
    // Check that the bytes are a multiple of the scalar size
    if bytes.len() % BYTES_PER_FIELD_ELEMENT != 0 {
        return Err(SerializationError::ScalarHasInvalidLength {
            length: bytes.len(),
            bytes: bytes.to_vec(),
        });
    }

    bytes
        .chunks_exact(BYTES_PER_FIELD_ELEMENT)
        .map(deserialize_bytes_to_scalar)
        .collect()
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

pub(crate) fn deserialize_compressed_g1_points(
    points: Vec<&[u8; BYTES_PER_G1_POINT]>,
) -> Result<Vec<G1Point>, SerializationError> {
    points
        .into_iter()
        .map(|point| deserialize_compressed_g1(point))
        .collect()
}

pub(crate) fn serialize_scalars_to_cell(scalars: &[Scalar]) -> Vec<u8> {
    assert_eq!(
        scalars.len(),
        FIELD_ELEMENTS_PER_CELL,
        "must have exactly {FIELD_ELEMENTS_PER_CELL} scalars to serialize to a cell"
    );

    scalars.iter().flat_map(Scalar::to_bytes_be).collect()
}

pub(crate) fn deserialize_cells(
    cells: Vec<&[u8; BYTES_PER_CELL]>,
) -> Result<Vec<Vec<Scalar>>, SerializationError> {
    cells
        .into_iter()
        .map(|c| deserialize_cell_to_scalars(c))
        .collect()
}

/// Converts a set of scalars (evaluations) to the `Cell` type.
pub(crate) fn coset_evaluations_to_cells<T: AsRef<[Scalar]>>(
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
        .unwrap_or_else(|_| panic!("expected {CELLS_PER_EXT_BLOB} number of cells"))
}

pub(crate) fn serialize_cells_and_proofs(
    coset_evaluations: Vec<Vec<Scalar>>,
    proofs: &[G1Point],
) -> ([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]) {
    // Serialize the evaluation sets into `Cell`s.
    let cells = serialize_cells(coset_evaluations);

    // Serialize the proofs into `KZGProof` objects.
    let proofs: Vec<_> = proofs.iter().map(serialize_g1_compressed).collect();
    let proofs = proofs
        .try_into()
        .unwrap_or_else(|_| panic!("expected {CELLS_PER_EXT_BLOB} number of proofs"));

    (cells, proofs)
}

pub(crate) fn serialize_cells(coset_evaluations: Vec<Vec<Scalar>>) -> [Cell; CELLS_PER_EXT_BLOB] {
    // Serialize the evaluation sets into `Cell`s.
    coset_evaluations_to_cells(coset_evaluations.into_iter())
}
