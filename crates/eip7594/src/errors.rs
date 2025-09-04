use erasure_codes::errors::RSError;
use serialization::errors::Error as SerializationError;

use crate::CellIndex;

/// Errors that can occur either during proving, verification or serialization.
#[derive(Debug)]
pub enum Error {
    /// Error that occurred during proving.
    Prover(ProverError),
    /// Error that occurred during verification.
    Verifier(VerifierError),
    /// Error that occurred during data recovery.
    Recovery(RecoveryError),
    /// Error that occurred while serializing or deserializing data.
    Serialization(SerializationError),
    /// Error that occurred from the EIP-4844 implementation.
    EIP4844(eip4844::Error),
}

impl Error {
    /// Returns true if the reason for the error was due to a proof failing verification.
    ///
    /// Note: This distinction in practice, is not meaningful for the caller and is mainly
    /// here due to the specs and spec tests making this distinction.
    pub const fn is_proof_invalid(&self) -> bool {
        matches!(
            self,
            Self::Verifier(VerifierError::FK20(_))
                | Self::EIP4844(eip4844::Error::Verifier(
                    eip4844::VerifierError::InvalidProof
                ))
        )
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

impl From<RSError> for Error {
    fn from(value: RSError) -> Self {
        Self::Recovery(RecoveryError::ReedSolomon(value))
    }
}

impl From<eip4844::Error> for Error {
    fn from(value: eip4844::Error) -> Self {
        Self::EIP4844(value)
    }
}

/// Errors that can occur while calling a method in the Prover API
#[derive(Debug)]
pub enum ProverError {
    /// Underlying recovery failure encountered during proving.
    RecoveryFailure(RecoveryError),
}

impl From<RecoveryError> for ProverError {
    fn from(value: RecoveryError) -> Self {
        Self::RecoveryFailure(value)
    }
}

/// Error type returned when data reconstruction via erasure coding fails.
#[derive(Debug)]
pub enum RecoveryError {
    /// Not enough cells were provided to reconstruct the original data.
    NotEnoughCellsToReconstruct {
        /// Number of cells that were received.
        num_cells_received: usize,
        /// Minimum number of cells required to perform reconstruction.
        min_cells_needed: usize,
    },
    /// The number of provided cell indices does not match the number of provided cells.
    NumCellIndicesNotEqualToNumCells {
        /// Number of cell indices provided.
        num_cell_indices: usize,
        /// Number of cell values provided.
        num_cells: usize,
    },
    /// Too many cells were received for reconstruction (more than required).
    TooManyCellsReceived {
        /// Number of cells received.
        num_cells_received: usize,
        /// Maximum number of cells that should be used.
        max_cells_needed: usize,
    },
    /// A provided cell index exceeded the valid range.
    CellIndexOutOfRange {
        /// Invalid cell index.
        cell_index: CellIndex,
        /// Maximum allowed number of cells.
        max_number_of_cells: u64,
    },
    /// Cell indices provided for reconstruction are not in ascending order or unique.
    CellIndicesNotUniquelyOrdered,
    /// Failure in the underlying Reed-Solomon decoding.
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
    /// A cell index was out of the valid range for the given blob.
    CellIndexOutOfRange {
        /// Invalid cell index accessed.
        cell_index: CellIndex,
        /// Maximum allowed number of cells.
        max_number_of_cells: u64,
    },
    /// A commitment index was outside the valid range.
    InvalidCommitmentIndex {
        /// The commitment index being accessed.
        commitment_index: u64,
        /// Maximum number of allowed commitments.
        max_number_of_commitments: u64,
    },
    /// Proof failed verification.
    InvalidProof,
    /// Inputs to batch verification did not have consistent lengths.
    BatchVerificationInputsMustHaveSameLength {
        /// Length of commitment indices input.
        commitment_indices_len: usize,
        /// Length of cell indices input.
        cell_indices_len: usize,
        /// Length of cell values input.
        cells_len: usize,
        /// Length of proofs input.
        proofs_len: usize,
    },
    /// Failure in FK20 batch proof verification.
    FK20(kzg_multi_open::VerifierError),
    /// The polynomial had an unexpected length.
    PolynomialHasInvalidLength {
        /// Actual number of coefficients.
        num_coefficients: usize,
        /// Expected number of coefficients based on context.
        expected_num_coefficients: usize,
    },
}

impl From<kzg_multi_open::VerifierError> for VerifierError {
    fn from(value: kzg_multi_open::VerifierError) -> Self {
        Self::FK20(value)
    }
}
