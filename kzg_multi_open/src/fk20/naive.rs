use crate::commit_key::CommitKey;
use bls12_381::{g1_batch_normalize, G1Point, Scalar};
use polynomial::domain::Domain;
use polynomial::monomial::PolyCoeff;

use super::cosets::reverse_bit_order;

/// This is doing \floor{f(x) / x^d}
/// which essentially means removing the first d coefficients
///
/// Note: This is just doing a shifting of the polynomial coefficients. However,
/// we refrain from calling this method `shift_polynomial` due to the specs
/// naming a method with different functionality that name.
pub fn divide_by_monomial_floor(poly: &PolyCoeff, degree: usize) -> &[Scalar] {
    let n = poly.len();
    if degree >= n {
        // Return an empty slice if the degree is greater than or equal to
        // the number of coefficients
        //
        // This is the same behavior you get when you right-shift
        // a number by more tha the amount of bits needed to represent that number.
        &[]
    } else {
        &poly[degree..]
    }
}

/// Naively compute the `h`` polynomials for the FK20 proofs.
///
/// See section 3.1.1 of the FK20 paper for more details.
///
/// FK20 computes the commitments to these polynomials in 3.1.1.
pub fn compute_h_poly(polynomial: &PolyCoeff, coset_size: usize) -> Vec<&[Scalar]> {
    assert!(
        coset_size.is_power_of_two(),
        "expected coset_size to be a power of two, found {}",
        coset_size
    );

    let num_coefficients = polynomial.len();
    assert!(
        num_coefficients.is_power_of_two(),
        "expected polynomial to have power of 2 number of coefficients. Found {}",
        num_coefficients
    );

    let k: usize = num_coefficients / coset_size;
    assert!(
        k.is_power_of_two(),
        "expected k to be a power of two, found {}",
        k
    );

    let mut h_polys = Vec::with_capacity(k);
    for index in 1..=k {
        let degree = index * coset_size;
        let h_poly_i = divide_by_monomial_floor(polynomial, degree);
        h_polys.push(h_poly_i);
    }

    h_polys
}

/// Computes FK20 proofs over multiple cosets without using a toeplitz matrix.
/// of the `h` polynomials and MSMs for computing the proofs using a naive approach.
pub fn fk20_open_multi_point(
    commit_key: &CommitKey,
    polynomial: &PolyCoeff,
    coset_size: usize,
    number_of_points_to_open: usize,
) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
    assert!(coset_size.is_power_of_two());
    assert!(number_of_points_to_open.is_power_of_two());
    assert!(number_of_points_to_open > coset_size);
    assert!(polynomial.len().is_power_of_two());
    assert!(commit_key.g1s.len() >= polynomial.len());

    let proof_domain = Domain::new(number_of_points_to_open / coset_size);
    let ext_domain = Domain::new(number_of_points_to_open);

    let h_polys = compute_h_poly(polynomial, coset_size);
    let commitment_h_polys = h_polys
        .iter()
        .map(|h_poly| commit_key.commit_g1(h_poly))
        .collect::<Vec<_>>();

    let proofs = proof_domain.fft_g1(commitment_h_polys);
    let mut proofs_affine = g1_batch_normalize(&proofs);

    // reverse the order of the proofs, since fft_g1 was applied using
    // the regular order.
    reverse_bit_order(&mut proofs_affine);

    let evaluation_sets = fk20_compute_evaluation_set(polynomial, coset_size, &ext_domain);

    (proofs_affine, evaluation_sets)
}

fn fk20_compute_evaluation_set(
    polynomial: &PolyCoeff,
    coset_size: usize,
    ext_domain: &Domain,
) -> Vec<Vec<Scalar>> {
    // Compute the evaluations of the polynomial at the cosets by doing an fft
    let mut evaluations = ext_domain.fft_scalars(polynomial.clone());
    reverse_bit_order(&mut evaluations);

    evaluations
        .chunks_exact(coset_size)
        .map(|slice| slice.to_vec())
        .collect()
}

#[cfg(test)]
mod tests {
    use bls12_381::Scalar;

    use crate::fk20::naive::divide_by_monomial_floor;

    use crate::{
        create_insecure_commit_opening_keys,
        fk20::{naive, FK20},
    };
    use bls12_381::ff::Field;

    #[test]
    fn check_consistency_of_proofs_against_naive() {
        let poly_len = 4096;
        let poly = vec![Scalar::random(&mut rand::thread_rng()); poly_len];
        let coset_size = 64;
        let (commit_key, _) = create_insecure_commit_opening_keys();

        let (expected_proofs, expected_evaluations) =
            naive::fk20_open_multi_point(&commit_key, &poly, coset_size, 2 * poly_len);

        let fk20 = FK20::new(commit_key, poly_len, coset_size, 2 * poly_len);
        let (got_proofs, got_evaluations) =
            fk20.compute_multi_opening_proofs_poly_coeff(poly.clone());

        assert_eq!(got_proofs.len(), expected_proofs.len());
        assert_eq!(got_evaluations.len(), expected_evaluations.len());

        assert_eq!(got_evaluations, expected_evaluations);
        assert_eq!(got_proofs, expected_proofs);
    }

    #[test]
    fn check_divide_by_monomial_floor() {
        // \floor(x^2 + x + 10 / x) = x + 1
        let poly = vec![Scalar::from(10u64), Scalar::from(1u64), Scalar::from(1u64)];
        let result = divide_by_monomial_floor(&poly, 1);
        assert_eq!(result, vec![Scalar::from(1u64), Scalar::from(1u64)]);
    }
}
