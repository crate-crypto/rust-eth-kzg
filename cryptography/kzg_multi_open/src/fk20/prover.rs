use crate::commit_key::CommitKey;
use crate::fk20::batch_toeplitz::BatchToeplitzMatrixVecMul;
use crate::fk20::cosets::reverse_bit_order;
use crate::fk20::h_poly::take_every_nth;
use bls12_381::fixed_base_msm::UsePrecomp;
use bls12_381::group::prime::PrimeCurveAffine;
use bls12_381::{g1_batch_normalize, G1Point, Scalar};
use polynomial::{domain::Domain, poly_coeff::PolyCoeff};

use super::h_poly::compute_h_poly_commitments;

/// Input contains the various structures that we can make FK20 proofs over.
pub enum Input {
    /// This is akin to creating proofs over a polynomial in monomial basis.
    PolyCoeff(Vec<Scalar>),
    /// Data: This is akin to creating proofs over a polynomial in lagrange basis.
    /// This variant has the useful property that the output evaluations will
    /// contain the data in the order that it was passed in.
    Data(Vec<Scalar>),
}

/// FK20Prover initializes all of the components needed to compute a KZG multi point
/// proof using the FK20 method.
///
/// The FK20 method gives an efficient algorithm for opening points, where
/// the points are roots of unity. (It cannot be used to open arbitrary points)
///
/// See [Fk21](https://github.com/khovratovich/Kate/blob/master/Kate_amortized.pdf) for details
/// on the scheme.
#[derive(Debug)]
pub struct FK20Prover {
    batch_toeplitz: BatchToeplitzMatrixVecMul,
    /// The amount of points that a single proof will attest to the opening of.
    ///
    /// Note: FK20 allows you to create a proof of an opening for multiple points.
    /// Each proof will attest to the opening of `l` points.
    /// In the FK20 paper, this is also referred to as `l` (ELL).
    coset_size: usize,
    /// The total number of points that we want to open a polynomial at.
    ///
    /// Note: A proof will attest to `point_set_size` of these points at a
    /// time.
    number_of_points_to_open: usize,

    /// Domain used in FK20 to create the opening proofs
    proof_domain: Domain,
    /// Domain used to evaluate the polynomial at the points we want to open at.
    evaluation_domain: Domain,
    /// Domain used for converting polynomial to monomial form.
    poly_domain: Domain,
    /// Commitment key used for committing to the polynomial
    /// in monomial form.
    commit_key: CommitKey,
}

impl FK20Prover {
    /// Initialize a FK20 struct with the given parameters.
    ///
    /// commit_key: The commitment key used to commit to polynomials.
    /// polynomial_bound: The number of coefficients in the polynomial.
    /// points_per_proof: The number of points that a single proof will attest to.
    /// number_of_points_to_open: The total number of points that we want to open a polynomial at.
    pub fn new(
        commit_key: CommitKey,
        polynomial_bound: usize,
        points_per_proof: usize,
        number_of_points_to_open: usize,
        use_precomp: UsePrecomp,
    ) -> Self {
        assert!(points_per_proof.is_power_of_two());
        assert!(number_of_points_to_open.is_power_of_two());
        assert!(number_of_points_to_open > points_per_proof);
        assert!(polynomial_bound.is_power_of_two());
        assert!(commit_key.g1s.len() >= polynomial_bound);
        assert!(commit_key.g1s.len() > points_per_proof);

        // 1. Compute the SRS vectors that we will multiply the toeplitz matrices by.
        //
        // Skip the last `coset_size` points in the srs
        //
        // To intuitively understand why this normal, note that the conventional
        // KZG polynomial commitment scheme for opening a polynomial at a single point
        // does not require all of the coefficients of the polynomial to compute
        // the quotient polynomial.
        let srs_truncated: Vec<_> = commit_key
            .g1s
            .clone()
            .into_iter()
            .rev()
            .skip(points_per_proof)
            .collect();
        let mut srs_vectors = take_every_nth(&srs_truncated, points_per_proof);

        // Pad srs vectors to the next power of two
        //
        // This is not strictly needed since our FFT implementation
        // will pad these.
        for srs_vector in &mut srs_vectors {
            let pad_by = srs_vector.len().next_power_of_two();
            srs_vector.resize(pad_by, G1Point::identity());
        }

        // Initialize structure that will allow us to do efficient sum of multiple toeplitz matrix
        // vector multiplication, where the vector is fixed.
        let batch_toeplitz = BatchToeplitzMatrixVecMul::new(srs_vectors, use_precomp);

        // 2. Compute the domains needed to produce the proofs and the evaluations
        //
        let num_proofs = number_of_points_to_open / points_per_proof;
        let proof_domain = Domain::new(num_proofs);
        let evaluation_domain = Domain::new(number_of_points_to_open);
        let poly_domain = Domain::new(polynomial_bound);

        Self {
            batch_toeplitz,
            coset_size: points_per_proof,
            number_of_points_to_open,
            proof_domain,
            evaluation_domain,
            poly_domain,
            commit_key,
        }
    }

    /// Commit to the `Input` that we will be creating FK20 proofs over.
    pub fn commit(&self, input: Input) -> G1Point {
        let poly_coeff = match input {
            Input::PolyCoeff(poly_coeff) => poly_coeff,
            Input::Data(mut data) => {
                // Reverse the order of the data, so that they are in bit-reversed order.
                //
                // FK20 will operate over the bit-reversed permutation of the data.
                reverse_bit_order(&mut data);

                // Interpolate the data, to get a polynomial in monomial form that corresponds
                // to the bit reversed data.
                self.poly_domain.ifft_scalars(data)
            }
        };

        // Commit to the interpolated polynomial.
        self.commit_key.commit_g1(&poly_coeff).into()
    }

    /// The number of proofs that will be produced.
    pub const fn num_proofs(&self) -> usize {
        self.number_of_points_to_open / self.coset_size
    }

    /// Evaluates the polynomial at all of the relevant cosets.
    ///
    /// Instead of evaluating each coset individually, we can evaluate the polynomial
    /// at all of the points we want to open at, and then use reverse bit ordering
    /// to group the evaluations into the relevant cosets.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    fn compute_coset_evaluations(&self, polynomial: PolyCoeff) -> Vec<Vec<Scalar>> {
        let mut evaluations = self.evaluation_domain.fft_scalars(polynomial);
        reverse_bit_order(&mut evaluations);
        evaluations
            .chunks_exact(self.coset_size)
            .map(|slice| slice.to_vec())
            .collect()
    }

    /// Computes multi-opening proofs over the given `Input`.
    ///
    /// When the input is set to Data;
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#compute_cells_and_kzg_proofs
    ///
    /// Returning the opening proofs and the corresponding coset evaluations.
    pub fn compute_multi_opening_proofs(&self, input: Input) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        // Convert data to polynomial coefficients
        let poly_coeff = match input {
            Input::PolyCoeff(polynomial) => polynomial,
            Input::Data(mut data) => {
                reverse_bit_order(&mut data);
                self.poly_domain.ifft_scalars(data)
            }
        };

        self.compute_multi_opening_proofs_poly_coeff(poly_coeff)
    }

    /// Extends the polynomial by computing its coset evaluations
    pub fn extend_polynomial(&self, input: Input) -> Vec<Vec<Scalar>> {
        // Convert data to polynomial coefficients
        let poly_coeff = match input {
            Input::PolyCoeff(polynomial) => polynomial,
            Input::Data(mut data) => {
                reverse_bit_order(&mut data);
                self.poly_domain.ifft_scalars(data)
            }
        };
        self.compute_coset_evaluations(poly_coeff)
    }

    /// Computes multi-opening proofs over a given polynomial in coefficient form.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#compute_cells_and_kzg_proofs_polynomialcoeff
    //
    // Note: one can view this implementation of FK20 as only working over polynomials in coefficient form.
    // ie the core algorithms never consider polynomials in lagrange form.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    fn compute_multi_opening_proofs_poly_coeff(
        &self,
        polynomial: PolyCoeff,
    ) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        // Compute opening proofs for the polynomial
        //
        let h_poly_commitments =
            compute_h_poly_commitments(&self.batch_toeplitz, polynomial.clone(), self.coset_size);
        let mut proofs = {
            #[cfg(feature = "tracing")]
            let _span = tracing::info_span!("compute proof from h_poly_commitments").entered();
            self.proof_domain.fft_g1(h_poly_commitments)
        };

        // Reverse bit order the set of proofs, so that the proofs line up with the
        // coset evaluations.
        reverse_bit_order(&mut proofs);

        (
            g1_batch_normalize(&proofs),
            self.compute_coset_evaluations(polynomial),
        )
    }

    #[cfg(test)]
    pub(crate) const fn batch_toeplitz_matrix(&self) -> &BatchToeplitzMatrixVecMul {
        &self.batch_toeplitz
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{FK20Prover, Input};
    use crate::{
        create_insecure_commit_verification_keys,
        fk20::{cosets::generate_cosets, naive as fk20naive, verifier::FK20Verifier},
        naive as kzgnaive,
    };
    use bls12_381::{fixed_base_msm::UsePrecomp, Scalar};

    #[test]
    fn data_is_contained_in_the_first_section_of_cells() {
        // This tests that if we create proofs over Input::Data
        // then the first set of cells will contain the data.

        let (commit_key, _) = create_insecure_commit_verification_keys();

        let poly_len = 4096;
        let num_points_to_open = 2 * poly_len;
        let coset_size = 64;

        let fk20 = FK20Prover::new(
            commit_key,
            poly_len,
            coset_size,
            num_points_to_open,
            UsePrecomp::No,
        );

        let data: Vec<_> = (0..poly_len).map(|i| Scalar::from(i as u64)).collect();
        let (_, cells) = fk20.compute_multi_opening_proofs(Input::Data(data.clone()));

        // Now check that the first set of cells contains the data
        let cells_flattened = cells.into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&data, &cells_flattened[..poly_len]);
    }

    #[test]
    fn smoke_test_prove_verify() {
        let (commit_key, verification_key) = create_insecure_commit_verification_keys();

        let poly_len = 4096;
        let num_points_to_open = 2 * poly_len;
        let coset_size = 64;
        let num_cosets = num_points_to_open / coset_size;

        let fk20 = FK20Prover::new(
            commit_key,
            poly_len,
            coset_size,
            num_points_to_open,
            UsePrecomp::No,
        );
        let fk20_verifier = FK20Verifier::new(verification_key, num_points_to_open, num_cosets);

        let data: Vec<_> = (0..poly_len).map(|i| Scalar::from(i as u64)).collect();
        let (proofs, cells) = fk20.compute_multi_opening_proofs(Input::Data(data.clone()));

        let commitment = fk20.commit(Input::Data(data));

        let coset_indices: Vec<u64> = (0..num_cosets as u64).collect();

        let valid = fk20_verifier.verify_multi_opening(
            &[commitment],
            &vec![0u64; num_cosets],
            &coset_indices,
            &cells,
            &proofs,
        );
        assert!(valid.is_ok());
    }

    #[test]
    fn check_consistency_of_proofs_against_naive_fk20_implementation() {
        let poly_len = 4096;
        let poly: Vec<_> = (0..poly_len).map(|i| -Scalar::from(i as u64)).collect();
        let coset_size = 64;
        let (commit_key, _) = create_insecure_commit_verification_keys();

        // Compute the proofs and evaluations using naive fk20
        let (expected_proofs, expected_evaluations) =
            fk20naive::open_multi_point(&commit_key, &poly, coset_size, 2 * poly_len);

        // Compute proofs using optimized FK20 implementation
        let fk20 = FK20Prover::new(
            commit_key,
            poly_len,
            coset_size,
            2 * poly_len,
            UsePrecomp::No,
        );
        let (got_proofs, got_evaluations) = fk20.compute_multi_opening_proofs_poly_coeff(poly);

        assert_eq!(got_proofs.len(), expected_proofs.len());
        assert_eq!(got_evaluations.len(), expected_evaluations.len());

        assert_eq!(got_evaluations, expected_evaluations);
        assert_eq!(got_proofs, expected_proofs);
    }

    #[test]
    fn test_consistency_between_naive_kzg_naive_fk20() {
        // Setup
        //
        let (ck, _) = create_insecure_commit_verification_keys();

        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;
        const COSET_SIZE: usize = 64;

        let cosets = generate_cosets(NUMBER_OF_POINTS_TO_EVALUATE, COSET_SIZE, true);

        let polynomial: Vec<_> = (0..POLYNOMIAL_LEN)
            .map(|i| -Scalar::from(i as u64))
            .collect();

        // Compute FK20 the naive way
        let (got_proofs, got_set_of_output_points) =
            fk20naive::open_multi_point(&ck, &polynomial, COSET_SIZE, NUMBER_OF_POINTS_TO_EVALUATE);

        for k in 0..got_proofs.len() {
            let input_points = &cosets[k];
            // Compute the opening proofs the naive way (without fk20)
            let (expected_quotient_comm, expected_output_points) =
                kzgnaive::compute_multi_opening(&ck, &polynomial, input_points);

            // Output points will be the same set, but they won't be in the same order
            // since generate_cosets does not use the bit_reverse_order method.
            //
            // We compare them as multi-sets in this case.
            assert!(set_equality_scalar(
                &expected_output_points,
                &got_set_of_output_points[k]
            ));
            assert_eq!(expected_quotient_comm, got_proofs[k]);
        }
    }

    fn set_equality_scalar(lhs: &[Scalar], rhs: &[Scalar]) -> bool {
        if lhs.len() != rhs.len() {
            return false;
        }

        let lhs_set: HashSet<_> = lhs.iter().map(|s| s.to_bytes_be()).collect();
        let rhs_set: HashSet<_> = rhs.iter().map(|s| s.to_bytes_be()).collect();

        lhs_set == rhs_set
    }
}
