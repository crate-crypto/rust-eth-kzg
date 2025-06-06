pub use kzg_single_open::VerifierError;
pub use serialization::SerializationError;

/// Top-level error type for EIP-4844 verification and serialization operations.
#[derive(Debug)]
pub enum Error {
    ProverNotInitialized,
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
