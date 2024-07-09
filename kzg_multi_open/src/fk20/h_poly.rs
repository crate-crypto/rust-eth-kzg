use crate::fk20::{toeplitz::ToeplitzMatrix, FK20};
use bls12_381::{G1Projective, Scalar};
use polynomial::monomial::PolyCoeff;

/// Given a vector `k` and an integer `l`
/// Where `l` is less than |k|. We return `l-downsampled` groups.
/// Example: k = [a_0, a_1, a_3, a_4, a_5, a_6], l = 2
/// Result = [[a_0, a_3, a_5], [a_1, a_4, a_6]]
#[inline(always)]
pub(crate) fn take_every_nth<T: Clone + Copy>(list: &[T], n: usize) -> Vec<Vec<T>> {
    (0..n)
        .map(|i| list.iter().copied().skip(i).step_by(n).collect())
        .collect()
}

impl FK20 {
    /// Computes the `h` polynomials for the FK20 proofs.
    ///
    /// The `h` polynomial refer to the polynomial that are shared across the computation
    /// of different proofs. The main trick behind FK20 is to compute these polynomials
    /// once and then use an FFT to compute all of the proofs from commitment to these
    /// polynomial.
    ///
    /// See section 3.1.1 of the FK20 paper for more details.
    ///
    /// FK20 computes the commitments to these polynomials in 3.1.1.
    pub(crate) fn compute_h_poly_commitments(
        &self,
        mut polynomial: PolyCoeff,
        coset_size: usize,
    ) -> Vec<G1Projective> {
        assert!(
            coset_size.is_power_of_two(),
            "expected coset_size to be a power of two, found {}",
            coset_size
        );

        let num_coefficients: usize = polynomial.len();
        assert!(
            num_coefficients.is_power_of_two(),
            "expected polynomial to have power of 2 number of coefficients. Found {}",
            num_coefficients
        );

        // Reverse polynomial so highest coefficient is first.
        // See 3.1.1 of the FK20 paper, for the ordering.
        polynomial.reverse();

        // Compute the toeplitz rows for the `l` toeplitz matrices
        let toeplitz_rows = take_every_nth(&polynomial, coset_size);

        // Compute the Toeplitz matrices
        let mut matrices = Vec::with_capacity(toeplitz_rows.len());
        // We want to do `l` toeplitz matrix multiplications
        for row in toeplitz_rows.into_iter() {
            let mut toeplitz_column = vec![Scalar::from(0u64); row.len()];
            toeplitz_column[0] = row[0];

            matrices.push(ToeplitzMatrix::new(row, toeplitz_column));
        }

        // Compute `l` toeplitz matrix-vector multiplications and sum them together
        self.batch_toeplitz.sum_matrix_vector_mul(matrices)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        create_insecure_commit_opening_keys,
        fk20::{h_poly::take_every_nth, naive, FK20},
    };
    use bls12_381::Scalar;

    #[test]
    fn smoke_test_downsample() {
        let k = vec![5, 4, 3, 2];
        let downsampled_lists = take_every_nth(&k, 2);
        let result = vec![vec![5, 3], vec![4, 2]];
        assert_eq!(downsampled_lists, result)
    }

    #[test]
    fn check_consistency_of_toeplitz_h_polys() {
        let poly: Vec<_> = (0..4096).map(|i| -Scalar::from(i)).collect();
        let coset_size: usize = 64;
        let (commit_key, _) = create_insecure_commit_opening_keys();

        // Compute the commitment to the h_polynomials naively
        //
        let h_polynomials = naive::compute_h_poly(&poly, coset_size);
        let expected_comm_h_polys = h_polynomials
            .iter()
            .map(|h_poly| commit_key.commit_g1(h_poly))
            .collect::<Vec<_>>();

        // Compute the commitment to the h_polynomials using the method noted in the FK20 paper
        //
        let fk20 = FK20::new(commit_key, 4096, coset_size, 2 * 4096);
        let got_comm_h_polys = fk20.compute_h_poly_commitments(poly, coset_size);

        assert_eq!(expected_comm_h_polys.len(), got_comm_h_polys.len());
        assert_eq!(expected_comm_h_polys, got_comm_h_polys);
    }
}
