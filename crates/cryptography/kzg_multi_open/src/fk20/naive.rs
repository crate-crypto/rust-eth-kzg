use bls12_381::{g1_batch_normalize, G1Point, Scalar};
use polynomial::{domain::Domain, poly_coeff::PolyCoeff};

use super::cosets::reverse_bit_order;
use crate::commit_key::CommitKey;

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

    let num_proofs = num_coefficients / coset_size;
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
    assert!(
        coset_size.is_power_of_two()
            && number_of_points_to_open.is_power_of_two()
            && polynomial.len().is_power_of_two()
            && number_of_points_to_open > coset_size
            && commit_key.g1s.len() >= polynomial.len()
    );

    let h_polys = compute_h_poly(polynomial, coset_size);
    let commitment_h_polys = h_polys
        .iter()
        .map(|h_poly| commit_key.commit_g1(h_poly))
        .collect();

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
    use bls12_381::{traits::*, G1Projective};

    use super::*;

    #[test]
    fn check_divide_by_monomial_floor() {
        // \floor(x^2 + x + 10 / x) = x + 1
        let poly = PolyCoeff(vec![
            Scalar::from(10u64),
            Scalar::from(1u64),
            Scalar::from(1u64),
        ]);
        let result = shift_polynomial(&poly, 1);
        assert_eq!(result, vec![Scalar::from(1u64), Scalar::from(1u64)]);
    }

    #[test]
    fn test_shift_polynomial_edge_cases() {
        // Poly: f(x) = 3 + 2x + x^2 + 4x^3
        let poly = PolyCoeff(vec![
            Scalar::from(3u64),
            Scalar::from(2u64),
            Scalar::from(1u64),
            Scalar::from(4u64),
        ]);

        // Shift by 0 = original polynomial
        let shifted0 = shift_polynomial(&poly, 0);
        assert_eq!(shifted0, &poly[..]);

        // Shift by 2 = drop 2 lowest degrees: x^2 + 4x^3
        let shifted2 = shift_polynomial(&poly, 2);
        assert_eq!(shifted2, &[Scalar::from(1u64), Scalar::from(4u64)]);

        // Shift by 4 = drop all terms
        let shifted4 = shift_polynomial(&poly, 4);
        assert!(shifted4.is_empty());

        // Shift by more than length = should return empty
        let shifted5 = shift_polynomial(&poly, 5);
        assert!(shifted5.is_empty());
    }

    #[test]
    fn test_compute_h_poly_basic() {
        // Polynomial: degree 7, coeffs [1,2,3,4,5,6,7,8]
        let poly = PolyCoeff((1u64..=8).map(Scalar::from).collect());

        // Coset size = 2, so h polynomials will be:
        // h1 = drop first 2 coeffs => [3,4,5,6,7,8]
        // h2 = drop first 4 coeffs => [5,6,7,8]
        // h3 = drop first 6 coeffs => [7,8]
        // h4 = drop first 8 coeffs => []
        let h = compute_h_poly(&poly, 2);
        assert_eq!(h.len(), 4);

        assert_eq!(
            h[0],
            &[
                Scalar::from(3),
                Scalar::from(4),
                Scalar::from(5),
                Scalar::from(6),
                Scalar::from(7),
                Scalar::from(8)
            ]
        );
        assert_eq!(
            h[1],
            &[
                Scalar::from(5),
                Scalar::from(6),
                Scalar::from(7),
                Scalar::from(8)
            ]
        );
        assert_eq!(h[2], &[Scalar::from(7), Scalar::from(8)]);
        assert_eq!(h[3], &[]);
    }

    /// Helper to create a simple CommitKey where g1s[i] = G1 generator * i
    fn dummy_commit_key(size: usize) -> CommitKey {
        let g = G1Projective::generator();
        let g1s: Vec<_> = (0..size)
            .map(|i| (g * Scalar::from(i as u64)).to_affine())
            .collect();
        CommitKey { g1s }
    }

    #[test]
    fn test_open_multi_point_naive() {
        // Create a polynomial with 8 coefficients
        let poly = PolyCoeff((1u64..=8).map(Scalar::from).collect());

        // Use a dummy commit key with enough points
        let commit_key = dummy_commit_key(8);

        // Coset size = 2, total points = 8, so 4 cosets
        let coset_size = 2;
        let number_of_points_to_open = 8;

        let (proofs, evals) =
            open_multi_point(&commit_key, &poly, coset_size, number_of_points_to_open);

        // Check that we got 4 G1 proofs (number_of_points_to_open / coset_size)
        assert_eq!(proofs.len(), 4);

        // Check that we got 4 coset evaluations, each of size 2
        assert_eq!(evals.len(), 4);
        for coset in &evals {
            assert_eq!(coset.len(), 2);
        }

        // The total number of coefficients is 8
        // The polynomial evaluations should be a permutation of the input poly’s FFT
        let domain = Domain::new(8);
        let mut expected = domain.fft_scalars(poly);
        reverse_bit_order(&mut expected);
        let expected_chunks: Vec<Vec<_>> =
            expected.chunks_exact(2).map(<[Scalar]>::to_vec).collect();

        assert_eq!(evals, expected_chunks);
    }
}
