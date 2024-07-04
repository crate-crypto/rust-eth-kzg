
use bls12_381::{batch_inversion::batch_inverse, multi_pairings, G1Point, G1Projective, G2Point, G2Prepared, Scalar};
use polynomial::monomial::{lagrange_interpolate, poly_add, poly_eval, poly_sub, vanishing_poly, PolyCoeff};

use crate::{commit_key::CommitKey, opening_key::OpeningKey, reverse_bit_order};

/// A proof that a polynomial was opened at multiple points.
///
/// This creates a KZG proof as noted in [BDF21](https://eprint.iacr.org/2020/081.pdf)
/// using the techniques from [FK20](https://github.com/khovratovich/Kate/blob/master/Kate_amortized.pdf)
/// since the points being opened are roots of unity.
pub struct Proof {
    /// Commitment to the `witness` or quotient polynomial
    quotient_commitment: G1Point,
    /// Evaluation of the polynomial at the input points.
    ///
    /// This implementation is only concerned with the case where the input points are roots of unity.
    output_points: Vec<Scalar>,
}

impl Proof {
    // TODO(Note): It would be great if we could make this method take
    // TODO: a commitment to the polynomial too. This would generalize
    // TODO: quite nicely to multipoint reduction arguments that need to
    // TODO: to use randomness since they need to hash the commitment.
    pub fn compute(
        commit_key: &CommitKey,
        polynomial: &PolyCoeff,
        input_points: &[Scalar],
    ) -> Proof {
        let (quotient_commitment, output_points) =
            compute_multi_opening_naive(commit_key, polynomial, input_points);

        Proof {
            quotient_commitment,
            output_points,
        }
    }
    /// Verifies a multi-point opening proof.
    /// TODO: We may want to return a Result here so that errors can
    /// TODO be more descriptive.
    pub fn verify(
        &self,
        opening_key: &OpeningKey,
        commitment: G1Point,
        input_points: &[Scalar],
    ) -> bool {
        verify_multi_opening_naive(
            opening_key,
            commitment,
            self.quotient_commitment,
            input_points,
            &self.output_points,
        )
    }
}

/// Verifies a multi-opening proof using the general formula.
///
/// Note: This copies the exact implementation that the consensus-specs uses.
pub fn verify_multi_opening_naive(
    opening_key: &OpeningKey,
    commitment: G1Point,
    proof: G1Point,
    input_points: &[Scalar],
    output_points: &[Scalar],
) -> bool {
    // e([Commitment] - [I(x)], [1]) == e([Q(x)], [Z(X)])

    let coordinates: Vec<_> = input_points
        .iter()
        .zip(output_points.iter())
        .map(|(p, e)| (*p, *e))
        .collect();
    let r_x = lagrange_interpolate(&coordinates).unwrap();

    let vanishing_poly = vanishing_poly(input_points);
    let comm_vanishing_poly: G2Point = opening_key.commit_g2(&vanishing_poly).into();

    let comm_r_x = opening_key.commit_g1(&r_x);
    let comm_minus_r_x: G1Point = (G1Projective::from(commitment) - comm_r_x).into();
    multi_pairings(&[
        (&proof, &G2Prepared::from(comm_vanishing_poly)),
        (&comm_minus_r_x, &G2Prepared::from(-opening_key.g2_gen())),
    ])
}


fn compute_fiat_shamir_challenge(opening_key : &OpeningKey, row_commitments : &[G1Point], row_indices : &[u64], column_indices : &[u64], coset_evals : &[Vec<Scalar>], proofs : &[G1Point]) -> Scalar {
    let domain_sep = "RCKZGCBATCH__V1_";
    let mut hash_input : Vec<u8> = Vec::new();

    // Domain separation
    hash_input.extend(domain_sep.as_bytes());

    const FIELD_ELEMENTS_PER_BLOB : u64 = 4096;
    const FIELD_ELEMENTS_PER_CELL : u64 = 64;
    
    // polynomial bound
    hash_input.extend((opening_key.num_coefficients_in_polynomial as u64).to_be_bytes());

    // field elements per cell
    hash_input.extend((opening_key.multi_opening_size as u64).to_be_bytes());

    let num_commitments = row_commitments.len() as u64;
    hash_input.extend(num_commitments.to_be_bytes());

    let num_cells = column_indices.len() as u64;
    hash_input.extend(num_cells.to_be_bytes());

    for commitment in row_commitments{
        hash_input.extend(commitment.to_compressed())
    }

    for k in 0..num_cells {
        hash_input.extend(row_indices[k as usize].to_be_bytes());
        hash_input.extend(column_indices[k as usize].to_be_bytes());
        for eval in &coset_evals[k as usize] {
            hash_input.extend(eval.to_bytes_be())
        }
        hash_input.extend(proofs[k as usize].to_compressed())
    }
    use bls12_381::ff::Field;
    
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(hash_input);
    let mut result : [u8;32] = hasher.finalize().try_into().expect("sha256 should return a 32 byte array");
    
    // For randomization, we only need a 128 bit scalar, since this is used for batch verification.
    // See for example, the randomizers section in : https://cr.yp.to/badbatch/badbatch-20120919.pdf
    // 
    // This is noted because when we truncate the 256 bit hash into a scalar,
    // a bias will be introduced. This however does not affect our security guarantees 
    // because the bias is negligible given we want a uniformly random 128 bit integer.
    //
    // So that we know it fits into a scalar, we shave off 2 bits.
    result[0] = (result[0] << 2) >> 2;
    let scalar = Scalar::from_bytes_be(&result).expect("254 bit integer should have been reducible to a scalar");

    // TODO: Could remove this, since it is statistically improbable
    // TODO: we add 1 to the scalar, so that it can never be 0
    // TODO: This is also taken from: https://cr.yp.to/badbatch/badbatch-20120919.pdf
    let scalar = scalar + Scalar::ONE;

    scalar
}

fn compute_powers(value : Scalar, num_elements : usize) -> Vec<Scalar> {
    use bls12_381::ff::Field;
    
    let mut powers = Vec::new();
    let mut current_power = Scalar::ONE;
    
    for _ in 0..num_elements {
        powers.push(current_power);
        current_power *= value;
    }

    powers
}

pub fn verify_multi_opening(opening_key : &OpeningKey, row_commitments : &[G1Point], commitment_indices : &[u64], cell_indices : &[u64], coset_shifts: &[Scalar], cosets : &[Vec<Scalar>], coset_evals : &[Vec<Scalar>], proofs : &[G1Point]) -> bool { 
    use bls12_381::ff::Field;
    
    // Compute random challenges for batching the opening together.
    // 
    // We compute one challenge `r` using fiat-shamir and the rest are powers of `r`
    // This is safe since 1, X, X^2, ..., X^n of a variable X are linearly independent (ie there is no non-trivial linear combination that equals zero)
    //
    // TODO: Because this method takes in G1Points and not their serialized form, there is a roundtrip that happens
    // TODO: when we serialize the point for fiat shamir. (I'm leaving this TOOD here until we benchmark the diff)
    let r = compute_fiat_shamir_challenge(opening_key, row_commitments, commitment_indices, cell_indices, coset_evals, proofs);
    let r_powers = compute_powers(r, commitment_indices.len());

    // Convert the proofs to Projective form.
    // This is essentially free and we are mainly paying for the allocation cost here.
    let proofs = proofs.iter().map(bls12_381::G1Projective::from).collect::<Vec<_>>();
    let row_commitments = row_commitments.iter().map(bls12_381::G1Projective::from).collect::<Vec<_>>();

    let num_cells = cell_indices.len();
    let n = opening_key.multi_opening_size;
    let num_unique_commitments = row_commitments.len();

    // First compute a random linear combination of the proofs
    let random_sum_proofs = bls12_381::lincomb::g1_lincomb(&proofs, &r_powers);

    // Now compute a random linear combination of the commitments
    //
    // We know that many of the commitments are duplicated, so we optimize for this 
    // use case.
    // 
    // For example, imagine we wanted to do r_1 * G_1 + r_2 * G_1
    // This would be equivalent to doing (r_1 + r_2) * G_1
    // The (r_1 + r_2) is what is being referred to as the `weight` 
    let mut weights = vec![Scalar::from(0); num_unique_commitments];
    for k in 0..num_cells {
        // For each row index, we get its commitment index `i`.
        // ie, `i` just means we are looking at G_i
        let commitment_index = commitment_indices[k];
        // We then add the contribution of `r` as a part of that commitments weight.
        weights[commitment_index as usize] += r_powers[k];
    }
    let random_sum_commitments = bls12_381::lincomb::g1_lincomb(&row_commitments, &weights);

    let domain = polynomial::domain::Domain::new(opening_key.multi_opening_size);

    // Compute a random linear combination of the interpolation polynomials
    let mut sum_interpolation_poly = Vec::new();
    for k in 0..num_cells {    
        let mut coset_evals_clone = coset_evals[k].clone();
        reverse_bit_order(&mut coset_evals_clone);

        // Compute the interpolation polynomial
        let ifft_scalars = domain.ifft_scalars(coset_evals_clone);
        let h_k = coset_shifts[cell_indices[k] as usize];
        let mut inv_h_k_powers = compute_powers(h_k, ifft_scalars.len());
        batch_inverse(&mut inv_h_k_powers);
        let ifft_scalars : Vec<_>= ifft_scalars.into_iter().zip(inv_h_k_powers).map(|(scalar, inv_h_k_pow)| scalar * inv_h_k_pow).collect();
    
        let scale_factor = r_powers[k];
        let r_x = ifft_scalars.into_iter().map(|coeff| coeff * scale_factor).collect::<Vec<_>>();
        
        sum_interpolation_poly = poly_add(sum_interpolation_poly, r_x);
    }
    let random_sum_interpolation = opening_key.commit_g1(&sum_interpolation_poly);
    
    // [s^n]
    let s_pow_n = opening_key.g2s[n];

    let mut weighted_r_powers = Vec::with_capacity(num_cells);
    for k in 0..num_cells {
        // This is expensive and does not need to be done all the time.
        let h_k = coset_shifts[cell_indices[k] as usize];
        let h_k_pow = h_k.pow_vartime(&[n as u64]);
        let wrp = r_powers[k] * h_k_pow;
        weighted_r_powers.push(wrp);
    }
    let random_weighted_sum_proofs = bls12_381::lincomb::g1_lincomb(&proofs, &weighted_r_powers);

    // TODO: Find a better name for this
    let rl = (random_sum_commitments - random_sum_interpolation) + random_weighted_sum_proofs;

    // TODO: These .into are a bit expensive since they are converting from projective to affine
    
    let s_pow_n : G2Point= s_pow_n.into();
    multi_pairings(&[
        (&random_sum_proofs.into(), &G2Prepared::from(s_pow_n)),
        (&rl.into(), &G2Prepared::from(-opening_key.g2_gen())),
    ])
}

/// Computes a multi-point opening proof using the general formula.
///
/// Note: This copies the implementation that the consensus-specs uses.
/// With the exception that division is done using Ruffini's rule.
pub fn compute_multi_opening_naive(
    commit_key: &CommitKey,
    polynomial: &PolyCoeff,
    points: &[Scalar],
) -> (G1Point, Vec<Scalar>) {
    // Divides `self` by x-z using Ruffinis rule
    fn divide_by_linear(poly: &[Scalar], z: Scalar) -> Vec<Scalar> {
        let mut quotient: Vec<Scalar> = Vec::with_capacity(poly.len());
        let mut k = Scalar::from(0u64);

        for coeff in poly.iter().rev() {
            let t = *coeff + k;
            quotient.push(t);
            k = z * t;
        }

        // Pop off the remainder term
        quotient.pop();

        // Reverse the results as monomial form stores coefficients starting with lowest degree
        quotient.reverse();
        quotient
    }

    let mut evaluations = Vec::new();
    for point in points {
        let evaluation = poly_eval(polynomial, point);
        evaluations.push(evaluation);
    }

    // Compute f(x) - I(x) / \prod (x - z_i)
    // Where I(x) is the polynomial such that r(z_i) = f(z_i) for all z_i
    //
    // We can speed up computation of I(x) by doing an IFFT, given the coset generator, since
    // we know all of the points are of the form k * \omega where \omega is a root of unity

    let coordinates: Vec<_> = points
        .iter()
        .zip(evaluations.iter())
        .map(|(p, e)| (*p, *e))
        .collect();

    let r_x = lagrange_interpolate(&coordinates).unwrap();

    // check that the r_x polynomial is correct, it should essentially be the polynomial that
    // evaluates to f(z_i) = r(z_i)
    for (point, evaluation) in points.iter().zip(evaluations.iter()) {
        assert_eq!(poly_eval(&r_x, point), *evaluation);
    }

    let poly_shifted = poly_sub(polynomial.to_vec().clone(), r_x.clone());

    let mut quotient_poly = poly_shifted.to_vec().clone();
    for point in points.iter() {
        quotient_poly = divide_by_linear(&quotient_poly, *point);
    }

    (commit_key.commit_g1(&quotient_poly).into(), evaluations)
}
