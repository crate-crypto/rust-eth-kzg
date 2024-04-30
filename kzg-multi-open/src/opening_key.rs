use crate::lincomb::g2_lincomb;
use bls12_381::{G1Projective, G2Projective, Scalar};

/// Opening Key is used to verify opening proofs made about a committed polynomial.
#[derive(Clone, Debug)]
pub struct OpeningKey {
    /// The generator of G1 used in the setup
    pub g1_gen: G1Projective,
    /// The powers of tau G2 used in the setup
    ///
    /// ie group elements of the form `{ \tau^i G }`
    pub g2s: Vec<G2Projective>,
}

impl OpeningKey {
    pub fn new(g1_gen: G1Projective, g2s: Vec<G2Projective>) -> Self {
        Self { g1_gen, g2s }
    }
    /// Commit to a polynomial in monomial form using the G2 group elements
    pub fn commit_g2(&self, polynomial: &[Scalar]) -> G2Projective {
        assert!(self.g2s.len() >= polynomial.len());
        g2_lincomb(&self.g2s[0..polynomial.len()], &polynomial)
    }
}
