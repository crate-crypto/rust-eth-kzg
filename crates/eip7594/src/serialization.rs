use bls12_381::{G1Point, Scalar};

pub use crate::errors::SerializationError;
use crate::{
    constants::{
        BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT,
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_CELL,
    },
    Cell, KZGProof,
};

/// Deserializes a byte slice into a vector of `Scalar`s.
///
/// The input must be a multiple of the scalar size (32 bytes).
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

/// Deserializes a blob into a vector of `Scalar`s.
///
/// The blob must be exactly `BYTES_PER_BLOB` long (4096 field elements).
/// Returns an error if the length is incorrect or parsing fails.
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

/// Deserializes a 32-byte slice into a single `Scalar`.
///
/// This expects the input to be exactly 32 bytes.
/// Fails if the bytes do not correspond to a valid field element.
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

/// Converts a compressed G1 point (48 bytes) to a `G1Point`.
///
/// Returns an error if the length is incorrect or the bytes are invalid.
/// Wraps the `from_compressed` function from the BLS12-381 crate.
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

/// Serializes a G1 point into its compressed representation.
pub(crate) fn serialize_g1_compressed(point: &G1Point) -> [u8; BYTES_PER_G1_POINT] {
    point.to_compressed()
}

/// Deserializes a list of compressed G1 point byte slices.
///
/// Returns a vector of `G1Point`s or fails on the first invalid point.
/// Each input slice must be exactly 48 bytes.
pub(crate) fn deserialize_compressed_g1_points(
    points: Vec<&[u8; BYTES_PER_G1_POINT]>,
) -> Result<Vec<G1Point>, SerializationError> {
    points
        .into_iter()
        .map(|point| deserialize_compressed_g1(point))
        .collect()
}

/// Serializes a slice of `Scalar`s into a byte vector representing a cell.
///
/// The input must be exactly `FIELD_ELEMENTS_PER_CELL` elements long.
/// Produces a flat byte array suitable for storage or transmission.
pub(crate) fn serialize_scalars_to_cell(scalars: &[Scalar]) -> Vec<u8> {
    assert_eq!(
        scalars.len(),
        FIELD_ELEMENTS_PER_CELL,
        "must have exactly {FIELD_ELEMENTS_PER_CELL} scalars to serialize to a cell"
    );

    scalars.iter().flat_map(Scalar::to_bytes_be).collect()
}

/// Deserializes a vector of cell byte slices into vectors of `Scalar`s.
///
/// Each cell must be `BYTES_PER_CELL` bytes long.
/// Returns an error if parsing any cell fails.
pub(crate) fn deserialize_cells(
    cells: Vec<&[u8; BYTES_PER_CELL]>,
) -> Result<Vec<Vec<Scalar>>, SerializationError> {
    cells
        .into_iter()
        .map(|c| deserialize_bytes_to_scalars(c))
        .collect()
}

/// Serializes both cells and corresponding proofs into flat output formats.
///
/// Converts evaluation sets to `Cell`s and G1 points to `KZGProof`s.
/// Expects exactly `CELLS_PER_EXT_BLOB` items in both inputs.
pub(crate) fn serialize_cells_and_proofs(
    coset_evaluations: &[Vec<Scalar>],
    proofs: &[G1Point],
) -> ([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]) {
    (
        serialize_cells(coset_evaluations),
        std::array::from_fn(|i| proofs[i].to_compressed()),
    )
}

/// Serializes a list of evaluation sets into an array of `Cell`s.
///
/// Each set must contain exactly `FIELD_ELEMENTS_PER_CELL` scalars.
/// Returns a fixed-size array with length `CELLS_PER_EXT_BLOB`.
pub(crate) fn serialize_cells(coset_evaluations: &[Vec<Scalar>]) -> [Cell; CELLS_PER_EXT_BLOB] {
    // Serialize the evaluation sets into `Cell`s.
    std::array::from_fn(|i| {
        let evals = &coset_evaluations[i];
        let bytes = serialize_scalars_to_cell(evals);
        bytes
            .into_boxed_slice()
            .try_into()
            .expect("infallible: serialized cell must be BYTES_PER_CELL long")
    })
}

#[cfg(test)]
mod tests {
    use bls12_381::{traits::*, G1Point, G1Projective, Scalar};
    use rand::thread_rng;

    use super::*;
    use crate::constants::FIELD_ELEMENTS_PER_BLOB;

    /// Returns a randomly generated scalar field element.
    fn random_scalar() -> Scalar {
        Scalar::random(thread_rng())
    }

    /// Returns a random scalar serialized to `BYTES_PER_FIELD_ELEMENT` big-endian bytes.
    fn scalar_bytes() -> [u8; BYTES_PER_FIELD_ELEMENT] {
        random_scalar().to_bytes_be()
    }

    /// Constructs a valid blob by repeating a random scalar `FIELD_ELEMENTS_PER_BLOB` times.
    fn valid_blob() -> Vec<u8> {
        scalar_bytes().repeat(FIELD_ELEMENTS_PER_BLOB)
    }

    /// Constructs a valid cell by repeating a random scalar FIELD_ELEMENTS_PER_CELL times.
    fn valid_cell() -> [u8; BYTES_PER_CELL] {
        scalar_bytes()
            .repeat(FIELD_ELEMENTS_PER_CELL)
            .try_into()
            .unwrap()
    }

    #[test]
    fn test_deserialize_scalar_valid() {
        let bytes = scalar_bytes();
        let scalar = deserialize_bytes_to_scalar(&bytes).unwrap();
        assert_eq!(scalar.to_bytes_be(), bytes);
    }

    #[test]
    #[should_panic]
    fn test_deserialize_scalar_invalid_length() {
        let bytes = vec![1u8; 31]; // invalid
        let _ = deserialize_bytes_to_scalar(&bytes);
    }

    #[test]
    fn test_deserialize_blob_to_scalars_valid() {
        let blob = valid_blob();
        let scalars = deserialize_blob_to_scalars(&blob).unwrap();
        assert_eq!(scalars.len(), FIELD_ELEMENTS_PER_BLOB);
    }

    #[test]
    fn test_deserialize_blob_to_scalars_invalid_length() {
        let blob = vec![0u8; BYTES_PER_BLOB - 1];
        assert!(matches!(
            deserialize_blob_to_scalars(&blob),
            Err(SerializationError::BlobHasInvalidLength { .. })
        ));
    }

    #[test]
    fn test_deserialize_bytes_to_scalars_valid() {
        let cell = valid_cell();
        let scalars = deserialize_bytes_to_scalars(&cell).unwrap();
        assert_eq!(scalars.len(), FIELD_ELEMENTS_PER_CELL);
    }

    #[test]
    fn test_serialize_scalars_to_cell_and_back() {
        let scalars: Vec<_> = (0..FIELD_ELEMENTS_PER_CELL)
            .map(|_| random_scalar())
            .collect();
        let cell_bytes = serialize_scalars_to_cell(&scalars);
        let scalars_back = deserialize_bytes_to_scalars(&cell_bytes).unwrap();
        assert_eq!(scalars, scalars_back);
    }

    #[test]
    fn test_serialize_deserialize_g1_point() {
        let point = G1Point::from(G1Projective::generator());
        let compressed = point.to_compressed();
        let decompressed = deserialize_compressed_g1(&compressed).unwrap();
        assert_eq!(G1Point::from(decompressed), point);
    }

    #[test]
    fn test_deserialize_compressed_g1_invalid_length() {
        let bad_bytes = vec![0u8; 47];
        assert!(matches!(
            deserialize_compressed_g1(&bad_bytes),
            Err(SerializationError::G1PointHasInvalidLength { .. })
        ));
    }

    #[test]
    fn test_coset_evaluations_to_cells() {
        let evaluations: Vec<_> = (0..CELLS_PER_EXT_BLOB)
            .map(|_| {
                (0..FIELD_ELEMENTS_PER_CELL)
                    .map(|_| random_scalar())
                    .collect::<Vec<_>>()
            })
            .collect();
        let cells = serialize_cells(&evaluations);
        assert_eq!(cells.len(), CELLS_PER_EXT_BLOB);
        for cell in &cells {
            assert_eq!(cell.len(), BYTES_PER_CELL);
        }
    }

    #[test]
    fn test_serialize_cells_and_proofs() {
        let evaluations: Vec<_> = (0..CELLS_PER_EXT_BLOB)
            .map(|_| {
                (0..FIELD_ELEMENTS_PER_CELL)
                    .map(|_| random_scalar())
                    .collect::<Vec<_>>()
            })
            .collect();
        let proofs: Vec<_> = (0..CELLS_PER_EXT_BLOB)
            .map(|_| G1Point::from(G1Projective::generator()))
            .collect();

        let (cells, proofs) = serialize_cells_and_proofs(&evaluations, &proofs);
        assert_eq!(cells.len(), CELLS_PER_EXT_BLOB);
        assert_eq!(proofs.len(), CELLS_PER_EXT_BLOB);
    }
}
