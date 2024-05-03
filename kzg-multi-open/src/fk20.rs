// [FK20] is a paper by Dmitry Khovratovich and Dankrad Feist that describes a method for
// efficiently opening a set of points when the opening points are roots of unity.

mod batch_toeplitz;
mod toeplitz;

pub mod naive;

use bls12_381::group::prime::PrimeCurveAffine;
use bls12_381::group::Curve;
use bls12_381::group::Group;
use bls12_381::{G1Point, G1Projective, Scalar};
use polynomial::{domain::Domain, monomial::PolyCoeff};

use crate::fk20::batch_toeplitz::BatchToeplitzMatrixVecMul;
use crate::fk20::toeplitz::ToeplitzMatrix;
use crate::{commit_key::CommitKey, reverse_bit_order};

/// FK20 initializes all of the components needed to compute a KZG multipoint
/// proof using the FK20 method.
pub struct FK20 {
    batch_toeplitz: BatchToeplitzMatrixVecMul,
}

impl FK20 {
    pub fn new(commit_key: &CommitKey, l: usize) -> FK20 {
        // Compute the SRS vectors that we will multiply the toeplitz matrices by.
        //
        // Skip the last `l` points in the srs
        let srs_truncated: Vec<_> = commit_key.g1s.clone().into_iter().rev().skip(l).collect();
        let mut srs_vectors = take_every_nth(&srs_truncated, l);

        // TODO: We don't need to do this padding, since `BatchToeplitzMatrixVecMul` doesn't
        // TODO necessitate it.
        // Pad srs vectors by the next power of two
        for srs_vector in &mut srs_vectors {
            let pad_by = srs_vector.len().next_power_of_two();
            srs_vector.resize(pad_by, G1Projective::identity());
        }

        // Compute `l` toeplitz matrix-vector multiplications and sum them together
        let batch_toeplitz = BatchToeplitzMatrixVecMul::new(srs_vectors);
        FK20 { batch_toeplitz }
    }

    pub fn compute_multi_opening_proofs(
        &self,
        polynomial: PolyCoeff,
        ext_domain: &Domain,
        proof_domain: &Domain,
        l: usize,
    ) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        // TODO: reverse bit order on the polynomial

        // Compute proofs for the polynomial
        let h_poly_commitments = self.compute_h_poly_commitments(polynomial.clone(), l);
        let mut proofs = proof_domain.fft_g1(h_poly_commitments);
        // apply reverse bit order permutation, since fft_g1 was applied using
        // the regular order.
        reverse_bit_order(&mut proofs);
        let mut proofs_affine = vec![G1Point::identity(); proofs.len()];
        // TODO: This does not seem to be using the batch affine trick
        bls12_381::G1Projective::batch_normalize(&proofs, &mut proofs_affine);

        // Compute the evaluations of the polynomial at the cosets by doing an fft
        let mut evaluations = ext_domain.fft_scalars(polynomial.clone());
        reverse_bit_order(&mut evaluations);
        let set_of_output_points: Vec<_> = evaluations
            .chunks_exact(l)
            .map(|slice| slice.to_vec())
            .collect();

        (proofs_affine, set_of_output_points)
    }

    fn compute_h_poly_commitments(&self, mut polynomial: PolyCoeff, l: usize) -> Vec<G1Projective> {
        assert!(
            l.is_power_of_two(),
            "expected l to be a power of two (its the size of the cosets), found {}",
            l
        );

        let m = polynomial.len();
        assert!(
            m.is_power_of_two(),
            "expected polynomial to have power of 2 number of evaluations. Found {}",
            m
        );

        let k = m / l;
        assert!(
            k.is_power_of_two(),
            "expected k to be a power of two, found {}",
            k
        );

        // Reverse polynomial so highest coefficient is first.
        // See 3.1.1 of the FK20 paper, for the ordering.
        polynomial.reverse();

        // Compute the toeplitz rows for the `l` toeplitz matrices
        let toeplitz_rows = take_every_nth(&polynomial, l);

        // Compute the Toeplitz matrices
        let mut matrices = Vec::with_capacity(toeplitz_rows.len());
        // We want to do `l` toeplitz matrix multiplications
        for row in toeplitz_rows.into_iter() {
            // TODO: We could have a special constructor/Toeplitz struct for the column,
            // TODO: if this allocation shows to be non-performant.
            let mut toeplitz_column = vec![Scalar::from(0u64); row.len()];
            toeplitz_column[0] = row[0];

            matrices.push(ToeplitzMatrix::new(row, toeplitz_column));
        }

        // Compute `l` toeplitz matrix-vector multiplications and sum them together
        self.batch_toeplitz.sum_matrix_vector_mul(matrices)
    }
}

/// Given a vector `k` and an integer `l`
/// Where `l` is less than |k|. We return `l-downsampled` groups.
/// Example: k = [a_0, a_1, a_3, a_4, a_5, a_6], l = 2
/// Result = [[a_0, a_3, a_5], [a_1, a_4, a_6]]
#[inline(always)]
fn take_every_nth<T: Clone + Copy>(list: &[T], n: usize) -> Vec<Vec<T>> {
    (0..n)
        .map(|i| list.iter().copied().skip(i).step_by(n).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        create_eth_commit_opening_keys,
        fk20::{naive, take_every_nth, FK20},
    };
    use bls12_381::group::Group;
    use bls12_381::Scalar;
    use polynomial::domain::Domain;

    #[test]
    fn smoke_test_downsample() {
        let k = vec![5, 4, 3, 2];
        let downsampled_lists = take_every_nth(&k, 2);
        let result = vec![vec![5, 3], vec![4, 2]];
        assert_eq!(downsampled_lists, result)
    }

    #[test]
    fn check_consistency_of_toeplitz_h_polys() {
        use bls12_381::ff::Field;
        let poly = vec![Scalar::random(&mut rand::thread_rng()); 4096];
        let l = 64;
        let (commit_key, _) = create_eth_commit_opening_keys();

        let h_polynomials = naive::compute_h_poly(&poly, l);
        let mut expected_comm_h_polys = h_polynomials
            .iter()
            .map(|h_poly| commit_key.commit_g1(h_poly))
            .collect::<Vec<_>>();
        // Add the identity element to h_polys to pad it to a power of two
        expected_comm_h_polys.push(bls12_381::G1Projective::identity());
        let fk20 = FK20::new(&commit_key, l);
        let got_comm_h_polys = fk20.compute_h_poly_commitments(poly, l);
        assert_eq!(expected_comm_h_polys.len(), got_comm_h_polys.len());
        assert_eq!(expected_comm_h_polys, got_comm_h_polys);
    }

    #[test]
    fn check_consistency_of_proofs_against_naive() {
        use bls12_381::ff::Field;
        let poly_len = 4096;
        let poly = vec![Scalar::random(&mut rand::thread_rng()); poly_len];
        let l = 64;
        let (commit_key, _) = create_eth_commit_opening_keys();
        let proof_domain = Domain::new(l);
        let ext_domain = Domain::new(2 * poly_len);

        let (expected_proofs, expected_evaluations) =
            naive::fk20_open_multi_point(&commit_key, &proof_domain, &ext_domain, &poly, l);

        let fk20 = FK20::new(&commit_key, l);
        let (got_proofs, got_evaluations) =
            fk20.compute_multi_opening_proofs(poly, &ext_domain, &proof_domain, l);

        assert_eq!(got_proofs.len(), expected_proofs.len());
        assert_eq!(got_evaluations.len(), expected_evaluations.len());

        assert_eq!(got_evaluations, expected_evaluations);
        assert_eq!(got_proofs, expected_proofs);
    }
}
