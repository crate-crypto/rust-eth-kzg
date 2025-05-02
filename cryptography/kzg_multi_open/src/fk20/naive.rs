use crate::commit_key::CommitKey;
use bls12_381::{g1_batch_normalize, G1Point, Scalar};
use polynomial::domain::Domain;
use polynomial::poly_coeff::PolyCoeff;

use super::cosets::reverse_bit_order;

/// This is doing \floor{f(x) / x^d}
/// which essentially means removing the first d coefficients
///
/// Another way to view this, is that this function is performing a right shift
/// on the polynomial by `degree` amount.
pub(crate) fn shift_polynomial(poly: &PolyCoeff, degree: usize) -> &[Scalar] {
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
pub(crate) fn compute_h_poly(polynomial: &PolyCoeff, coset_size: usize) -> Vec<&[Scalar]> {
    assert!(
        coset_size.is_power_of_two(),
        "expected coset_size to be a power of two, found {coset_size}"
    );

    let num_coefficients = polynomial.len();
    assert!(
        num_coefficients.is_power_of_two(),
        "expected polynomial to have power of 2 number of coefficients. Found {num_coefficients}"
    );

    let num_proofs: usize = num_coefficients / coset_size;
    assert!(
        num_proofs.is_power_of_two(),
        "expected num_proofs to be a power of two, found {num_proofs}"
    );

    let mut h_polys = Vec::with_capacity(num_proofs);
    for index in 1..=num_proofs {
        let degree = index * coset_size;
        let h_poly_i = shift_polynomial(polynomial, degree);
        h_polys.push(h_poly_i);
    }

    h_polys
}

/// Computes FK20 proofs over multiple cosets without using a toeplitz matrix.
/// of the `h` polynomials and MSMs for computing the proofs using a naive approach.
pub(crate) fn open_multi_point(
    commit_key: &CommitKey,
    polynomial: &PolyCoeff,
    coset_size: usize,
    number_of_points_to_open: usize,
) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
    assert!(coset_size.is_power_of_two());
    assert!(number_of_points_to_open.is_power_of_two());
    assert!(polynomial.len().is_power_of_two());
    assert!(number_of_points_to_open > coset_size);
    assert!(commit_key.g1s.len() >= polynomial.len());

    let h_polys = compute_h_poly(polynomial, coset_size);
    let commitment_h_polys = h_polys
        .iter()
        .map(|h_poly| commit_key.commit_g1(h_poly))
        .collect::<Vec<_>>();

    let proof_domain = Domain::new(number_of_points_to_open / coset_size);
    let proofs = proof_domain.fft_g1(commitment_h_polys);
    let mut proofs_affine = g1_batch_normalize(&proofs);

    // Reverse the proofs so they align with the coset evaluations
    reverse_bit_order(&mut proofs_affine);

    // Compute the coset evaluations
    let evaluation_domain = Domain::new(number_of_points_to_open);
    let coset_evaluations = compute_coset_evaluations(polynomial, coset_size, &evaluation_domain);

    (proofs_affine, coset_evaluations)
}

fn compute_coset_evaluations(
    polynomial: &PolyCoeff,
    coset_size: usize,
    evaluation_domain: &Domain,
) -> Vec<Vec<Scalar>> {
    // Compute the evaluations of the polynomial at the cosets by doing an fft
    let mut evaluations = evaluation_domain.fft_scalars(polynomial.clone());
    reverse_bit_order(&mut evaluations);

    evaluations
        .chunks_exact(coset_size)
        .map(<[Scalar]>::to_vec)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::fk20::naive::shift_polynomial;
    use bls12_381::Scalar;

    #[test]
    fn check_divide_by_monomial_floor() {
        // \floor(x^2 + x + 10 / x) = x + 1
        let poly = vec![Scalar::from(10u64), Scalar::from(1u64), Scalar::from(1u64)];
        let result = shift_polynomial(&poly, 1);
        assert_eq!(result, vec![Scalar::from(1u64), Scalar::from(1u64)]);
    }
}
