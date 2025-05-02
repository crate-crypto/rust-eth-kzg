use bls12_381::{lincomb::g1_lincomb, G1Point, G1Projective, Scalar};

// The key that is used to commit to polynomials in monomial form
//
/// This contains group elements of the form `{ \tau^i G }`
///  Where:
/// - `i` ranges from 0 to `degree`.
/// - `G` is some generator of the group
#[derive(Debug, Clone)]
pub struct CommitKey {
    pub g1s: Vec<G1Point>,
}

impl CommitKey {
    pub fn new(g1_points: Vec<G1Point>) -> Self {
        assert!(
            !g1_points.is_empty(),
            "cannot initialize `CommitKey` with no g1 points"
        );

        Self { g1s: g1_points }
    }

    /// Commit to `polynomial` in monomial form using the G1 group elements
    pub fn commit_g1(&self, poly_coeff: &[Scalar]) -> G1Projective {
        // Note: We could use g1_lincomb_unsafe here, because we know that none of the points are the
        // identity element.
        // We use g1_lincomb because it is safer and the performance difference is negligible
        g1_lincomb(&self.g1s[0..poly_coeff.len()], poly_coeff)
            .expect("number of g1 points is equal to the number of coefficients in the polynomial")
    }
}
