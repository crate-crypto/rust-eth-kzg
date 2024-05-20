use bls12_381::lincomb::g1_lincomb;
use bls12_381::{G1Projective, Scalar};
use polynomial::domain::Domain;

// The key that is used to commit to polynomials in monomial form
//
/// This contains group elements of the form `{ \tau^i G }`
///  Where:
/// - `i` ranges from 0 to `degree`.
/// - `G` is some generator of the group
#[derive(Debug, Clone)]
pub struct CommitKey {
    pub g1s: Vec<G1Projective>,
}

// The key that is used to commit to polynomials in lagrange form
//
/// The G1 group elements are of the form `{ \L_i(\tau) * G }`
/// Where :
/// - `i` ranges from 0 to `degree`
/// -  L_i is the i'th lagrange polynomial
/// - `G` is some generator of the group
pub struct CommitKeyLagrange {
    pub g1s: Vec<G1Projective>,
}

impl CommitKey {
    pub fn new(g1_points: Vec<G1Projective>) -> CommitKey {
        assert!(
            !g1_points.is_empty(),
            "cannot initialize `CommitKey` with no g1 points"
        );

        CommitKey { g1s: g1_points }
    }

    /// Convert the `CommitKey` to a `CommitKeyLagrange`
    ///
    /// This is done by computing the lagrange basis of the G1 group elements
    pub fn into_lagrange(self, domain: &Domain) -> CommitKeyLagrange {
        CommitKeyLagrange {
            g1s: domain.ifft_g1(self.g1s),
        }
    }

    /// Commit to `polynomial` in monomial form using the G1 group elements
    pub fn commit_g1(&self, poly_coeff: &[Scalar]) -> G1Projective {
        // Note: We could use g1_lincomb_unsafe here, because we know that none of the points are the
        // identity element.
        // We use g1_lincomb because it is safer and the performance difference is negligible
        g1_lincomb(&self.g1s[0..poly_coeff.len()], poly_coeff)
    }
}

impl CommitKeyLagrange {
    pub fn new(g1s: Vec<G1Projective>) -> CommitKeyLagrange {
        CommitKeyLagrange { g1s }
    }

    /// Commit to a polynomial in lagrange form using the G1 group elements
    pub fn commit_g1(&self, polynomial: &[Scalar]) -> G1Projective {
        assert!(self.g1s.len() >= polynomial.len());
        g1_lincomb(&self.g1s[0..polynomial.len()], polynomial)
    }
}
