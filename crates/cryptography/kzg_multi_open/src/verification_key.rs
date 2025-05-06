use bls12_381::{
    lincomb::{g1_lincomb, g2_lincomb},
    G1Point, G1Projective, G2Point, G2Projective, Scalar,
};

/// Verification Key is used to verify opening proofs made about a committed polynomial.
#[derive(Clone, Debug)]
pub struct VerificationKey {
    /// The powers of tau G1 used in the setup
    ///
    /// ie group elements of the form `{ \tau^i G }`
    pub g1s: Vec<G1Point>,
    /// The powers of tau G2 used in the setup
    ///
    /// ie group elements of the form `{ \tau^i G }`
    pub g2s: Vec<G2Point>,
    /// The degree-0 term in the powers of tau G2 elements.
    pub g2_gen: G2Point,
    /// This is the number of points that will be a
    /// opened at any one time. Another way to think
    /// of this, is that its the number of points a
    /// proof will attest to.
    ///
    /// In most cases, this is the number of G1 elements,
    /// however, we have this explicit parameter to
    /// avoid foot guns.
    pub coset_size: usize,
    /// The number of coefficients in the polynomial that we want to
    /// verify claims about.
    ///
    /// Note: We could also use the max degree bound here. (This is a matter of preference)
    pub num_coefficients_in_polynomial: usize,
}

impl VerificationKey {
    pub fn new(
        g1s: Vec<G1Point>,
        g2s: Vec<G2Point>,
        coset_size: usize,
        num_coefficients_in_polynomial: usize,
    ) -> Self {
        // This assumes that the trusted setup contains more than 1 element.
        //
        // For all of our purposes and for any useful applications, this will be the case.
        let g2_gen = g2s[0];

        assert!(coset_size < g2s.len(), "The coset size must be less than the amount of g2 elements as the verifier needs to do a g2 msm of size `coset_size`");

        Self {
            g1s,
            g2s,
            g2_gen,
            coset_size,
            num_coefficients_in_polynomial,
        }
    }

    /// Commit to a polynomial in monomial form using the G2 group elements
    pub fn commit_g2(&self, polynomial: &[Scalar]) -> G2Projective {
        assert!(self.g2s.len() >= polynomial.len());
        g2_lincomb(&self.g2s[..polynomial.len()], polynomial)
            .expect("number of g2 points is equal to the number of coefficients in the polynomial")
    }

    /// Commit to a polynomial in monomial form using the G1 group elements
    pub fn commit_g1(&self, polynomial: &[Scalar]) -> G1Projective {
        assert!(self.g1s.len() >= polynomial.len());
        g1_lincomb(&self.g1s[..polynomial.len()], polynomial)
            .expect("number of g1 points is equal to the number of coefficients in the polynomial")
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::{group::Group, G1Projective, Scalar};

    use super::*;

    #[test]
    fn test_commit_g1_matches_lincomb() {
        // Polynomial coefficients
        let poly = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];

        // G1 points: just use the generator three times
        let g1s: Vec<G1Point> = (0..3).map(|_| G1Projective::generator().into()).collect();

        // Dummy G2s, unused in this test
        let g2s: Vec<_> = (0..4).map(|_| G2Projective::generator().into()).collect();

        let vk = VerificationKey::new(g1s.clone(), g2s, 2, 3);

        // Expected = 1 * G + 2 * G + 3 * G = (1 + 2 + 3) * G = 6 * G
        let g = G1Projective::generator();
        let expected = g * Scalar::from(6u64);

        let actual = vk.commit_g1(&poly);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_commit_g2_matches_lincomb() {
        // Polynomial coefficients
        let poly = vec![Scalar::from(5u64), Scalar::from(7u64), Scalar::from(11u64)];

        // G2 points: use the generator three times
        let g2s: Vec<G2Point> = (0..3).map(|_| G2Projective::generator().into()).collect();

        // Dummy G1s, not used here
        let g1s: Vec<_> = (0..4).map(|_| G1Projective::generator().into()).collect();

        let vk = VerificationKey::new(g1s, g2s.clone(), 2, 3);

        // Expected: 5 * G + 7 * G + 11 * G = (5 + 7 + 11) * G = 23 * G
        let g = G2Projective::generator();
        let expected = g * Scalar::from(23u64);

        let actual = vk.commit_g2(&poly);

        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn test_coset_size_check_panics() {
        let g1s = vec![G1Projective::generator().into(); 2];
        let g2s = vec![G2Projective::generator().into(); 2];
        let _vk = VerificationKey::new(g1s, g2s, 2, 2); // coset_size == g2s.len() → panic
    }

    #[test]
    fn test_g2_gen_is_first_element() {
        let g2s: Vec<_> = (0..4).map(|_| G2Projective::generator().into()).collect();
        let g1s: Vec<_> = (0..4).map(|_| G1Projective::generator().into()).collect();

        let vk = VerificationKey::new(g1s, g2s.clone(), 1, 3);
        assert_eq!(vk.g2_gen, g2s[0]);
    }

    #[test]
    #[should_panic]
    fn test_commit_g1_panics_when_poly_longer_than_g1s() {
        // 2 G1 points
        let g1s: Vec<G1Point> = (0..2).map(|_| G1Projective::generator().into()).collect();
        let g2s: Vec<G2Point> = (0..3).map(|_| G2Projective::generator().into()).collect();

        // Polynomial of length 3 — longer than g1s
        let poly = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];

        let vk = VerificationKey::new(g1s, g2s, 1, 3);

        // This will panic: not enough g1s to match poly length
        let _ = vk.commit_g1(&poly);
    }

    #[test]
    #[should_panic]
    fn test_commit_g2_panics_when_poly_longer_than_g2s() {
        // 2 G2 points
        let g2s: Vec<G2Point> = (0..2).map(|_| G2Projective::generator().into()).collect();
        let g1s: Vec<G1Point> = (0..3).map(|_| G1Projective::generator().into()).collect();

        // Polynomial of length 3 — longer than g2s
        let poly = vec![Scalar::from(5), Scalar::from(7), Scalar::from(11)];

        let vk = VerificationKey::new(g1s, g2s, 1, 3);

        // This will panic: not enough g2s to match poly length
        let _ = vk.commit_g2(&poly);
    }

    #[test]
    fn test_commit_g1_with_more_g1s_than_poly_len() {
        // 5 G1 points in trusted setup
        let g1s: Vec<G1Point> = (0..5).map(|_| G1Projective::generator().into()).collect();
        let g2s: Vec<G2Point> = (0..5).map(|_| G2Projective::generator().into()).collect();

        // Polynomial of length 3
        let poly = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];

        let vk = VerificationKey::new(g1s, g2s, 1, 5);

        // Compute manually: 1 * G + 2 * G + 3 * G = 6 * G
        let g = G1Projective::generator();
        let expected = g * Scalar::from(6u64);

        let actual = vk.commit_g1(&poly);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_commit_g2_with_more_g2s_than_poly_len() {
        // 5 G2 points in trusted setup
        let g2s: Vec<G2Point> = (0..5).map(|_| G2Projective::generator().into()).collect();
        let g1s: Vec<G1Point> = (0..5).map(|_| G1Projective::generator().into()).collect();

        // Polynomial of length 3
        let poly = vec![Scalar::from(2), Scalar::from(4), Scalar::from(6)];

        let vk = VerificationKey::new(g1s, g2s, 1, 5);

        // Compute manually: 2 * G + 4 * G + 6 * G = 12 * G
        let g = G2Projective::generator();
        let expected = g * Scalar::from(12u64);

        let actual = vk.commit_g2(&poly);
        assert_eq!(actual, expected);
    }
}
