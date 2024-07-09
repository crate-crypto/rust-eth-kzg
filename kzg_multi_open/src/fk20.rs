mod batch_toeplitz;
pub(crate) mod cosets;
mod h_poly;

#[cfg(test)]
pub(crate) mod naive;

mod toeplitz;
pub mod verify;

use bls12_381::{g1_batch_normalize, group::Group, G1Point, G1Projective, Scalar};
use cosets::{log2, reverse_bits};
use h_poly::take_every_nth;
use polynomial::{domain::Domain, monomial::PolyCoeff};

use crate::commit_key::CommitKey;
use crate::fk20::batch_toeplitz::BatchToeplitzMatrixVecMul;

use cosets::reverse_bit_order;

pub use cosets::coset_gens;
/// FK20 initializes all of the components needed to compute a KZG multi point
/// proof using the FK20 method.
///
/// The FK20 method gives an efficient algorithm for opening points, where
/// the points are roots of unity. (It cannot be used to open arbitrary points)
///
/// See [Fk21](https://github.com/khovratovich/Kate/blob/master/Kate_amortized.pdf) for details
/// on the scheme.
#[derive(Debug)]
pub struct FK20 {
    batch_toeplitz: BatchToeplitzMatrixVecMul,
    /// FK20 allows you to create a proof of an opening for multiple points.
    /// Each proof will attest to the opening of `l` points.
    /// In the FK20 paper, this is also referred to as `l` (ELL).
    ///
    /// TODO(Note): This has ramifications for the number of G2 points, but it is not checked
    /// TODO: in the constructor here.
    point_set_size: usize,
    /// The total number of points that we want to open a polynomial at.
    ///
    /// Note: A proof will attest to `point_set_size` of these points at a
    /// time.
    number_of_points_to_open: usize,
    /// Domain used in FK20 to create the opening proofs
    proof_domain: Domain,
    ext_domain: Domain,
    /// Domain used for converting polynomial to monomial form.
    poly_domain: Domain,
    /// Commitment key used for committing to the polynomial
    /// in monomial form.
    commit_key: CommitKey,
}

impl FK20 {
    pub fn new(
        commit_key: CommitKey,
        polynomial_bound: usize,
        point_set_size: usize,
        number_of_points_to_open: usize,
    ) -> FK20 {
        assert!(point_set_size.is_power_of_two());
        assert!(number_of_points_to_open.is_power_of_two());
        assert!(number_of_points_to_open > point_set_size);
        assert!(polynomial_bound.is_power_of_two());
        assert!(commit_key.g1s.len() >= polynomial_bound);

        // 1. Compute the SRS vectors that we will multiply the toeplitz matrices by.
        //
        // Skip the last `l` points in the srs
        assert!(commit_key.g1s.len() > point_set_size);
        let srs_truncated: Vec<_> = commit_key
            .g1s
            .clone()
            .into_iter()
            .rev()
            .skip(point_set_size)
            .collect();
        let mut srs_vectors = take_every_nth(&srs_truncated, point_set_size);

        // Pad srs vectors to the next power of two
        //
        // This is not strictly needed since our FFT implementation
        // will pad these. However, doing it now saves work.
        for srs_vector in &mut srs_vectors {
            let pad_by = srs_vector.len().next_power_of_two();
            srs_vector.resize(pad_by, G1Projective::identity());
        }

        // Compute `l` toeplitz matrix-vector multiplications and sum them together
        let batch_toeplitz = BatchToeplitzMatrixVecMul::new(srs_vectors);

        // 2. Compute the domains needed to produce the proofs and the evaluations
        //
        // The size of the proof domain corresponds to the number of proofs that will be returned.
        let proof_domain = Domain::new(number_of_points_to_open / point_set_size);
        // The size of the extension domain corresponds to the number of points that we want to open
        let ext_domain = Domain::new(number_of_points_to_open);
        // The domain needed to convert the polynomial from lagrange form to monomial form.
        let poly_domain = Domain::new(polynomial_bound);

        FK20 {
            batch_toeplitz,
            point_set_size,
            number_of_points_to_open,
            proof_domain,
            ext_domain,
            poly_domain,
            commit_key,
        }
    }

    /// Commit to the data that we will be creating FK20 proofs over.
    pub fn commit_to_data(&self, mut data: Vec<Scalar>) -> G1Point {
        // Reverse the order of the scalars, so that they are in bit-reversed order.
        //
        // FK20 will operate over the bit-reversed permutation of the data.
        reverse_bit_order(&mut data);

        let poly_coeff = self.poly_domain.ifft_scalars(data);

        // Commit to the bit reversed data in lagrange form using the lagrange version of the commit key
        self.commit_key.commit_g1(&poly_coeff).into()
    }

    /// Given a group of coset evaluations, this method will return/reorder the evaluations as if
    /// we evaluated them on the relevant extended domain. The coset indices in domain order
    /// will also be returned.
    //
    // For evaluations that are missing, this method will fill these in with zeroes.
    //
    // Note: It is the callers responsibility to ensure that there are no duplicate
    // coset indices.
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

    /// The number of proofs that will be produced.
    pub fn num_proofs(&self) -> usize {
        self.number_of_points_to_open / self.point_set_size
    }

    pub fn compute_multi_opening_proofs_poly_coeff(
        &self,
        polynomial: PolyCoeff,
    ) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        // Compute proofs for the polynomial
        let h_poly_commitments =
            self.compute_h_poly_commitments(polynomial.clone(), self.point_set_size);
        let mut proofs = self.proof_domain.fft_g1(h_poly_commitments);

        // apply reverse bit order permutation, since fft_g1 was applied using
        // the regular order, and we want the cosets to be in bit-reversed order
        //
        // TODO: Add note about making the cosets line up for the evaluation sets
        reverse_bit_order(&mut proofs);

        let proofs_affine = g1_batch_normalize(&proofs);

        let evaluation_sets = self.compute_evaluation_sets(polynomial);

        (proofs_affine, evaluation_sets)
    }

    pub fn compute_multi_opening_proofs_on_data(
        &self,
        mut data: Vec<Scalar>,
    ) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        reverse_bit_order(&mut data);
        let poly_coeff = self.poly_domain.ifft_scalars(data);

        self.compute_multi_opening_proofs_poly_coeff(poly_coeff)
    }

    // TODO: evaluation_sets might not be the best name here.
    // TODO: It is a Vector/list of coset evaluations
    fn compute_evaluation_sets(&self, polynomial: PolyCoeff) -> Vec<Vec<Scalar>> {
        // Compute the evaluations of the polynomial on the cosets by doing an fft
        let mut evaluations = self.ext_domain.fft_scalars(polynomial);
        reverse_bit_order(&mut evaluations);
        evaluations
            .chunks_exact(self.point_set_size)
            .map(|slice| slice.to_vec())
            .collect()
    }
}
