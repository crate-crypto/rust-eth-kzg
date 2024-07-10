mod batch_toeplitz;
pub(crate) mod cosets;
mod h_poly;

#[cfg(test)]
pub(crate) mod naive;

mod toeplitz;
pub mod verifier;

use bls12_381::{g1_batch_normalize, group::Group, G1Point, G1Projective, Scalar};
use cosets::{log2, reverse_bits};
use h_poly::take_every_nth;
use polynomial::{domain::Domain, monomial::PolyCoeff};

use crate::commit_key::CommitKey;
use crate::fk20::batch_toeplitz::BatchToeplitzMatrixVecMul;
use cosets::reverse_bit_order;

/// Input contains the various structures that we can make FK20 proofs over.
pub enum Input {
    /// This is akin to creating proofs over a polynomial in monomial basis.
    PolyCoeff(Vec<Scalar>),
    /// Data: This is akin to creating proofs over a polynomial in lagrange basis.
    /// This variant has the useful property that the output evaluations will
    /// contain the data in the order that it was passed in.
    Data(Vec<Scalar>),
}

/// FK20 initializes all of the components needed to compute a KZG multi point
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
    ) -> FK20Prover {
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
            srs_vector.resize(pad_by, G1Projective::identity());
        }

        // Initialize structure that will allow us to do efficient sum of multiple toeplitz matrix
        // vector multiplication, where the vector is fixed.
        let batch_toeplitz = BatchToeplitzMatrixVecMul::new(srs_vectors);

        // 2. Compute the domains needed to produce the proofs and the evaluations
        //
        let num_proofs = number_of_points_to_open / points_per_proof;
        let proof_domain = Domain::new(num_proofs);
        let evaluation_domain = Domain::new(number_of_points_to_open);
        let poly_domain = Domain::new(polynomial_bound);

        FK20Prover {
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
    pub fn num_proofs(&self) -> usize {
        self.number_of_points_to_open / self.coset_size
    }

    /// Evaluates the polynomial at all of the relevant cosets.
    ///
    /// Instead of evaluating each coset individually, we can evaluate the polynomial
    /// at all of the points we want to open at, and then use reverse bit ordering
    /// to group the evaluations into the relevant cosets.
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

    /// Computes multi-opening proofs over a given polynomial in coefficient form.
    ///
    // Note: one can view this implementation of FK20 as only working over polynomials in coefficient form.
    // ie the core algorithms never consider polynomials in lagrange form.
    fn compute_multi_opening_proofs_poly_coeff(
        &self,
        polynomial: PolyCoeff,
    ) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        // Compute opening proofs for the polynomial
        //
        let h_poly_commitments =
            self.compute_h_poly_commitments(polynomial.clone(), self.coset_size);
        let mut proofs = self.proof_domain.fft_g1(h_poly_commitments);

        // Reverse bit order the set of proofs, so that the proofs line up with the
        // coset evaluations.
        reverse_bit_order(&mut proofs);

        (
            g1_batch_normalize(&proofs),
            self.compute_coset_evaluations(polynomial),
        )
    }

    /// Given a group of coset evaluations, this method will return/reorder the evaluations as if
    /// we evaluated them on the relevant extended domain.
    /// The coset indices are returned in domain order.
    //
    // Note: For evaluations that are missing, this method will fill these in with zeroes.
    //
    // Note: It is the callers responsibility to ensure that there are no duplicate
    // coset indices.
    // TODO: possibly put on FK20verifier
    pub fn recover_evaluations_in_domain_order(
        domain_size: usize,
        coset_indices: Vec<usize>,
        coset_evaluations: Vec<Vec<Scalar>>,
    ) -> Option<(Vec<usize>, Vec<Scalar>)> {
        assert!(coset_indices.len() == coset_evaluations.len());

        if coset_indices.is_empty() {
            return None;
        }

        let mut elements = vec![Scalar::from(0u64); domain_size];

        // Check that each coset has the same size
        let coset_len = coset_evaluations[0].len();
        let same_len = coset_evaluations
            .iter()
            .all(|coset| coset.len() == coset_len);
        if !same_len {
            return None;
        }

        // Check that none of the indices are "out of bounds"
        // This would result in the subsequent indexing operations to panic
        //
        // The greatest index we will be using is:
        // `t = coset_index * coset_len`
        // lets denote the returned vectors length as `k`
        // We want t < k
        // => coset_index * coset_len < k
        // => coset_index < k / coset_len
        let index_bound = domain_size / coset_len;
        let all_coset_indices_within_bound = coset_indices
            .iter()
            .all(|coset_index| *coset_index < index_bound);
        if !all_coset_indices_within_bound {
            return None;
        }

        // Iterate over each coset evaluation set and place the evaluations in the correct locations
        for (&coset_index, coset_evals) in coset_indices.iter().zip(coset_evaluations) {
            let start = coset_index * coset_len;
            let end = start + coset_len;

            elements[start..end].copy_from_slice(&coset_evals);
        }

        // Now bit reverse the result, so we get the evaluations as if we had just done
        // and FFT on them. ie we computed the evaluation set and did not do a reverse bit order.
        reverse_bit_order(&mut elements);

        // The order of the coset indices in the returned vector will be different.
        // The new indices of the cosets can be figured out by reverse bit ordering
        // the existing indices.
        let cosets_per_full_domain = domain_size / coset_len;
        let num_bits_coset_per_full_domain = log2(cosets_per_full_domain as u32);

        let new_coset_indices: Vec<_> = coset_indices
            .into_iter()
            .map(|rbo_coset_index| reverse_bits(rbo_coset_index, num_bits_coset_per_full_domain))
            .collect();

        Some((new_coset_indices, elements))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{FK20Prover, Input};
    use crate::{
        create_insecure_commit_opening_keys,
        fk20::{cosets::generate_cosets, naive as fk20naive, verifier::FK20Verifier},
        naive as kzgnaive,
    };
    use bls12_381::Scalar;

    #[test]
    fn data_is_contained_in_the_first_section_of_cells() {
        // This tests that if we create proofs over Input::Data
        // then the first set of cells will contain the data.

        let (commit_key, _) = create_insecure_commit_opening_keys();

        let poly_len = 4096;
        let num_points_to_open = 2 * poly_len;
        let coset_size = 64;

        let fk20 = FK20Prover::new(commit_key, poly_len, coset_size, num_points_to_open);

        let data: Vec<_> = (0..poly_len).map(|i| Scalar::from(i as u64)).collect();
        let (_, cells) = fk20.compute_multi_opening_proofs(Input::Data(data.clone()));

        // Now check that the first set of cells contains the data
        let cells_flattened = cells.into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&data, &cells_flattened[..poly_len]);
    }

    #[test]
    fn smoke_test_prove_verify() {
        let (commit_key, opening_key) = create_insecure_commit_opening_keys();

        let poly_len = 4096;
        let num_points_to_open = 2 * poly_len;
        let coset_size = 64;
        let num_cosets = num_points_to_open / coset_size;

        let fk20 = FK20Prover::new(commit_key, poly_len, coset_size, num_points_to_open);
        let fk20_verifier = FK20Verifier::new(opening_key, num_points_to_open, num_cosets, true);

        let data: Vec<_> = (0..poly_len).map(|i| Scalar::from(i as u64)).collect();
        let (proofs, cells) = fk20.compute_multi_opening_proofs(Input::Data(data.clone()));

        let commitment = fk20.commit(Input::Data(data));

        let coset_indices: Vec<u64> = (0..num_cosets as u64).collect();

        let is_valid = fk20_verifier.verify_multi_opening(
            &vec![commitment],
            &vec![0u64; num_cosets],
            &coset_indices,
            &cells,
            &proofs,
        );
        assert!(is_valid);
    }

    #[test]
    fn check_consistency_of_proofs_against_naive_fk20_implementation() {
        let poly_len = 4096;
        let poly: Vec<_> = (0..poly_len).map(|i| -Scalar::from(i as u64)).collect();
        let coset_size = 64;
        let (commit_key, _) = create_insecure_commit_opening_keys();

        // Compute the proofs and evaluations using naive fk20
        let (expected_proofs, expected_evaluations) =
            fk20naive::open_multi_point(&commit_key, &poly, coset_size, 2 * poly_len);

        // Compute proofs using optimized FK20 implementation
        let fk20 = FK20Prover::new(commit_key, poly_len, coset_size, 2 * poly_len);
        let (got_proofs, got_evaluations) =
            fk20.compute_multi_opening_proofs_poly_coeff(poly.clone());

        assert_eq!(got_proofs.len(), expected_proofs.len());
        assert_eq!(got_evaluations.len(), expected_evaluations.len());

        assert_eq!(got_evaluations, expected_evaluations);
        assert_eq!(got_proofs, expected_proofs);
    }

    #[test]
    fn test_consistency_between_naive_kzg_naive_fk20() {
        // Setup
        //
        let (ck, _) = create_insecure_commit_opening_keys();

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
