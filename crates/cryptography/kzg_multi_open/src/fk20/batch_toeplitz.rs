use bls12_381::{
    fixed_base_msm::{FixedBaseMSM, UsePrecomp},
    g1_batch_normalize, G1Point, G1Projective,
};
use maybe_rayon::prelude::*;
use polynomial::domain::Domain;

use crate::fk20::toeplitz::{CirculantMatrix, ToeplitzMatrix};

/// BatchToeplitzMatrixVecMul allows one to compute multiple matrix vector multiplications
/// and sum them together.
/// It is optimized for the usecase where:
///  - The matrices being used are Toeplitz matrices (hence we can use FFTs to compute the matrix-vector multiplication)
///  - The vectors being used are fixed, so we can precompute their FFTs.
///  - The vectors being used as group elements (curve points)
#[derive(Debug)]
pub struct BatchToeplitzMatrixVecMul {
    /// This contains the number of matrix-vector multiplications that
    /// we can do in a batch.
    batch_size: usize,
    /// Precomputed FFTs of the fixed input vectors, stored as fixed-base MSM tables.
    /// These allow efficient scalar multiplications during the matrix-vector product phase.
    precomputed_fft_vectors: Vec<FixedBaseMSM>,
    /// This is the length of the vector that we are multiplying the matrices with.
    /// and subsequently will be the length of the final result of the matrix-vector multiplication.
    size_of_vector: usize,
    /// This is the domain used in the circulant matrix-vector multiplication.
    /// It will be double the size of the length of a pre-computed vector.
    circulant_domain: Domain,
}

impl BatchToeplitzMatrixVecMul {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(vectors: Vec<Vec<G1Point>>, use_precomp: UsePrecomp) -> Self {
        let size_of_vector = vectors[0].len();
        let vectors_all_same_length = vectors.iter().all(|v| v.len() == size_of_vector);
        assert!(
            vectors_all_same_length,
            "expected all vectors to be the same length"
        );
        assert!(
            size_of_vector.is_power_of_two(),
            "expected the size of the vector to be a power of two"
        );

        let circulant_domain = Domain::new(size_of_vector * 2);

        // Precompute the FFT of the vectors, since they do not change per matrix-vector multiplication
        let vectors: Vec<_> = vectors
            .maybe_par_iter()
            .map(|vector| {
                let vector_projective = vector
                    .iter()
                    .map(|point| G1Projective::from(*point))
                    .collect();
                g1_batch_normalize(&circulant_domain.fft_g1(vector_projective))
            })
            .collect();
        let batch_size = vectors.len();

        let transposed_msm_vectors = transpose(vectors);

        // Configurable parameter to denote the amount of pre-computation one should do
        // for the fixed base multi-scalar multiplication.
        //
        // This is a trade-off between storage and computation, where storage grows exponentially.
        let precomputed_fft_vectors = transposed_msm_vectors
            .maybe_into_par_iter()
            .map(|v| FixedBaseMSM::new(v, use_precomp))
            .collect();

        Self {
            size_of_vector,
            circulant_domain,
            precomputed_fft_vectors,
            batch_size,
        }
    }

    /// Computes the aggregated sum of many Toeplitz matrix-vector multiplications.
    ///
    /// ie this method computes \sum_{i}^{n} A_i* x_i (where x_i is fixed)
    ///
    /// Note: This is faster than computing the matrix vector multiplication for each Toeplitz matrix using circulant
    /// matrix-vector multiplication and then summing the results since only one IFFT is done as opposed to `n`
    pub fn sum_matrix_vector_mul(&self, matrices: Vec<ToeplitzMatrix>) -> Vec<G1Projective> {
        assert_eq!(
            matrices.len(),
            self.batch_size,
            "expected the number of matrices to be the same as the number of vectors"
        );

        // Embed Toeplitz matrices into circulant matrices
        let circulant_matrices = matrices
            .maybe_into_par_iter()
            .map(CirculantMatrix::from_toeplitz);

        // Perform circulant matrix-vector multiplication between all of the matrices and vectors
        // and sum them together.
        //
        // Transpose the circulant matrices so that we convert a group of hadamard products into a group of
        // inner products.
        let col_ffts = circulant_matrices
            .maybe_into_par_iter()
            .map(|matrix| self.circulant_domain.fft_scalars(matrix.row.into()))
            .collect();
        let msm_scalars = transpose(col_ffts);

        let result = {
            #[cfg(feature = "tracing")]
            let _span =
                tracing::info_span!("compute fixed-base msm on matrix-vec-mul result").entered();
            self.precomputed_fft_vectors
                .maybe_par_iter()
                .zip(msm_scalars)
                .map(|(points, scalars)| points.msm(&scalars))
                .collect()
        };

        // Once the aggregate circulant matrix-vector multiplication is done, we need to take the first half
        // of the result, as the second half are extra terms that were added due to the fact that the Toeplitz matrices
        // were embedded into circulant matrices.
        self.circulant_domain
            .ifft_g1_take_n(result, Some(self.size_of_vector))
    }
}

/// Transposes a 2D matrix
///
/// This function takes a vector of vectors (representing a matrix) and returns its transpose,
/// ie a new matrix whose rows are the columns of the original.
///
/// # Examples
///
/// ```text
/// matrix = [
///     [1, 2, 3],
///     [4, 5, 6]
/// ];
/// Transpose will produce the following matrix:
///
/// [
///     [1, 4],
///     [2, 5],
///     [3, 6]
/// ]
/// ```
pub(crate) fn transpose<T: Clone>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    if v.is_empty() || v[0].is_empty() {
        return Vec::new();
    }

    let rows = v.len();
    let cols = v[0].len();

    let mut result = vec![Vec::with_capacity(rows); cols];

    for row in v {
        for (i, elem) in row.into_iter().enumerate() {
            result[i].push(elem);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use bls12_381::{
        fixed_base_msm::UsePrecomp, g1_batch_normalize, group::Group, G1Projective, Scalar,
    };

    use crate::fk20::{batch_toeplitz::BatchToeplitzMatrixVecMul, toeplitz::ToeplitzMatrix};

    #[test]
    fn smoke_aggregated_matrix_vector_mul() {
        // Create the toeplitz matrices and vectors that we want to perform matrix-vector multiplication with
        let mut toeplitz_matrices = Vec::new();
        let mut vectors = Vec::new();
        let mut vectors_affine = Vec::new();

        let num_matrices = 10;
        for i in 0..num_matrices {
            let col = vec![
                Scalar::from((i + 1) as u64),
                Scalar::from((i + 2) as u64),
                Scalar::from((i + 3) as u64),
                Scalar::from((i + 4) as u64),
            ];
            let row = vec![
                Scalar::from((i + 1) as u64),
                Scalar::from((i + 5) as u64),
                Scalar::from((i + 6) as u64),
                Scalar::from((i + 7) as u64),
            ];
            let vector = vec![
                G1Projective::generator() * Scalar::from((i + 1) as u64),
                G1Projective::generator() * Scalar::from((i + 2) as u64),
                G1Projective::generator() * Scalar::from((i + 3) as u64),
                G1Projective::generator() * Scalar::from((i + 4) as u64),
            ];

            vectors_affine.push(g1_batch_normalize(&vector.clone()));
            vectors.push(vector);
            toeplitz_matrices.push(ToeplitzMatrix::new(row, col));
        }

        let bm = BatchToeplitzMatrixVecMul::new(vectors_affine, UsePrecomp::Yes { width: 8 });
        let got_result = bm.sum_matrix_vector_mul(toeplitz_matrices.clone());

        let mut expected_result = vec![G1Projective::identity(); got_result.len()];
        for (matrix, vector) in toeplitz_matrices.into_iter().zip(vectors) {
            let intermediate_result = matrix.vector_mul_g1(vector);
            for (got, expected) in expected_result.iter_mut().zip(intermediate_result) {
                *got += expected;
            }
        }

        assert_eq!(expected_result, got_result);
    }
}
