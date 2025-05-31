pub use kzg_single_open::VerifierError;

/// Top-level error type for EIP-4844 verification and serialization operations.
#[derive(Debug)]
pub enum Error {
    /// Error encountered during verification of a blob proof.
    Verifier(VerifierError),
    /// Error encountered while (de)serializing blobs, scalars, or group elements.
    Serialization(SerializationError),
}

impl From<VerifierError> for Error {
    fn from(value: VerifierError) -> Self {
        Self::Verifier(value)
    }
}

impl From<SerializationError> for Error {
    fn from(value: SerializationError) -> Self {
        Self::Serialization(value)
    }
}

/// Errors that can occur during deserialization of input data, either from
/// the public interface (e.g. blobs, commitments, proofs) or the trusted setup.
#[derive(Debug)]
pub enum SerializationError {
    /// Failed to deserialize a scalar from the input byte sequence.
    CouldNotDeserializeScalar {
        /// Raw bytes that failed to deserialize.
        bytes: Vec<u8>,
    },

    /// Failed to deserialize a G1 group element from the input byte sequence.
    CouldNotDeserializeG1Point {
        /// Raw bytes that failed to deserialize.
        bytes: Vec<u8>,
    },

    /// Input byte slice used to deserialize a scalar had the wrong length.
    ScalarHasInvalidLength {
        /// Raw bytes with invalid length.
        bytes: Vec<u8>,
        /// Actual length of the byte slice.
        length: usize,
    },

    /// Input byte slice used to deserialize a blob had the wrong length.
    BlobHasInvalidLength {
        /// Raw bytes with invalid length.
        bytes: Vec<u8>,
        /// Actual length of the byte slice.
        length: usize,
    },

    /// Input byte slice used to deserialize a G1 point had the wrong length.
    G1PointHasInvalidLength {
        /// Raw bytes with invalid length.
        bytes: Vec<u8>,
        /// Actual length of the byte slice.
        length: usize,
    },
}
