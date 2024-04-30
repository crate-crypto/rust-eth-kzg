use bls12_381::{G1Projective, Scalar};

use crate::commit_key::CommitKey;

/// A proof that a polynomial was opened at multiple points.
///
/// This creates a KZG proof as noted in [BDF21](https://eprint.iacr.org/2020/081.pdf)
/// using the techniques from [FK20](https://github.com/khovratovich/Kate/blob/master/Kate_amortized.pdf)
/// since the points being opened are roots of unity.
pub struct Proof {
    /// Commitment to the `witness`'s or quotient polynomials
    quotient_commitments: Vec<G1Projective>,
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
    pub fn verify(&self) -> bool {
        todo!()
    }
}
