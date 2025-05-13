// The abstractions in this file were taken and modified from: https://github.com/EspressoSystems/jellyfish/blob/8f48813ca52d964090dbf0de62f07f5e0c7e22c6/primitives/src/toeplitz.rs#L1

use bls12_381::Scalar;

/// A Toeplitz matrix is a matrix in which each descending diagonal from left to right is constant.
/// "Constant" here means that all elements along any given diagonal have the same value.
///
/// For example, in the matrix:
/// ```text
/// [a b c d]
/// [e a b c]
/// [f e a b]
/// [g f e a]
/// ```
/// The main diagonal (top-left to bottom-right) is constant with value 'a'.
/// The diagonal above it is constant with value 'b', the one above that with 'c', and so on.
/// Similarly, the diagonal below the main one is constant with value 'e', the next with 'f', etc.
///
/// # Efficient Representation
///
/// Due to the constant diagonal property, a Toeplitz matrix is fully determined by its first row
/// and first column. This allows for an efficient representation using only these two vectors,
/// significantly reducing memory usage for large matrices.
///
/// - The first row contains all the elements that appear on or above the main diagonal.
/// - The first column (excluding the first element) contains all the elements below the main diagonal.
///
/// This structure leverages this property to store the entire matrix using only these two vectors.
///
/// # Examples
///
/// ```text
///
/// row = [1, 2, 3, 4];
/// col = [1, 5, 6, 7];
///
///  This efficiently represents the following 4x4 Toeplitz matrix:
///  [1 2 3 4]
///  [5 1 2 3]
///  [6 5 1 2]
///  [7 6 5 1]
/// ```
///
/// In this example, we only store 8 elements (4 in `row` and 4 in `col`) to represent
/// a 4x4 matrix that would normally require 16 elements.
#[derive(Debug, Clone)]
pub struct ToeplitzMatrix {
    /// A vector representing the first row of the matrix.
    row: Vec<Scalar>,
    ///  A vector representing the first column of the matrix, including the first element
    ///  (even though the first element is already included in the `row`).
    col: Vec<Scalar>,
}

impl ToeplitzMatrix {
    /// Constructs a Toeplitz matrix from its first row and column.
    ///
    /// A Toeplitz matrix is fully determined by its first row and column:
    /// - `row[0]` must equal `col[0]` (the shared top-left entry),
    /// - `row` defines the elements on and above the main diagonal,
    /// - `col` defines the elements below the main diagonal.
    ///
    /// # Panics
    /// Panics if either `row` or `col` is empty, or if `row[0] != col[0]`.
    pub fn new(row: Vec<Scalar>, col: Vec<Scalar>) -> Self {
        assert!(
            !row.is_empty() && !col.is_empty(),
            "row and col must be non-empty"
        );
        assert_eq!(
            row[0], col[0],
            "Toeplitz matrix must satisfy row[0] == col[0] (shared top-left entry)"
        );
        Self { row, col }
    }
}

/// A circulant matrix is a special kind of Toeplitz matrix where each row is rotated one
/// element to the right relative to the preceding row. This structure allows for an even
/// more efficient representation than a general Toeplitz matrix.
///
/// # Efficient Representation
///
/// Due to the circulant property, the entire matrix is fully determined by its first row alone.
/// Each subsequent row is a cyclic shift of the first row. This allows for an extremely
/// memory-efficient representation, storing only a single vector for the entire matrix.
///
/// For an n × n circulant matrix, we only need to store n elements instead of n^2.
///
/// # Example
///
/// Given the first row [a, b, c, d], the full 4x4 circulant matrix would be:
/// ```text
/// [a b c d]
/// [d a b c]
/// [c d a b]
/// [b c d a]
/// ```
///
/// # Examples
///
/// ```text
/// use ekzg_multi_open::fk20::toeplitz::CirculantMatrix;
///
/// row = [1, 2, 3, 4]
///
/// This efficiently represents the following 4x4 circulant matrix:
///  [1 2 3 4]
///  [4 1 2 3]
///  [3 4 1 2]
///  [2 3 4 1]
/// ```
///
/// In this example, we only store 4 elements to represent a 4x4 matrix that would
/// normally require 16 elements.
///
/// # Properties
///
/// The main property of Circulant matrices that we leverage is that they are diagonalized by the Fourier
/// transform, which allows for efficient computations.
#[derive(Debug, Clone)]
pub(crate) struct CirculantMatrix {
    /// A vector representing the first row of the matrix. This single row defines
    /// the entire circulant matrix.
    pub(crate) row: Vec<Scalar>,
}

impl CirculantMatrix {
    /// This method takes a Toeplitz matrix and embeds it into a larger circulant matrix.
    /// The resulting circulant matrix has a dimension that is twice as large as the original
    /// Toeplitz matrix.
    pub(crate) fn from_toeplitz(tm: ToeplitzMatrix) -> Self {
        let mut extension_col = tm.row;
        extension_col.rotate_left(1);
        extension_col.reverse();

        Self {
            row: [tm.col, extension_col].concat(),
        }
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::{traits::*, G1Projective};

    use super::*;

    impl ToeplitzMatrix {
        fn vector_mul_scalars(self, vector: Vec<Scalar>) -> Vec<Scalar> {
            let n = vector.len();
            assert_eq!(vector.len(), self.col.len());
            let cm = CirculantMatrix::from_toeplitz(self);
            let circulant_result = cm.vector_mul_scalar(vector);

            // We take the first half of the result, as this is the result of the Toeplitz matrix multiplication
            circulant_result.0.into_iter().take(n).collect()
        }

        pub(crate) fn vector_mul_g1(self, vector: Vec<G1Projective>) -> Vec<G1Projective> {
            let n = vector.len();
            let cm = CirculantMatrix::from_toeplitz(self);
            let circulant_result = cm.vector_mul_g1(vector);

            // We take the first half of the result, as this is the result of the Toeplitz matrix multiplication
            circulant_result.into_iter().take(n).collect()
        }
    }

    impl CirculantMatrix {
        /// This method performs an efficient multiplication of the circulant matrix
        /// with a vector of scalars using FFT.
        ///
        /// See https://www.johndcook.com/blog/2023/05/12/circulant-matrices/ for more details.
        fn vector_mul_scalar(self, vector: Vec<Scalar>) -> polynomial::poly_coeff::PolyCoeff {
            let domain = polynomial::domain::Domain::new(vector.len() * 2);
            let m_fft = domain.fft_scalars(vector.into());
            let col_fft = domain.fft_scalars(self.row.into());

            let mut evaluations = Vec::new();
            for (a, b) in m_fft.into_iter().zip(col_fft) {
                evaluations.push(a * b);
            }

            domain.ifft_scalars(evaluations)
        }

        /// This method performs an efficient multiplication of the circulant matrix
        /// with a vector of G1 points using FFT.
        ///
        /// See https://www.johndcook.com/blog/2023/05/12/circulant-matrices/ for more details.
        fn vector_mul_g1(self, vector: Vec<G1Projective>) -> Vec<G1Projective> {
            assert!(vector.len().is_power_of_two());

            // Compute the circulant domain
            let domain = polynomial::domain::Domain::new(vector.len() * 2);
            // Compute the fft of the vector
            let m_fft = domain.fft_g1(vector);
            let col_fft = domain.fft_scalars(self.row.into());

            let mut evaluations = Vec::new();
            for (a, b) in m_fft.into_iter().zip(col_fft) {
                evaluations.push(a * b);
            }
            domain.ifft_g1(evaluations)
        }
    }

    /// This structure stores a matrix as a vector of vectors, where each inner
    /// vector represents a row of the matrix.
    ///
    /// This should should only be used for tests.
    #[derive(Debug, Clone)]
    struct DenseMatrix {
        inner: Vec<Vec<Scalar>>,
    }

    impl DenseMatrix {
        /// Converts a `ToeplitzMatrix` into a `DenseMatrix`
        fn from_toeplitz(toeplitz: &ToeplitzMatrix) -> Self {
            let rows = toeplitz.col.len();
            let cols = toeplitz.row.len();
            let mut matrix = vec![vec![Scalar::ZERO; toeplitz.col.len()]; toeplitz.row.len()];

            for (i, r) in matrix.iter_mut().enumerate().take(rows) {
                for (j, rc) in r.iter_mut().enumerate().take(cols) {
                    // Determine the value based on the distance from the diagonal
                    if i <= j {
                        *rc = toeplitz.row[j - i];
                    } else {
                        *rc = toeplitz.col[i - j];
                    }
                }
            }

            Self { inner: matrix }
        }

        /// Computes a matrix vector multiplication between `DenseMatrix` and `vector`
        fn vector_mul_scalar(self, vector: &[Scalar]) -> Vec<Scalar> {
            fn inner_product(lhs: &[Scalar], rhs: &[Scalar]) -> Scalar {
                lhs.iter().zip(rhs).map(|(a, b)| a * b).sum()
            }

            self.vector_mul(vector, inner_product)
        }
        /// Performs a generalized matrix-vector multiplication.
        ///
        /// This method allows for matrix-vector multiplication with custom types and
        /// inner product operations.
        fn vector_mul<T>(
            self,
            vector: &[T],
            inner_product: fn(lhs: &[T], rhs: &[Scalar]) -> T,
        ) -> Vec<T> {
            let row_length = self.inner[0].len();
            assert_eq!(
                row_length,
                vector.len(),
                "Matrix row and vector length do not match, matrix: {}, vector: {}",
                row_length,
                vector.len()
            );

            self.inner
                .into_iter()
                .map(|row| inner_product(vector, &row))
                .collect()
        }
    }

    fn is_toeplitz(dense_matrix: &DenseMatrix) -> bool {
        let num_rows = dense_matrix.inner.len();
        if num_rows == 0 || dense_matrix.inner[0].is_empty() {
            return false;
        }

        let num_cols = dense_matrix.inner[0].len();
        for i in 0..num_rows - 1 {
            for j in 0..num_cols - 1 {
                if dense_matrix.inner[i][j] != dense_matrix.inner[i + 1][j + 1] {
                    return false;
                }
            }
        }

        true
    }

    #[test]
    fn smoke_test_check_dense_toeplitz_matrix_construction() {
        let col = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let row = vec![Scalar::from(1u64), Scalar::from(5u64), Scalar::from(6u64)];

        /*
        [1, 5, 6]
        [2, 1, 5]
        [3, 2, 1]
        */
        let tm = ToeplitzMatrix::new(col, row);
        let mut dm = DenseMatrix::from_toeplitz(&tm);
        assert!(
            is_toeplitz(&dm),
            "dense matrix should represent a toeplitz matrix"
        );

        /*
        [1, 5, 6]
        [2, 1, 5]
        [3, 2, 1]
        */
        dm.inner[0][0] = Scalar::from(2u64);
        assert!(
            !is_toeplitz(&dm),
            "dense matrix should not represent a toeplitz matrix"
        );
    }

    #[test]
    fn smoke_test_dense_matrix_vector_mul() {
        let col = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let row = vec![Scalar::from(1u64), Scalar::from(5u64), Scalar::from(6u64)];

        let tm = ToeplitzMatrix::new(row, col);
        let dm = DenseMatrix::from_toeplitz(&tm);

        let vector = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        /*
        [1, 5, 6][1]   [29]
        [2, 1, 5][2] = [19]
        [3, 2, 1][3]   [10]
        */
        let expected = vec![
            Scalar::from(29u64),
            Scalar::from(19u64),
            Scalar::from(10u64),
        ];
        let got = dm.vector_mul_scalar(&vector);
        assert_eq!(got, expected);
    }

    #[test]
    fn smoke_test_circulant_matrix() {
        let col = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
        ];
        let row = vec![
            Scalar::from(1u64),
            Scalar::from(5u64),
            Scalar::from(6u64),
            Scalar::from(7u64),
        ];

        let tm = ToeplitzMatrix::new(col, row);
        let dm = DenseMatrix::from_toeplitz(&tm);

        let vector = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
        ];
        let got = tm.vector_mul_scalars(vector.clone());
        let expected = dm.vector_mul_scalar(&vector);
        assert_eq!(got, expected);
    }

    #[test]
    #[should_panic]
    fn toeplitz_matrix_panics_on_mismatched_top_left() {
        let row = vec![Scalar::from(1u64), Scalar::from(2u64)];
        let col = vec![Scalar::from(9u64), Scalar::from(3u64)]; // col[0] != row[0]
        let _ = ToeplitzMatrix::new(row, col);
    }
}
