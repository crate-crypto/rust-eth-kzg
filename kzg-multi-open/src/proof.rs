use bls12_381::{multi_pairings, G1Point, G1Projective, G2Point, G2Prepared, Scalar};
use polynomial::monomial::{lagrange_interpolate, poly_eval, poly_sub, vanishing_poly, PolyCoeff};

use crate::{commit_key::CommitKey, opening_key::OpeningKey};

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
    // e([Commitment] - [r(x)], [1]) == e([Q(x)], [Z(X)])

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
            let t = *coeff + &k;
            quotient.push(t);
            k = z * &t;
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

    // Compute f(x) - r(X) / \prod (x - z_i)
    // Where r(x) is the polynomial such that r(z_i) = f(z_i) for all z_i
    //
    // We can speed up computation of r(x) by doing an IFFT, given the coset generator, since
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
