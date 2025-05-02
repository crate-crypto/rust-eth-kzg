/// Errors that can occur during Reed-Solomon encoding or erasure recovery.
#[derive(Debug)]
pub enum RSError {
    /// Raised when the input polynomial exceeds the allowed maximum number of coefficients.
    ///
    /// This occurs if `poly.len() > poly_len` in the `encode` function.
    PolynomialHasTooManyCoefficients {
        /// Number of coefficients in the provided polynomial.
        num_coefficients: usize,
        /// Maximum allowed number of coefficients (i.e., `poly_len`).
        max_num_coefficients: usize,
    },

    /// Raised when the recovered polynomial has a degree greater than expected.
    ///
    /// This typically signals an invalid recovery due to incorrect erasure input or domain mismatch.
    PolynomialHasInvalidLength {
        /// Total number of coefficients returned.
        num_coefficients: usize,
        /// Expected number of coefficients (`poly_len`).
        expected_num_coefficients: usize,
    },

    /// Raised when the number of block-synchronized erasures exceeds the correction capacity.
    ///
    /// This is checked during block-erasure decoding.
    TooManyBlockErasures {
        /// Number of block indices requested for erasure.
        num_block_erasures: usize,
        /// Maximum number of block erasures that can be corrected.
        max_num_block_erasures_accepted: usize,
    },

    /// Raised when an invalid block index is supplied (i.e., out of range).
    ///
    /// Block indices must be less than `block_size`.
    InvalidBlockIndex {
        /// The offending block index.
        block_index: usize,
        /// The size of each block, used as the upper bound.
        block_size: usize,
    },
}
