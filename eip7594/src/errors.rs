use crate::CellID;

/// Errors that can occur while calling a method in the Prover API
#[derive(Debug)]
pub enum ProverError {
    // TODO: This will be getting removed, waiting for consensus-specs PR
    NumProofsDoesNotEqualNumCells,
    Serialization(SerializationError),
    RecoveryFailure(VerifierError),
}

/// Errors that can occur while calling a method in the Verifier API
#[derive(Debug)]
pub enum VerifierError {
    Serialization(SerializationError),
    CellIDsNotEqualToNumberOfCells {
        num_cell_ids: usize,
        num_cells: usize,
    },
    CellIDsNotUnique,
    NotEnoughCellsToReconstruct {
        num_cells_received: usize,
        min_cells_needed: usize,
    },
    TooManyCellsHaveBeenGiven {
        num_cells_received: usize,
        max_cells_needed: usize,
    },
    CellDoesNotContainEnoughBytes {
        cell_id: CellID,
        num_bytes: usize,
        expected_num_bytes: usize,
    },
    CellIDOutOfRange {
        cell_id: CellID,
        max_number_of_cells: u64,
    },
    InvalidRowID {
        row_id: u64,
        max_number_of_rows: u64,
    },
    InvalidProof,
    BatchVerificationInputsMustHaveSameLength {
        row_indices_len: usize,
        column_indices_len: usize,
        cells_len: usize,
        proofs_len: usize,
    },
    PolynomialHasInvalidLength {
        num_coefficients: usize,
        expected_num_coefficients: usize,
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
