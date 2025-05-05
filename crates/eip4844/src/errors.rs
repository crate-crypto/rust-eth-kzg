/// Errors that can occur either during proving, verification or serialization.
#[derive(Debug)]
pub enum Error {
    Prover(ProverError),
    Verifier(VerifierError),
    Serialization(SerializationError),
}

impl From<ProverError> for Error {
    fn from(value: ProverError) -> Self {
        Self::Prover(value)
    }
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

/// Errors that can occur while calling a method in the Prover API
#[derive(Debug)]
pub enum ProverError {}

/// Errors that can occur while calling a method in the Verifier API
#[derive(Debug)]
pub enum VerifierError {
    InvalidProof,
    BatchVerificationInputsMustHaveSameLength {
        blobs_len: usize,
        commitments_len: usize,
        proofs_len: usize,
    },
}

/// Errors that can occur during deserialization of untrusted input from the public API
/// or the trusted setup.
#[derive(Debug)]
pub enum SerializationError {
    CouldNotDeserializeScalar { bytes: Vec<u8> },
    CouldNotDeserializeG1Point { bytes: Vec<u8> },
    ScalarHasInvalidLength { bytes: Vec<u8>, length: usize },
    BlobHasInvalidLength { bytes: Vec<u8>, length: usize },
    G1PointHasInvalidLength { bytes: Vec<u8>, length: usize },
}
