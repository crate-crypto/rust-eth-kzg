/// Errors that can occur when verifying a blob proof using the Verifier API.
#[derive(Debug)]
pub enum VerifierError {
    /// The proof failed verification.
    InvalidProof,
    /// Inputs to batch verification did not have consistent lengths.
    BatchVerificationInputsMustHaveSameLength {
        /// Number of blobs provided as input.
        blobs_len: usize,
        /// Number of corresponding KZG commitments.
        commitments_len: usize,
        /// Number of provided KZG proofs.
        proofs_len: usize,
    },
}
