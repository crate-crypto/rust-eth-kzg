use bls12_381::{G1Point, Scalar};

use crate::{
    constants::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT},
    errors::SerializationError,
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

pub(crate) fn deserialize_bytes_to_scalar(
    scalar_bytes: &[u8],
) -> Result<Scalar, SerializationError> {
    let bytes32 = scalar_bytes.try_into().expect("infallible: expected blob chunks to be exactly {SCALAR_SERIALIZED_SIZE} bytes, since blob was a multiple of {SCALAR_SERIALIZED_SIZE");

    // Convert the CtOption into Option
    let option_scalar: Option<Scalar> = Option::from(Scalar::from_bytes_be(bytes32));
    option_scalar.ok_or_else(|| SerializationError::CouldNotDeserializeScalar {
        bytes: scalar_bytes.to_vec(),
    })
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

#[cfg(test)]
mod tests {
    use bls12_381::{
        traits::{Field, Group},
        G1Point, G1Projective, Scalar,
    };
    use rand::thread_rng;

    use super::*;
    use crate::constants::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, BYTES_PER_G1_POINT};

    /// Returns a randomly generated scalar field element.
    fn random_scalar() -> Scalar {
        Scalar::random(thread_rng())
    }

    /// Returns a random scalar serialized to big-endian bytes.
    fn scalar_bytes() -> [u8; BYTES_PER_FIELD_ELEMENT] {
        random_scalar().to_bytes_be()
    }

    /// Constructs a valid blob (BYTES_PER_BLOB long) using valid scalars.
    fn valid_blob() -> Vec<u8> {
        scalar_bytes().repeat(BYTES_PER_BLOB / BYTES_PER_FIELD_ELEMENT)
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
        let bytes = vec![1u8; BYTES_PER_FIELD_ELEMENT - 1]; // invalid length
        let _ = deserialize_bytes_to_scalar(&bytes).unwrap();
    }

    #[test]
    fn test_deserialize_scalar_invalid_encoding() {
        let mut invalid_bytes = [0xffu8; BYTES_PER_FIELD_ELEMENT];
        // Ensure it's not a valid field element
        invalid_bytes[0] |= 0b1110_0000; // exceeds modulus in high bits
        let result = deserialize_bytes_to_scalar(&invalid_bytes);
        assert!(matches!(
            result,
            Err(SerializationError::CouldNotDeserializeScalar { .. })
        ));
    }

    #[test]
    fn test_deserialize_bytes_to_scalars_valid() {
        let scalars_bytes = scalar_bytes().repeat(4);
        let scalars = deserialize_bytes_to_scalars(&scalars_bytes).unwrap();
        assert_eq!(scalars.len(), 4);
    }

    #[test]
    fn test_deserialize_bytes_to_scalars_invalid_length() {
        let invalid = vec![0u8; BYTES_PER_FIELD_ELEMENT * 3 + 1];
        assert!(matches!(
            deserialize_bytes_to_scalars(&invalid),
            Err(SerializationError::ScalarHasInvalidLength { .. })
        ));
    }

    #[test]
    fn test_deserialize_blob_to_scalars_valid() {
        let blob = valid_blob();
        let scalars = deserialize_blob_to_scalars(&blob).unwrap();
        assert_eq!(scalars.len(), BYTES_PER_BLOB / BYTES_PER_FIELD_ELEMENT);
    }

    #[test]
    fn test_deserialize_blob_to_scalars_invalid_length() {
        let bad_blob = vec![0u8; BYTES_PER_BLOB - 1];
        let result = deserialize_blob_to_scalars(&bad_blob);
        assert!(matches!(
            result,
            Err(SerializationError::BlobHasInvalidLength { .. })
        ));
    }

    #[test]
    fn test_serialize_deserialize_g1_point() {
        let point = G1Point::from(G1Projective::generator());
        let compressed = serialize_g1_compressed(&point);
        let decompressed = deserialize_compressed_g1(&compressed).unwrap();
        assert_eq!(decompressed, point);
    }

    #[test]
    fn test_deserialize_compressed_g1_invalid_length() {
        let bad_bytes = vec![0u8; BYTES_PER_G1_POINT - 1];
        let result = deserialize_compressed_g1(&bad_bytes);
        assert!(matches!(
            result,
            Err(SerializationError::G1PointHasInvalidLength { .. })
        ));
    }

    #[test]
    fn test_deserialize_compressed_g1_invalid_encoding() {
        // Construct an invalid G1 point that won't decompress properly
        let invalid = [0xffu8; BYTES_PER_G1_POINT];
        let result = deserialize_compressed_g1(&invalid);
        assert!(matches!(
            result,
            Err(SerializationError::CouldNotDeserializeG1Point { .. })
        ));
    }
}
