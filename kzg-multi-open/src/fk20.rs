// [FK20] is a paper by Dmitry Khovratovich and Dankrad Feist that describes a method for
// efficiently opening a set of points when the opening points are roots of unity.

use bls12_381::group::prime::PrimeCurveAffine;
use bls12_381::group::Curve;
use bls12_381::{G1Point, Scalar};
use polynomial::{domain::Domain, monomial::PolyCoeff};

use crate::{commit_key::CommitKey, reverse_bit_order};

/// This is doing \floor{f(x) / x^d}
/// which essentially means removing the first d coefficients
///
/// Note: This is just doing a shifting of the polynomial coefficients. However,
/// we refrain from calling this method `shift_polynomial` due to the specs
/// naming a method with different functionality that name.
pub fn divide_by_monomial_floor(poly: &PolyCoeff, degree: usize) -> &[Scalar] {
    let n = poly.len();
    // If the degree of the monomial is greater than or equal to the number of coefficients,
    // the division results in the zero polynomial
    assert!(
        degree < n,
        "degree should be less than the number of coefficients"
    );
    &poly[degree..]
}

/// Naively compute the `h`` polynomials for the FK20 proof.
///
/// See section 3.1.1 of the FK20 paper for more details.
///
/// FK20 computes the commitments to these polynomials in 3.1.1.
pub fn naive_compute_h_poly(polynomial: &PolyCoeff, l: usize) -> Vec<&[Scalar]> {
    assert!(
        l.is_power_of_two(),
        "expected l to be a power of two (its the size of the cosets), found {}",
        l
    );

    let m = polynomial.len();
    assert!(
        m.is_power_of_two(),
        "expected polynomial to have power of 2 number of evaluations. Found {}",
        m
    );
    let k: usize = m / l;
    assert!(
        k.is_power_of_two(),
        "expected k to be a power of two, found {}",
        k
    );

    let mut h_polys = Vec::with_capacity(k - 1);
    for index in 1..k {
        let degree = index * l;
        let h_poly_i = divide_by_monomial_floor(&polynomial, degree);
        h_polys.push(h_poly_i);
    }

    assert!(h_polys.len() == k - 1);

    h_polys
}

/// Computes FK20 proofs over multiple cosets without using a toeplitz matrix.
/// of the `h` polynomials and MSMs for computing the proofs.
pub fn naive_fk20_open_multi_point(
    commit_key: &CommitKey,
    proof_domain: &Domain,
    ext_domain: &Domain,
    polynomial: &PolyCoeff,
    l: usize,
) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
    let h_polys = naive_compute_h_poly(polynomial, l);
    let commitment_h_polys = h_polys
        .iter()
        .map(|h_poly| commit_key.commit_g1(h_poly))
        .collect::<Vec<_>>();
    let proofs = proof_domain.fft_g1(commitment_h_polys.clone());

    let mut proofs_affine = vec![G1Point::identity(); proofs.len()];
    // TODO: This does not seem to be using the batch affine trick
    bls12_381::G1Projective::batch_normalize(&proofs, &mut proofs_affine);

    // Compute the evaluations of the polynomial at the cosets by doing an fft
    let mut evaluations = ext_domain.fft_scalars(polynomial.clone());
    reverse_bit_order(&mut evaluations);
    let set_of_output_points: Vec<_> = evaluations
        .chunks_exact(l)
        .into_iter()
        .map(|slice| slice.to_vec())
        .collect();

    // reverse the order of the proofs, since fft_g1 was applied using
    // the regular order.
    reverse_bit_order(&mut proofs_affine);

    (proofs_affine, set_of_output_points)
}

#[cfg(test)]
mod tests {
    use bls12_381::Scalar;

    use crate::fk20::divide_by_monomial_floor;

    #[test]
    fn check_divide_by_monomial_floor() {
        // \floor(x^2 + x + 10 / x) = x + 1
        let poly = vec![Scalar::from(10u64), Scalar::from(1u64), Scalar::from(1u64)];
        let result = divide_by_monomial_floor(&poly, 1);
        assert_eq!(result, vec![Scalar::from(1u64), Scalar::from(1u64)]);
    }
}
