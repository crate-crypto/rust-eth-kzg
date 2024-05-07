use bls12_381::fixed_base_msm::FixedBaseMSM;
use bls12_381::lincomb::g1_lincomb;
use bls12_381::G1Projective;
use polynomial::domain::Domain;

use super::toeplitz::ToeplitzMatrix;
use crate::fk20::toeplitz::CirculantMatrix;
/// BatchToeplitz is a structure that optimizes for the usecase where:
/// - You need to do multiple matrix-vector multiplications and sum them together
/// - The vector is known at compile time, so you can precompute it's FFT
/// - For now, the vector is a group element. We don't have any other usecases in the codebase.
pub struct BatchToeplitzMatrixVecMul {
    /// fft_vectors represents the group elements in the FFT domain.
    /// This means when we are computing the matrix-vector multiplication by embedding it
    /// into a circulant matrix, we will not need to do an FFT on the vector.
    fft_vectors: Vec<Vec<G1Projective>>,
    /// Multiples of the fft_vectors that are precomputed for the fixed base msm
    /// calculation.
    precomputed_fft_vectors: Vec<FixedBaseMSM>,
    // This is the length of the vector that we are multiplying the matrix with.
    // and subsequently will be the length of the final result of the matrix-vector multiplication.
    n: usize,
    /// This is the domain used in the circulant matrix-vector multiplication.
    /// It will be double the size of the length of the vector.
    circulant_domain: Domain,
}

impl BatchToeplitzMatrixVecMul {
    pub fn new(vectors: Vec<Vec<G1Projective>>) -> Self {
        let n = vectors[0].len();
        let vectors_all_same_length = vectors.iter().all(|v| v.len() == n);
        assert!(
            vectors_all_same_length,
            "expected all vectors to be the same length"
        );

        let circulant_domain = Domain::new(n * 2);

        // Precompute the FFT of the vectors
        let vectors: Vec<Vec<G1Projective>> = vectors
            .into_iter()
            .map(|vector| circulant_domain.fft_g1(vector))
            .collect();

        let mut transposed_msm_vectors = vec![vec![]; n * 2];
        for vector in &vectors {
            for (i, a) in vector.iter().enumerate() {
                transposed_msm_vectors[i].push(*a);
            }
        }

        let table_bits = 9;
        let precomputed_table: Vec<_> = transposed_msm_vectors
            .into_iter()
            .map(|v| FixedBaseMSM::new(v, table_bits))
            .collect();

        BatchToeplitzMatrixVecMul {
            n,
            fft_vectors: vectors,
            circulant_domain,
            precomputed_fft_vectors: precomputed_table,
        }
    }

    // Computes the aggregated sum of many Toeplitz matrix-vector multiplications.
    //
    // ie this method computes \sum_{i}^{n} A_i* x_i
    //
    // This is faster than computing the matrix vector multiplication for each Toeplitz matrix and then summing the results
    // since only one IFFT is done as opposed to `n`
    // TODO: This method should be refactored for better readability, once we are done applying optimizations
    pub fn sum_matrix_vector_mul(&self, matrices: Vec<ToeplitzMatrix>) -> Vec<G1Projective> {
        assert_eq!(
            matrices.len(),
            self.fft_vectors.len(),
            "expected the number of matrices to be the same as the number of vectors"
        );

        // Embed Toeplitz matrices into Circulant matrices
        let circulant_matrices = matrices.into_iter().map(CirculantMatrix::from_toeplitz);

        // Perform circulant matrix-vector multiplication between all of the matrices and vectors
        // and sum them together.
        //
        // We note that the aggregation step can be converted into msm's of size `l`
        let col_ffts = circulant_matrices
            .into_iter()
            .map(|matrix| self.circulant_domain.fft_scalars(matrix.row));
        let mut msm_scalars = vec![vec![]; self.n * 2];

        // Transpose the column ffts
        for col_fft in col_ffts {
            for (i, b) in col_fft.into_iter().enumerate() {
                msm_scalars[i].push(b);
            }
        }

        let result: Vec<_> = self
            .precomputed_fft_vectors
            .iter()
            .zip(msm_scalars.into_iter())
            .map(|(points, scalars)| points.msm(scalars))
            .collect();
        let circulant_sum = self.circulant_domain.ifft_g1(result);

        // Once the Circulant matrix-vector multiplication is done, we need to take the first half
        // of the result, as this is the result of the Toeplitz matrix multiplication
        circulant_sum[0..self.n].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use crate::fk20::batch_toeplitz::BatchToeplitzMatrixVecMul;
    use crate::fk20::toeplitz::ToeplitzMatrix;
    use bls12_381::group::Group;
    use bls12_381::{G1Projective, Scalar};

    #[test]
    fn smoke_aggregated_matrix_vector_mul() {
        // Create the toeplitz matrices and vectors that we want to perform matrix-vector multiplication with
        let mut toeplitz_matrices = Vec::new();
        let mut vectors = Vec::new();

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

            vectors.push(vector);
            toeplitz_matrices.push(ToeplitzMatrix::new(row, col));
        }

        let bm = BatchToeplitzMatrixVecMul::new(vectors.clone());
        let got_result = bm.sum_matrix_vector_mul(toeplitz_matrices.clone());

        let mut expected_result = vec![G1Projective::identity(); got_result.len()];
        for (matrix, vector) in toeplitz_matrices.into_iter().zip(vectors) {
            let intermediate_result = matrix.vector_mul_g1(vector);
            for (got, expected) in expected_result.iter_mut().zip(intermediate_result) {
                *got += expected;
            }
        }

        assert_eq!(expected_result, got_result)
    }
}
