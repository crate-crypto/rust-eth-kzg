use crate::lincomb::{g1_lincomb, g2_lincomb};
use bls12_381::{G1Projective, G2Point, G2Projective, Scalar};

/// Opening Key is used to verify opening proofs made about a committed polynomial.
#[derive(Clone, Debug)]
pub struct OpeningKey {
    /// The powers of tau G1 used in the setup
    ///
    /// ie group elements of the form `{ \tau^i G }`
    pub g1s: Vec<G1Projective>,
    /// The powers of tau G2 used in the setup
    ///
    /// ie group elements of the form `{ \tau^i G }`
    pub g2s: Vec<G2Projective>,
}

impl OpeningKey {
    pub fn new(g1s: Vec<G1Projective>, g2s: Vec<G2Projective>) -> Self {
        Self { g1s, g2s }
    }
    /// Commit to a polynomial in monomial form using the G2 group elements
    pub fn commit_g2(&self, polynomial: &[Scalar]) -> G2Projective {
        assert!(self.g2s.len() >= polynomial.len());
        g2_lincomb(&self.g2s[0..polynomial.len()], &polynomial)
    }

    /// Commit to a polynomial in monomial form using the G1 group elements
    pub fn commit_g1(&self, polynomial: &[Scalar]) -> G1Projective {
        assert!(self.g1s.len() >= polynomial.len());
        g1_lincomb(&self.g1s[0..polynomial.len()], &polynomial)
    }

    // TODO: Check if there is a cost to converting G2Projective to G2Point
    // TODO: when z==1
    pub fn g2_gen(&self) -> G2Point {
        self.g2s[0].into()
    }
}
