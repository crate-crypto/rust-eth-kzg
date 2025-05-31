/// Errors that can occur during deserialization of untrusted input from the public API
/// or the trusted setup.
#[derive(Debug)]
pub enum Error {
    /// Failed to deserialize a scalar value from the given bytes.
    CouldNotDeserializeScalar {
        /// Raw bytes attempted to deserialize.
        bytes: Vec<u8>,
    },
    /// Failed to deserialize a G1 group point from the given bytes.
    CouldNotDeserializeG1Point {
        /// Raw bytes attempted to deserialize.
        bytes: Vec<u8>,
    },
    /// Scalar had an incorrect byte length.
    ScalarHasInvalidLength {
        /// Raw bytes with incorrect length.
        bytes: Vec<u8>,
        /// Detected length of the bytes.
        length: usize,
    },
    /// Blob had an incorrect byte length.
    BlobHasInvalidLength {
        /// Raw bytes with incorrect length.
        bytes: Vec<u8>,
        /// Detected length of the bytes.
        length: usize,
    },
    /// G1 point had an incorrect byte length.
    G1PointHasInvalidLength {
        /// Raw bytes with incorrect length.
        bytes: Vec<u8>,
        /// Detected length of the bytes.
        length: usize,
    },
}
