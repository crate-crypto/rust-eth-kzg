use crate::{
    fk20::cosets::{coset_gens, reverse_bit_order},
    verification_key::VerificationKey,
};
use bls12_381::{
    ff::Field, g1_batch_normalize, lincomb::g1_lincomb, multi_pairings,
    reduce_bytes_to_scalar_bias, G1Point, G2Point, G2Prepared, Scalar,
};
use polynomial::{domain::Domain, poly_coeff::poly_add, CosetFFT};
use sha2::{Digest, Sha256};
use std::mem::size_of;

use super::errors::VerifierError;

/// CosetIndex is a reference to a coset.
///
/// Note: We are able to use CosetIndex instead of the coset because
/// the prover and verifier both know what the cosets are that
/// we will be making and verifying opening proofs for.
pub type CosetIndex = u64;

/// CommitmentIndex is a reference to a commitment.
///
/// In order to make verification cheaper, the verifier will
/// deduplicate the list of commitments that are needed in order to verify opening proofs.
/// They will then refer to a commitment via its position in an array of deduplicated commitments
/// with the CommitmentIndex.
pub type CommitmentIndex = u64;

/// FK20Verifier initializes all of the components needed to verify KZG multi point
/// proofs that were created using the FK20Prover.
///
/// Note: The proofs can be created naively not using the FK20 strategy (see naive.rs)
/// however, we put FK20 in the name since:
///  - From the callers perspective, this distinction is not important
///  - We only use FK20 to create proofs
#[derive(Debug)]
pub struct FK20Verifier {
    pub verification_key: VerificationKey,
    pub coset_gens_bit_reversed: Vec<Scalar>,
    coset_domain: Domain,
    // Pre-computations for the verification algorithm
    //
    // [tau^n]_2
    tau_pow_n: G2Prepared,
    // [-1]_2
    neg_g2_gen: G2Prepared,
    // Bit reversed vector of the coset generators raised
    // to the power of `n`, needed to verify a multi opening proof.
    pub bit_reversed_coset_gens_pow_n: Vec<Scalar>,
    // Bit reversed vector of the coset generators that
    // we will use to compute an inverse coset IFFT.
    bit_reversed_coset_fft_gens: Vec<CosetFFT>,
}

impl FK20Verifier {
    pub fn new(
        verification_key: VerificationKey,
        num_points_to_open: usize,
        num_cosets: usize,
    ) -> Self {
        const BIT_REVERSED: bool = true;
        let coset_gens = coset_gens(num_points_to_open, num_cosets, BIT_REVERSED);

        let coset_size = num_points_to_open / num_cosets;
        assert!(
            verification_key.g2s.len() >= coset_size,
            "need as many g2 points as coset size"
        );
        let coset_domain = polynomial::domain::Domain::new(verification_key.coset_size);

        let n = verification_key.coset_size;
        // [tau^n]_2
        let tau_pow_n = G2Prepared::from(G2Point::from(verification_key.g2s[n]));
        // [-1]_2
        let neg_g2_gen = G2Prepared::from(-verification_key.g2_gen());

        let coset_gens_pow_n = coset_gens
            .iter()
            .map(|&coset_gen| coset_gen.pow_vartime([n as u64]))
            .collect();

        let coset_fft_gens: Vec<_> = coset_gens.iter().map(|gen| CosetFFT::new(*gen)).collect();

        Self {
            verification_key,
            coset_gens_bit_reversed: coset_gens,
            coset_domain,
            tau_pow_n,
            neg_g2_gen,
            bit_reversed_coset_gens_pow_n: coset_gens_pow_n,
            bit_reversed_coset_fft_gens: coset_fft_gens,
        }
    }

    /// Verify multiple multi-opening proofs.
    ///
    /// Panics if the following slices do not have the same length:
    ///
    /// - commitment_indices
    /// - bit_reversed_coset_indices
    /// - bit_reversed_coset_evals
    /// - bit_reversed_proofs
    ///
    /// This corresponds to the guarantee that every opening should have an `input_point` and an `output_point`
    /// with a corresponding proof attesting to `f(input_point) = output_point` and a commitment to the polynomial `f`.  
    ///
    /// Note: Although this method is on the `FK20Verifier` structure, it is possible to verify methods that are not
    /// created by the `FK20Prover`. FK20Prover generates multi-proofs efficiently using the FK20 strategy, but we
    /// could just as well generate those proofs using the naive strategy that we test FK20 against. We leave this
    /// naming as is since it conveys a meaningful difference between the other internal provers which do not use FK20.
    /// On the API level, we however export this as Verifier.
    ///
    /// The matching function in the spec is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#verify_cell_kzg_proof_batch_impl
    pub fn verify_multi_opening(
        &self,

        deduplicated_commitments: &[G1Point],
        commitment_indices: &[CommitmentIndex],

        bit_reversed_coset_indices: &[CosetIndex],
        bit_reversed_coset_evals: &[Vec<Scalar>],
        bit_reversed_proofs: &[G1Point],
    ) -> Result<(), VerifierError> {
        assert_eq!(
            commitment_indices.len(),
            bit_reversed_proofs.len(),
            "Expected to have a proof for each commitment opening"
        );
        assert_eq!(
            bit_reversed_coset_indices.len(),
            bit_reversed_proofs.len(),
            "Expected to have a proof for each index we want to open at"
        );
        assert_eq!(
            bit_reversed_coset_evals.len(),
            bit_reversed_proofs.len(),
            "Expected to have a proof for each evaluation we want to prove an opening for"
        );
        // The batch size corresponds to how many openings, we ultimately want to be verifying.
        let batch_size = bit_reversed_coset_indices.len();

        // 1. Compute random challenges for batching the opening together.
        //
        // From hereon out, `random` will refer to using these random challenges.
        //
        // We compute one challenge `r` using fiat-shamir and the rest are powers of `r`
        // This is safe because of the Schwartz-Zippel Lemma.
        let r = compute_fiat_shamir_challenge(
            &self.verification_key,
            deduplicated_commitments,
            commitment_indices,
            bit_reversed_coset_indices,
            bit_reversed_coset_evals,
            bit_reversed_proofs,
        );
        let r_powers = compute_powers(r, batch_size);
        let num_unique_commitments = deduplicated_commitments.len();

        // 2. Compute a random linear combination of the proofs
        //
        // Safety: This unwrap can never trigger because `r_powers.len()` is `batch_size`
        // and `bit_reversed_proofs.len()` will equal `batch_size` since we must have a proof for each item in the batch.
        let comm_random_sum_proofs = g1_lincomb(bit_reversed_proofs, &r_powers)
            .expect("number of proofs and number of r_powers should be the same");

        // 3. Compute a weighted random linear combination of the proofs
        //
        // Where the `weight` refers to the coset_generators to the power of `n`
        let mut weighted_r_powers = Vec::with_capacity(batch_size);
        for (bit_reversed_coset_index, r_power) in bit_reversed_coset_indices.iter().zip(&r_powers)
        {
            let coset_gen_pow_n =
                self.bit_reversed_coset_gens_pow_n[*bit_reversed_coset_index as usize];
            weighted_r_powers.push(r_power * coset_gen_pow_n);
        }
        // Safety: This should never panic since `bit_reversed_proofs.len()` is equal to the batch_size.
        let random_weighted_sum_proofs = g1_lincomb(bit_reversed_proofs, &weighted_r_powers)
            .expect("number of proofs and number of weighted_r_powers should be the same");

        // 4. Compute a random linear combination of the commitments
        //
        // For each commitment_index/commitment, we add its contribution of `r` to
        // the associated weight for that commitment.
        //
        // This is essentially taking advantage of the fact that commitments may be
        // duplicated., so instead of calculating C = r_0 * C_0 + r_1 * C_0 naively
        // which would require a size 2 MSM, we compute C = (r_0 + r_1) * C_0
        // which would require a size 1 MSM and some extra field additions.
        //
        // One can view this as trading a scalar multiplication for a field addition.
        //
        // The extra field additions are being calculated in the for loop below.
        let mut weights = vec![Scalar::ZERO; num_unique_commitments];
        for (commitment_index, r_power) in commitment_indices.iter().zip(&r_powers) {
            weights[*commitment_index as usize] += r_power;
        }
        // Safety: This unwrap will never trigger because the length of `weights` has been initialized
        // to be `deduplicated_commitments.len()`.
        //
        // This only panics, if `deduplicated_commitments.len()` != `weights.len()`
        let random_sum_commitments = g1_lincomb(deduplicated_commitments, &weights)
            .expect("number of row_commitments and number of weights should be the same");

        // 5. Compute random linear combination of the interpolation polynomials
        let random_sum_interpolation_poly = compute_sum_interpolation_poly(
            &self.coset_domain,
            &self.bit_reversed_coset_fft_gens,
            &bit_reversed_coset_evals,
            &bit_reversed_coset_indices,
            &r_powers,
        );
        let comm_random_sum_interpolation_poly = self
            .verification_key
            .commit_g1(&random_sum_interpolation_poly);

        // 6. Compute pairing check
        //
        // Note: This variable is `rl` in the specs.
        let pairing_input_g1 = (random_sum_commitments - comm_random_sum_interpolation_poly)
            + random_weighted_sum_proofs;

        // The pairings function requires elements in affine representation, so we must batch normalize the
        // pairing inputs.
        let normalized_vectors = g1_batch_normalize(&[comm_random_sum_proofs, pairing_input_g1]);
        let random_sum_proofs = normalized_vectors[0];
        let pairing_input_g1 = normalized_vectors[1];

        let proof_valid = multi_pairings(&[
            (&random_sum_proofs, &self.tau_pow_n),
            (&pairing_input_g1, &self.neg_g2_gen),
        ]);
        if proof_valid {
            Ok(())
        } else {
            Err(VerifierError::InvalidProof)
        }
    }
}

/// Computes a random challenge which will allow us to efficiently verify multiple opening proofs.
///
/// Efficiently refers to being able to verify these proofs faster than verifying each proof individually.
///
/// The matching function in the spec is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#compute_verify_cell_kzg_proof_batch_challenge
#[allow(clippy::manual_slice_size_calculation)]
fn compute_fiat_shamir_challenge(
    verification_key: &VerificationKey,
    row_commitments: &[G1Point],
    row_indices: &[u64],
    coset_indices: &[u64],
    coset_evals: &[Vec<Scalar>],
    proofs: &[G1Point],
) -> Scalar {
    const DOMAIN_SEP: &str = "RCKZGCBATCH__V1_";
    let hash_input_size = DOMAIN_SEP.as_bytes().len()
            + size_of::<u64>() // polynomial bound
            + size_of::<u64>() // field elements per coset
            + size_of::<u64>() // num commitments
            + size_of::<u64>() // num cosets
            + row_commitments.len() * G1Point::compressed_size()
            + row_indices.len() * size_of::<u64>()
            + coset_indices.len() * size_of::<u64>()
            + coset_evals.len() * verification_key.coset_size * size_of::<Scalar>()
            + proofs.len() * G1Point::compressed_size();

    let mut hash_input: Vec<u8> = Vec::with_capacity(hash_input_size);

    hash_input.extend(DOMAIN_SEP.as_bytes());
    hash_input.extend((verification_key.num_coefficients_in_polynomial as u64).to_be_bytes());
    hash_input.extend((verification_key.coset_size as u64).to_be_bytes());

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

    assert_eq!(hash_input.len(), hash_input_size);
    let mut hasher = Sha256::new();
    hasher.update(hash_input);
    let result: [u8; 32] = hasher.finalize().into();

    // For randomization, we only need a 128 bit scalar, since this is used for batch verification.
    // See for example, the randomizers section in : https://cr.yp.to/badbatch/badbatch-20120919.pdf
    //
    // This is noted because when we convert a 256 bit hash to a scalar, a bias will be introduced.
    // This however does not affect our security guarantees because the bias is negligible given we
    // want a uniformly random 128 bit integer.
    //
    // Also there is a negligible probably that the scalar is zero, so we do not handle this case here.
    reduce_bytes_to_scalar_bias(result)
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

/// Computes `k` Interpolation polynomials and then combines
/// them linearly using `k` values from `r_powers`.
/// The computed value is I(X) = I_0(x) + r * I_1(x) + ... + r^{n-1} * I_{n-1}(x)
fn compute_sum_interpolation_poly(
    coset_domain: &Domain,
    bit_reversed_coset_fft_gens: &[CosetFFT],
    bit_reversed_coset_evals: &[Vec<Scalar>],
    bit_reversed_coset_indices: &[CosetIndex],
    r_powers: &[Scalar],
) -> Vec<Scalar> {
    let mut random_sum_interpolation_poly = Vec::new();

    for ((mut bit_reversed_coset_eval, bit_reversed_coset_index), scale_factor) in
        bit_reversed_coset_evals
            .to_vec()
            .into_iter()
            .zip(bit_reversed_coset_indices)
            .zip(r_powers)
    {
        // Reverse the order, so it matches the fft domain
        reverse_bit_order(&mut bit_reversed_coset_eval);
        let coset_eval = bit_reversed_coset_eval; // variable rename since we un-bit reversed the vector

        // Compute the interpolation polynomial using a coset fft
        let coset_gen = &bit_reversed_coset_fft_gens[*bit_reversed_coset_index as usize];
        let ifft_scalars = coset_domain.coset_ifft_scalars(coset_eval, coset_gen);

        // Scale the interpolation polynomial by the challenge
        let scaled_interpolation_poly = ifft_scalars
            .into_iter()
            .map(|coeff| coeff * scale_factor)
            .collect::<Vec<_>>();

        random_sum_interpolation_poly =
            poly_add(random_sum_interpolation_poly, scaled_interpolation_poly);
    }

    random_sum_interpolation_poly
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
