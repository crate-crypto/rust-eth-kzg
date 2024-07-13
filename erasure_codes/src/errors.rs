#[derive(Debug)]
pub enum RSError {
    PolynomialHasTooManyCoefficients {
        num_coefficients: usize,
        max_num_coefficients: usize,
    },
    PolynomialHasInvalidLength {
        num_coefficients: usize,
        expected_num_coefficients: usize,
    },
    TooManyBlockErasures {
        num_block_erasures: usize,
        max_num_block_erasures_accepted: usize,
    },
    InvalidBlockIndex {
        block_index: usize,
        block_size: usize,
    },
}
