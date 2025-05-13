use bls12_381::{lincomb::g1_lincomb, G1Point, G1Projective, Scalar};

/// The key that is used to commit to polynomials in monomial form
///
/// This contains group elements of the form `{ \tau^i G }`
///  Where:
/// - `i` ranges from 0 to `degree`.
/// - `G` is some generator of the group
#[derive(Debug, Clone)]
pub struct CommitKey {
    /// A list of G1 group elements of the form $\tau^i \cdot G$,
    /// used to commit to polynomial coefficients.
    ///
    /// The length of this vector determines the maximum degree polynomial
    /// that can be safely committed using this key.
    pub g1s: Vec<G1Point>,
}

impl CommitKey {
    /// Constructs a new `CommitKey` from a list of G1 group elements.
    ///
    /// # Arguments
    /// - `g1s`: A non-empty vector of G1 elements representing powers of the trapdoor $\tau$,
    ///   i.e., [ \tau^0 \cdot G, \tau^1 \cdot G, \dots ].
    ///
    /// # Panics
    /// Panics if `g1s` is empty.
    pub fn new(g1s: Vec<G1Point>) -> Self {
        assert!(
            !g1s.is_empty(),
            "cannot initialize `CommitKey` with no g1 points"
        );

        Self { g1s }
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

#[cfg(test)]
mod tests {
    use bls12_381::{traits::*, G1Projective, Scalar};
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_commit_g1_matches_manual_lincomb() {
        // Polynomial
        let poly = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];

        // Setup: 3 G1 generator points
        let g1s: Vec<G1Point> = (0..3).map(|_| G1Projective::generator().into()).collect();
        let ck = CommitKey::new(g1s);

        // Expected = 1*G + 2*G + 3*G = 6*G
        let expected = G1Projective::generator() * Scalar::from(6);
        let actual = ck.commit_g1(&poly);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_commit_g1_with_more_g1s_than_poly_len() {
        // Polynomial
        let poly = vec![Scalar::from(1), Scalar::from(2)];

        // 5 G1 generator points available, only 2 used
        let g1s: Vec<G1Point> = (0..5).map(|_| G1Projective::generator().into()).collect();
        let ck = CommitKey::new(g1s);

        // Expected = 1*G + 2*G = 3*G
        let expected = G1Projective::generator() * Scalar::from(3);
        let actual = ck.commit_g1(&poly);

        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn test_commit_g1_panics_when_poly_longer_than_g1s() {
        // Polynomial
        let poly = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];

        // Only 2 G1 points
        let g1s: Vec<G1Point> = (0..2).map(|_| G1Projective::generator().into()).collect();
        let ck = CommitKey::new(g1s);

        // Should panic because poly.len() > g1s.len()
        let _ = ck.commit_g1(&poly);
    }

    #[test]
    #[should_panic]
    fn test_commit_key_empty_panics() {
        let _ = CommitKey::new(vec![]);
    }

    #[test]
    fn test_commit_g1_identity_when_poly_is_zero() {
        // Polynomial is all zero coefficients
        let poly = vec![Scalar::ZERO, Scalar::ZERO, Scalar::ZERO];

        let g1s: Vec<G1Point> = (0..3).map(|_| G1Projective::generator().into()).collect();
        let ck = CommitKey::new(g1s);

        let result = ck.commit_g1(&poly);
        assert_eq!(result, G1Projective::identity());
    }

    #[test]
    fn test_commit_key_commit_g1_randomized_consistency() {
        let mut rng = StdRng::seed_from_u64(42);

        let g1s: Vec<G1Point> = (0..10)
            .map(|_| G1Projective::random(&mut rng).into())
            .collect();
        let poly: Vec<Scalar> = (0..10).map(|_| Scalar::random(&mut rng)).collect();

        let ck = CommitKey::new(g1s.clone());

        // Naive expected commitment
        let expected: G1Projective = g1s
            .iter()
            .zip(&poly)
            .map(|(g, s)| G1Projective::from(*g) * s)
            .sum();

        let actual = ck.commit_g1(&poly);
        assert_eq!(actual, expected);
    }
}
