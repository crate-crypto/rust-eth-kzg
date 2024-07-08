#[derive(Debug)]
pub enum DecodeError {
    PolynomialHasInvalidLength {
        num_coefficients: usize,
        expected_num_coefficients: usize,
    },
}
