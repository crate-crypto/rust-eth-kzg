use crate::CellIndex;
use erasure_codes::errors::RSError;

/// Errors that can occur either during proving, verification or serialization.
#[derive(Debug)]
pub enum Error {
    Prover(ProverError),
    Verifier(VerifierError),
    Recovery(RecoveryError),
    Serialization(SerializationError),
}

impl Error {
    /// Returns true if the reason for the error was due to a proof failing verification.
    ///
    /// Note: This distinction in practice, is not meaningful for the caller and is mainly
    /// here due to the specs and spec tests making this distinction.
    pub const fn invalid_proof(&self) -> bool {
        let verifier_error = match self {
            Self::Verifier(verifier_err) => verifier_err,
            _ => return false,
        };
        matches!(verifier_error, VerifierError::FK20(_))
    }
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
impl From<RecoveryError> for Error {
    fn from(value: RecoveryError) -> Self {
        Self::Recovery(value)
    }
}

/// Errors that can occur while calling a method in the Prover API
#[derive(Debug)]
pub enum ProverError {
    RecoveryFailure(RecoveryError),
}

impl From<RecoveryError> for ProverError {
    fn from(value: RecoveryError) -> Self {
        Self::RecoveryFailure(value)
    }
}

#[derive(Debug)]
/// Errors that can occur while calling the recovery procedure
pub enum RecoveryError {
    NotEnoughCellsToReconstruct {
        num_cells_received: usize,
        min_cells_needed: usize,
    },
    NumCellIndicesNotEqualToNumCells {
        num_cell_indices: usize,
        num_cells: usize,
    },
    TooManyCellsReceived {
        num_cells_received: usize,
        max_cells_needed: usize,
    },
    CellIndexOutOfRange {
        cell_index: CellIndex,
        max_number_of_cells: u64,
    },
    CellIndicesNotUnique,
    ReedSolomon(RSError),
}

impl From<RSError> for RecoveryError {
    fn from(value: RSError) -> Self {
        Self::ReedSolomon(value)
    }
}

/// Errors that can occur while calling a method in the Verifier API
#[derive(Debug)]
pub enum VerifierError {
    CellIndexOutOfRange {
        cell_index: CellIndex,
        max_number_of_cells: u64,
    },
    InvalidCommitmentIndex {
        commitment_index: u64,
        max_number_of_commitments: u64,
    },
    InvalidProof,
    BatchVerificationInputsMustHaveSameLength {
        commitment_indices_len: usize,
        cell_indices_len: usize,
        cells_len: usize,
        proofs_len: usize,
    },
    FK20(kzg_multi_open::VerifierError),
    PolynomialHasInvalidLength {
        num_coefficients: usize,
        expected_num_coefficients: usize,
    },
}

impl From<kzg_multi_open::VerifierError> for VerifierError {
    fn from(value: kzg_multi_open::VerifierError) -> Self {
        Self::FK20(value)
    }
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
