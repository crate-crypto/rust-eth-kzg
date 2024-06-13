// The abstractions in this file were taken and modified from: https://github.com/EspressoSystems/jellyfish/blob/8f48813ca52d964090dbf0de62f07f5e0c7e22c6/primitives/src/toeplitz.rs#L1

use bls12_381::Scalar;

#[cfg(test)]
use bls12_381::G1Projective;
#[cfg(test)]
use polynomial::domain::Domain;

#[derive(Debug, Clone)]
pub struct ToeplitzMatrix {
    row: Vec<Scalar>,
    col: Vec<Scalar>,
}

#[derive(Debug, Clone)]
pub(crate) struct CirculantMatrix {
    pub(crate) row: Vec<Scalar>,
}

impl ToeplitzMatrix {
    pub fn new(row: Vec<Scalar>, col: Vec<Scalar>) -> Self {
        assert!(
            !row.is_empty() && !col.is_empty(),
            "cannot initialize ToeplitzMatrix with empty row or col"
        );

        Self { row, col }
    }
}

impl CirculantMatrix {
    // Embeds the Toeplitz matrix into a circulant matrix, increasing the
    // dimension by two.
    pub(crate) fn from_toeplitz(tm: ToeplitzMatrix) -> CirculantMatrix {
        let mut extension_col = tm.row.clone();
        extension_col.rotate_left(1);
        extension_col.reverse();

        CirculantMatrix {
            row: [tm.col.clone(), extension_col].concat(),
        }
    }
}

#[cfg(test)]
impl CirculantMatrix {
    fn vector_mul_scalar(self, vector: Vec<Scalar>) -> Vec<Scalar> {
        let domain = Domain::new(vector.len() * 2);
        let m_fft = domain.fft_scalars(vector);
        let col_fft = domain.fft_scalars(self.row);

        let mut evaluations = Vec::new();
        for (a, b) in m_fft.into_iter().zip(col_fft) {
            evaluations.push(a * b)
        }

        domain.ifft_scalars(evaluations)
    }

    fn vector_mul_g1(self, vector: Vec<G1Projective>) -> Vec<G1Projective> {
        let domain = Domain::new(vector.len() * 2);
        let m_fft = domain.fft_g1(vector);
        let col_fft = domain.fft_scalars(self.row);

        let mut evaluations = Vec::new();
        for (a, b) in m_fft.into_iter().zip(col_fft) {
            evaluations.push(a * b)
        }
        domain.ifft_g1(evaluations)
    }
}

#[cfg(test)]
impl ToeplitzMatrix {
    fn vector_mul_scalars(self, vector: Vec<Scalar>) -> Vec<Scalar> {
        let n = vector.len();
        assert_eq!(vector.len(), self.col.len());
        let cm = CirculantMatrix::from_toeplitz(self);
        let circulant_result = cm.vector_mul_scalar(vector);

        // We take the first half of the result, as this is the result of the Toeplitz matrix multiplication
        circulant_result.into_iter().take(n).collect()
    }

    pub(crate) fn vector_mul_g1(self, vector: Vec<G1Projective>) -> Vec<G1Projective> {
        let n = vector.len();
        let cm = CirculantMatrix::from_toeplitz(self);
        let circulant_result = cm.vector_mul_g1(vector);

        // We take the first half of the result, as this is the result of the Toeplitz matrix multiplication
        circulant_result.into_iter().take(n).collect()
    }
}

#[derive(Debug)]
#[cfg(test)]
/// Dense representation of a matrix
/// This should only be used for tests
struct DenseMatrix {
    inner: Vec<Vec<Scalar>>,
}

#[cfg(test)]
impl DenseMatrix {
    /// Converts a `ToeplitzMatrix` into a `DenseMatrix`
    fn from_toeplitz(toeplitz: ToeplitzMatrix) -> DenseMatrix {
        let rows = toeplitz.col.len();
        let cols = toeplitz.row.len();
        let mut matrix = vec![vec![Scalar::from(0u64); toeplitz.col.len()]; toeplitz.row.len()];

        for i in 0..rows {
            for j in 0..cols {
                // Determine the value based on the distance from the diagonal
                if i <= j {
                    matrix[i][j] = toeplitz.row[j - i];
                } else {
                    matrix[i][j] = toeplitz.col[i - j];
                }
            }
        }

        DenseMatrix { inner: matrix }
    }

    /// Computes a matrix vector multiplication between `DenseMatrix` and `vector`
    fn vector_mul_scalar(self, vector: Vec<Scalar>) -> Vec<Scalar> {
        fn inner_product(lhs: &[Scalar], rhs: &[Scalar]) -> Scalar {
            lhs.iter().zip(rhs).map(|(a, b)| a * b).sum()
        }

        self.vector_mul(vector, inner_product)
    }
    fn vector_mul_g1(self, vector: Vec<G1Projective>) -> Vec<G1Projective> {
        use bls12_381::lincomb::g1_lincomb;
        self.vector_mul(vector, g1_lincomb)
    }

    fn vector_mul<T>(
        self,
        vector: Vec<T>,
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
            .map(|row| inner_product(&vector, &row))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::Scalar;

    use crate::fk20::toeplitz::ToeplitzMatrix;

    use super::DenseMatrix;

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
        let mut dm = DenseMatrix::from_toeplitz(tm);
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
        let dm = DenseMatrix::from_toeplitz(tm);

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
        let got = dm.vector_mul_scalar(vector);
        assert_eq!(got, expected)
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
        let dm = DenseMatrix::from_toeplitz(tm.clone());

        let vector = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
        ];
        let got = tm.vector_mul_scalars(vector.clone());
        let expected = dm.vector_mul_scalar(vector);
        assert_eq!(got, expected)
    }
}
