use super::coset_gens;
use crate::{fk20::cosets::reverse_bit_order, opening_key::OpeningKey};
use bls12_381::{
    batch_inversion::batch_inverse, multi_pairings, G1Point, G2Point, G2Prepared, Scalar,
};
use polynomial::monomial::poly_add;

#[derive(Debug)]
pub struct FK20Verifier {
    pub opening_key: OpeningKey,
    pub coset_shifts: Vec<Scalar>,
}

impl FK20Verifier {
    pub fn new(
        opening_key: OpeningKey,
        num_points_to_open: usize,
        coset_size: usize,
        bit_reversed: bool,
    ) -> Self {
        let coset_shifts = coset_gens(num_points_to_open, coset_size, bit_reversed);
        Self {
            opening_key,
            coset_shifts,
        }
    }

    pub fn verify_multi_opening(
        &self,
        row_commitments: &[G1Point],
        commitment_indices: &[u64],
        coset_indices: &[u64],
        coset_evals: &[Vec<Scalar>],
        proofs: &[G1Point],
    ) -> bool {
        verify_multi_opening(
            &self.opening_key,
            row_commitments,
            commitment_indices,
            coset_indices,
            &self.coset_shifts,
            coset_evals,
            proofs,
        )
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
    let mut hash_input: Vec<u8> = Vec::new();

    // Domain separation
    hash_input.extend(DOMAIN_SEP.as_bytes());

    // polynomial bound
    hash_input.extend((opening_key.num_coefficients_in_polynomial as u64).to_be_bytes());

    // field elements per coset
    hash_input.extend((opening_key.multi_opening_size as u64).to_be_bytes());

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
    use bls12_381::ff::Field;

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(hash_input);
    let mut result: [u8; 32] = hasher.finalize().into();

    // For randomization, we only need a 128 bit scalar, since this is used for batch verification.
    // See for example, the randomizers section in : https://cr.yp.to/badbatch/badbatch-20120919.pdf
    //
    // This is noted because when we truncate the 256 bit hash into a scalar,
    // a bias will be introduced. This however does not affect our security guarantees
    // because the bias is negligible given we want a uniformly random 128 bit integer.
    //
    // So that we know it fits into a scalar, we shave off 2 bits.
    result[0] = (result[0] << 2) >> 2;
    let scalar = Scalar::from_bytes_be(&result)
        .expect("254 bit integer should have been reducible to a scalar");

    // TODO: Could remove this, since it is statistically improbable
    // TODO: we add 1 to the scalar, so that it can never be 0
    // TODO: This is also taken from: https://cr.yp.to/badbatch/badbatch-20120919.pdf
    scalar + Scalar::ONE
}

fn compute_powers(value: Scalar, num_elements: usize) -> Vec<Scalar> {
    use bls12_381::ff::Field;

    let mut powers = Vec::new();
    let mut current_power = Scalar::ONE;

    for _ in 0..num_elements {
        powers.push(current_power);
        current_power *= value;
    }

    powers
}

pub(crate) fn verify_multi_opening(
    opening_key: &OpeningKey,
    row_commitments: &[G1Point],
    commitment_indices: &[u64],
    coset_indices: &[u64],
    coset_shifts: &[Scalar],
    coset_evals: &[Vec<Scalar>],
    proofs: &[G1Point],
) -> bool {
    use bls12_381::ff::Field;

    // Compute random challenges for batching the opening together.
    //
    // We compute one challenge `r` using fiat-shamir and the rest are powers of `r`
    // This is safe since 1, X, X^2, ..., X^n of a variable X are linearly independent (ie there is no non-trivial linear combination that equals zero)
    //
    // TODO: Because this method takes in G1Points and not their serialized form, there is a roundtrip that happens
    // TODO: when we serialize the point for fiat shamir. (I'm leaving this TOOD here until we benchmark the diff)
    let r = compute_fiat_shamir_challenge(
        opening_key,
        row_commitments,
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
    let row_commitments = row_commitments
        .iter()
        .map(bls12_381::G1Projective::from)
        .collect::<Vec<_>>();

    let num_cosets = coset_indices.len();
    let n = opening_key.multi_opening_size;
    let num_unique_commitments = row_commitments.len();

    // First compute a random linear combination of the proofs
    let random_sum_proofs = bls12_381::lincomb::g1_lincomb(&proofs, &r_powers)
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
    let random_sum_commitments = bls12_381::lincomb::g1_lincomb(&row_commitments, &weights)
        .expect("number of row_commitments and number of weights should be the same");

    let domain = polynomial::domain::Domain::new(opening_key.multi_opening_size);

    // Compute a random linear combination of the interpolation polynomials
    let mut sum_interpolation_poly = Vec::new();
    for k in 0..num_cosets {
        let mut coset_evals_clone = coset_evals[k].clone();
        reverse_bit_order(&mut coset_evals_clone);

        // Compute the interpolation polynomial
        let ifft_scalars = domain.ifft_scalars(coset_evals_clone);
        let h_k = coset_shifts[coset_indices[k] as usize];
        let mut inv_h_k_powers = compute_powers(h_k, ifft_scalars.len());
        batch_inverse(&mut inv_h_k_powers); // The coset generators are all roots of unity, so none of them will be zero
        let ifft_scalars: Vec<_> = ifft_scalars
            .into_iter()
            .zip(inv_h_k_powers)
            .map(|(scalar, inv_h_k_pow)| scalar * inv_h_k_pow)
            .collect();

        let scale_factor = r_powers[k];
        let r_x = ifft_scalars
            .into_iter()
            .map(|coeff| coeff * scale_factor)
            .collect::<Vec<_>>();

        sum_interpolation_poly = poly_add(sum_interpolation_poly, r_x);
    }
    let random_sum_interpolation = opening_key.commit_g1(&sum_interpolation_poly);

    // [s^n]
    let s_pow_n = opening_key.g2s[n];

    let mut weighted_r_powers = Vec::with_capacity(num_cosets);
    for k in 0..num_cosets {
        // This is expensive and does not need to be done all the time.
        let h_k = coset_shifts[coset_indices[k] as usize];
        let h_k_pow = h_k.pow_vartime([n as u64]);
        let wrp = r_powers[k] * h_k_pow;
        weighted_r_powers.push(wrp);
    }
    let random_weighted_sum_proofs = bls12_381::lincomb::g1_lincomb(&proofs, &weighted_r_powers)
        .expect("number of proofs and number of weighted_r_powers should be the same");

    // TODO: Find a better name for this
    let rl = (random_sum_commitments - random_sum_interpolation) + random_weighted_sum_proofs;

    // TODO: These .into are a bit expensive since they are converting from projective to affine

    let s_pow_n: G2Point = s_pow_n.into(); // TODO: This can be precomputed and stored in the opening-key
    multi_pairings(&[
        (&random_sum_proofs.into(), &G2Prepared::from(s_pow_n)),
        (&rl.into(), &G2Prepared::from(-opening_key.g2_gen())), // TODO: We can precompute and store `G2Prepared::from(-opening_key.g2_gen())` in the openingkey too
    ])
}
