use bls12_381::{multi_pairings, G1Point, G1Projective, G2Point, G2Prepared, Scalar};
use polynomial::monomial::{lagrange_interpolate, vanishing_poly};

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
    pub fn compute(commit_key: &CommitKey, polynomial: &[Scalar]) -> Proof {
        todo!()
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
