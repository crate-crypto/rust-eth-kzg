use crate::{
    fk20::cosets::{coset_gens, reverse_bit_order},
    opening_key::OpeningKey,
};
use bls12_381::{
    batch_inversion::batch_inverse, ff::Field, g1_batch_normalize, lincomb::g1_lincomb,
    multi_pairings, reduce_bytes_to_scalar_bias, G1Point, G2Point, G2Prepared, Scalar,
};
use polynomial::{domain::Domain, monomial::poly_add};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct FK20Verifier {
    pub opening_key: OpeningKey,
    // These are bit-reversed.
    pub coset_shifts: Vec<Scalar>,
    coset_domain: Domain,
    // Pre-computations for the verification algorithm
    //
    // [s^n]_2
    s_pow_n: G2Prepared,
    // [-1]_2
    neg_g2_gen: G2Prepared,
    //
    pub coset_shifts_pow_n: Vec<Scalar>,
    //
    inv_coset_shifts_pow_n: Vec<Vec<Scalar>>,
}

impl FK20Verifier {
    pub fn new(opening_key: OpeningKey, num_points_to_open: usize, num_cosets: usize) -> Self {
        const BIT_REVERSED: bool = true;
        let coset_shifts = coset_gens(num_points_to_open, num_cosets, BIT_REVERSED);

        let coset_size = num_points_to_open / num_cosets;
        assert!(
            opening_key.g2s.len() >= coset_size,
            "need as many g2 points as coset size"
        );
        let coset_domain = polynomial::domain::Domain::new(opening_key.coset_size);

        let n = opening_key.coset_size;
        // [s^n]_2
        let s_pow_n = G2Prepared::from(G2Point::from(opening_key.g2s[n]));
        // [-1]_2
        let neg_g2_gen = G2Prepared::from(-opening_key.g2_gen());

        let coset_shifts_pow_n = coset_shifts
            .iter()
            .map(|&coset_shift| coset_shift.pow_vartime([n as u64]))
            .collect();

        // TODO: We might be able to remove this if we modify the API for fft to take arbitrary cosets
        let inv_coset_shifts_pow_n: Vec<_> = coset_shifts
            .iter()
            .map(|&coset_shift| {
                let mut inv_coset_shift_powers = compute_powers(coset_shift, n);
                batch_inverse(&mut inv_coset_shift_powers); // The coset generators are all roots of unity, so none of them will be zero
                inv_coset_shift_powers
            })
            .collect();

        Self {
            opening_key,
            coset_shifts,
            coset_domain,
            s_pow_n,
            neg_g2_gen,
            coset_shifts_pow_n,
            inv_coset_shifts_pow_n,
        }
    }

    /// Verify multiple multi-opening proofs.
    ///
    /// The matching function in the spec is: https://github.com/ethereum/consensus-specs/blob/b9e7b031b5f2c18d76143007ea779a32b5505155/specs/_features/eip7594/polynomial-commitments-sampling.md#verify_cell_kzg_proof_batch_impl
    pub fn verify_multi_opening(
        &self,

        deduplicated_commitments: &[G1Point],
        commitment_indices: &[u64],

        // These are bit-reversed.
        coset_indices: &[u64],
        // These are bit-reversed.
        coset_evals: &[Vec<Scalar>],
        // These are bit-reversed.
        proofs: &[G1Point],
    ) -> bool {
        // Compute random challenges for batching the opening together.
        //
        // We compute one challenge `r` using fiat-shamir and the rest are powers of `r`
        // This is safe since 1, X, X^2, ..., X^n of a variable X are linearly independent (ie there is no non-trivial linear combination that equals zero)
        //
        // TODO: Because this method takes in G1Points and not their serialized form, there is a roundtrip that happens
        // TODO: when we serialize the point for fiat shamir. (I'm leaving this TOOD here until we benchmark the diff)
        let r = compute_fiat_shamir_challenge(
            &self.opening_key,
            deduplicated_commitments,
            commitment_indices,
            coset_indices,
            coset_evals,
            proofs,
        );
        let r_powers = compute_powers(r, commitment_indices.len());

        // Convert the proofs to Projective form.
        // This is essentially free and we are mainly paying for the allocation cost here.
        let proofs = proofs
            .iter()
            .map(bls12_381::G1Projective::from)
            .collect::<Vec<_>>();
        let deduplicated_commitments = deduplicated_commitments
            .iter()
            .map(bls12_381::G1Projective::from)
            .collect::<Vec<_>>();

        let num_cosets = coset_indices.len();
        let num_unique_commitments = deduplicated_commitments.len();

        // First compute a random linear combination of the proofs
        let comm_random_sum_proofs = g1_lincomb(&proofs, &r_powers)
            .expect("number of proofs and number of r_powers should be the same");

        // Now compute a random linear combination of the commitments
        //
        // We know that many of the commitments are duplicated, so we optimize for this
        // use case.
        //
        // For example, imagine we wanted to do r_1 * G_1 + r_2 * G_1
        // This would be equivalent to doing (r_1 + r_2) * G_1
        // The (r_1 + r_2) is what is being referred to as the `weight`
        let mut weights = vec![Scalar::from(0); num_unique_commitments];
        for k in 0..num_cosets {
            // For each row index, we get its commitment index `i`.
            // ie, `i` just means we are looking at G_i
            let commitment_index = commitment_indices[k];
            // We then add the contribution of `r` as a part of that commitments weight.
            weights[commitment_index as usize] += r_powers[k];
        }
        let comm_random_sum_commitments = g1_lincomb(&deduplicated_commitments, &weights)
            .expect("number of row_commitments and number of weights should be the same");

        // Compute a random linear combination of the interpolation polynomials
        let mut random_sum_interpolation_poly = Vec::new();
        let coset_evals = coset_evals.to_vec();
        for (k, mut coset_eval) in coset_evals.into_iter().enumerate() {
            // Reverse the order, so it matches the fft domain
            reverse_bit_order(&mut coset_eval);

            // Compute the interpolation polynomial
            let ifft_scalars = self.coset_domain.ifft_scalars(coset_eval);
            let inv_coset_shift_pow_n = &self.inv_coset_shifts_pow_n[coset_indices[k] as usize];
            let ifft_scalars: Vec<_> = ifft_scalars
                .into_iter()
                .zip(inv_coset_shift_pow_n)
                .map(|(scalar, inv_h_k_pow)| scalar * inv_h_k_pow)
                .collect();

            // Scale the interpolation polynomial by the challenge
            let scale_factor = r_powers[k];
            let scaled_interpolation_poly = ifft_scalars
                .into_iter()
                .map(|coeff| coeff * scale_factor)
                .collect::<Vec<_>>();

            random_sum_interpolation_poly =
                poly_add(random_sum_interpolation_poly, scaled_interpolation_poly);
        }
        let comm_random_sum_interpolation_poly =
            self.opening_key.commit_g1(&random_sum_interpolation_poly);

        let mut weighted_r_powers = Vec::with_capacity(num_cosets);
        for (coset_index, r_power) in coset_indices.into_iter().zip(r_powers) {
            let coset_shift_pow_n = self.coset_shifts_pow_n[*coset_index as usize];
            weighted_r_powers.push(r_power * coset_shift_pow_n);
        }
        let random_weighted_sum_proofs = g1_lincomb(&proofs, &weighted_r_powers)
            .expect("number of proofs and number of weighted_r_powers should be the same");

        // TODO: Find a better name for this (use it from specs)
        let rl = (comm_random_sum_commitments - comm_random_sum_interpolation_poly)
            + random_weighted_sum_proofs;

        let normalized_vectors = g1_batch_normalize(&[comm_random_sum_proofs, rl]);
        let random_sum_proofs = normalized_vectors[0];
        let rl = normalized_vectors[1];

        multi_pairings(&[(&random_sum_proofs, &self.s_pow_n), (&rl, &self.neg_g2_gen)])
    }
}

fn compute_fiat_shamir_challenge(
    opening_key: &OpeningKey,
    row_commitments: &[G1Point],
    row_indices: &[u64],
    coset_indices: &[u64],
    coset_evals: &[Vec<Scalar>],
    proofs: &[G1Point],
) -> Scalar {
    const DOMAIN_SEP: &str = "RCKZGCBATCH__V1_";
    let mut hash_input: Vec<u8> = Vec::with_capacity(
        DOMAIN_SEP.as_bytes().len()
            + row_commitments.len() * 48
            + (row_indices.len() + coset_indices.len()) * 8
            + (coset_evals.len() * opening_key.coset_size) * 32
            + proofs.len() * 48,
    ); // TODO: this capacity is not exact and the magic numbers here are not great, lets refactor and benchmark this

    // Domain separation
    hash_input.extend(DOMAIN_SEP.as_bytes());

    // polynomial bound
    hash_input.extend((opening_key.num_coefficients_in_polynomial as u64).to_be_bytes());

    // field elements per coset
    hash_input.extend((opening_key.coset_size as u64).to_be_bytes());

    let num_commitments = row_commitments.len() as u64;
    hash_input.extend(num_commitments.to_be_bytes());

    let num_cosets = coset_indices.len() as u64;
    hash_input.extend(num_cosets.to_be_bytes());

    for commitment in row_commitments {
        hash_input.extend(commitment.to_compressed())
    }

    for k in 0..num_cosets {
        hash_input.extend(row_indices[k as usize].to_be_bytes());
        hash_input.extend(coset_indices[k as usize].to_be_bytes());
        for eval in &coset_evals[k as usize] {
            hash_input.extend(eval.to_bytes_be())
        }
        hash_input.extend(proofs[k as usize].to_compressed())
    }

    let mut hasher = Sha256::new();
    hasher.update(hash_input);
    let result: [u8; 32] = hasher.finalize().into();

    // For randomization, we only need a 128 bit scalar, since this is used for batch verification.
    // See for example, the randomizers section in : https://cr.yp.to/badbatch/badbatch-20120919.pdf
    //
    // This is noted because when we convert a 256 bit hash to a scalar, a bias will be introduced.
    // This however does not affect our security guarantees because the bias is negligible given we
    // want a uniformly random 128 bit integer.
    let scalar = reduce_bytes_to_scalar_bias(result);

    // TODO: computing powers will remove the 128 bit structure, consider generating `n` 128 bit scalars
    // There is a negligible probably that the scalar is zero, so we do not handle this case here.
    scalar
}

/// Computes a vector of powers of a given scalar value.
///
/// Example: compute_powers(x, 5) = [1, x, x^2, x^3, x^4]
fn compute_powers(value: Scalar, num_elements: usize) -> Vec<Scalar> {
    use bls12_381::ff::Field;

    let mut powers = Vec::with_capacity(num_elements);
    let mut current_power = Scalar::ONE;

    for _ in 0..num_elements {
        powers.push(current_power);
        current_power *= value;
    }

    powers
}

#[cfg(test)]
mod tests {
    use super::*;
    use bls12_381::Scalar;

    #[test]
    fn test_compute_powers() {
        let base = Scalar::from(2u64);
        let num_elements = 5;

        let powers = compute_powers(base, num_elements);

        assert_eq!(powers.len(), num_elements);
        assert_eq!(powers[0], Scalar::ONE);
        assert_eq!(powers[1], base);
        assert_eq!(powers[2], base.pow_vartime(&[2]));
        assert_eq!(powers[3], base.pow_vartime(&[3]));
        assert_eq!(powers[4], base.pow_vartime(&[4]));

        let powers = compute_powers(base, 0);
        assert!(powers.is_empty());
    }
}
