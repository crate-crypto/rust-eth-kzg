use crate::{traits::*, G1Point, G1Projective, G2Point, G2Projective, Scalar};

/// A multi-scalar multiplication algorithm over G1 elements
///
/// Returns None if the points and the scalars are not the
/// same length.
pub fn g1_lincomb(points: &[G1Point], scalars: &[Scalar]) -> Option<G1Projective> {
    if points.len() != scalars.len() {
        return None;
    }

    if points.is_empty() {
        return Some(G1Projective::identity());
    }

    // Convert to Projective, since the API forces us to do this
    let proj_points: Vec<_> = points.iter().map(Into::into).collect();
    return Some(G1Projective::multi_exp(&proj_points, &scalars));
}

/// A multi-scalar multiplication algorithm over G2 elements
///
/// Returns None if the points and the scalars are not the
/// same length.
pub fn g2_lincomb(points: &[G2Point], scalars: &[Scalar]) -> Option<G2Projective> {
    if points.len() != scalars.len() {
        return None;
    }

    // Return group identity if no valid points remain
    if points.is_empty() {
        return Some(G2Projective::identity());
    }

    // Convert to Projective, since the API forces us to do this
    let proj_points: Vec<_> = points.iter().map(Into::into).collect();

    return Some(G2Projective::multi_exp(&proj_points, &scalars));
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn g1_lincomb_empty_inputs() {
        // MSM with empty input returns identity
        let points: Vec<G1Point> = vec![];
        let scalars: Vec<Scalar> = vec![];
        let result = g1_lincomb(&points, &scalars).expect("length mismatch");
        assert_eq!(result, G1Projective::identity());
    }

    #[test]
    fn g1_lincomb_length_mismatch_empty() {
        // MSM returns None when point and scalar lengths differ
        let points = vec![G1Point::generator()];
        let scalars = vec![];
        assert_eq!(g1_lincomb(&points, &scalars), None);
    }

    #[test]
    fn g1_lincomb_length_mismatch_not_empty() {
        // MSM returns None when point and scalar lengths differ
        let points = vec![G1Point::generator(); 4];
        let scalars = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        assert_eq!(g1_lincomb(&points, &scalars), None);
    }

    #[test]
    fn g2_lincomb_length_mismatch_empty() {
        // MSM returns None when point and scalar lengths differ (and one is empty)
        let points = vec![G2Point::generator()];
        let scalars = vec![];
        assert_eq!(g2_lincomb(&points, &scalars), None);
    }

    #[test]
    fn g2_lincomb_length_mismatch_not_empty() {
        // MSM returns None when point and scalar lengths differ
        let points = vec![G2Point::generator(); 4];
        let scalars = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        assert_eq!(g2_lincomb(&points, &scalars), None);
    }

    #[test]
    fn g1_lincomb_basic_correctness() {
        // Test 3P = P + P + P
        let p = G1Point::generator();
        let s1 = Scalar::ONE;
        let s3 = Scalar::from(3);
        let points = vec![p, p, p];
        let scalars = vec![s1, s1, s1];

        let expected = G1Projective::generator() * s3;
        let result = g1_lincomb(&points, &scalars).expect("length mismatch");
        assert_eq!(result, expected);
    }

    #[test]
    fn g1_lincomb_handles_identity_correctly() {
        // Mix generator and identity points with scalars
        let p = G1Point::generator();
        let zero = Scalar::ZERO;
        let one = Scalar::ONE;

        // Construct input points:
        // - First: the generator, contributes normally
        // - Second: the identity point, should be skipped
        // - Third: the generator, but its scalar is zero, so it contributes nothing
        let points = vec![p, G1Point::identity(), p];
        let scalars = vec![one, one, zero];

        // Note: blst used to have a bug where the identity point was not being handled correctly
        // so we have this test
        let result = g1_lincomb(&points, &scalars).expect("length mismatch");
        assert_eq!(result, G1Projective::generator());
    }

    #[test]
    fn g2_lincomb_handles_identity_correctly() {
        // Mix generator and identity points with scalars
        let p = G2Point::generator();
        let zero = Scalar::ZERO;
        let one = Scalar::ONE;

        // - The first point should contribute normally (1 * P),
        // - The second is the identity and should be skipped,
        // - The third has a zero scalar and should be skipped as well
        let points = vec![p, G2Point::identity(), p];
        let scalars = vec![one, one, zero];

        // Only the first point should contribute: 1 * G = G
        let result = g2_lincomb(&points, &scalars).expect("length mismatch");
        assert_eq!(result, G2Projective::generator());
    }

    #[test]
    fn g2_lincomb_basic_correctness() {
        // Test that 2P = P + P for G2
        let p = G2Point::generator();
        let one = Scalar::ONE;
        let two = Scalar::from(2);

        let points = vec![p, p];
        let scalars = vec![one, one];

        let expected = G2Projective::generator() * two;
        let result = g2_lincomb(&points, &scalars).expect("length mismatch");
        assert_eq!(result, expected);
    }

    #[test]
    fn g1_lincomb_randomized_consistency() {
        // Initialize a deterministic standard RNG
        let mut rng = StdRng::seed_from_u64(42);

        // Generate 10 random G1 points (projective), convert to affine
        let points: Vec<_> = (0..10)
            .map(|_| G1Projective::random(&mut rng).into())
            .collect();

        // Generate 10 random scalars
        let scalars: Vec<_> = (0..10).map(|_| Scalar::random(&mut rng)).collect();

        // Compute the naive expected result using individual scalar multiplications
        let expected: G1Projective = points
            .iter()
            .zip(&scalars)
            .map(|(p, s)| G1Projective::from(*p) * s)
            .sum();

        // Compute the result using the optimized linear combination function
        let result = g1_lincomb(&points, &scalars).expect("length mismatch");

        // Ensure the result matches the naive computation
        assert_eq!(result, expected);
    }

    #[test]
    fn g2_lincomb_randomized_consistency() {
        // Initialize a deterministic standard RNG
        let mut rng = StdRng::seed_from_u64(42);

        // Generate 10 random G2 points (projective), convert to affine
        let points: Vec<_> = (0..10)
            .map(|_| G2Projective::random(&mut rng).into())
            .collect();

        // Generate 10 random scalars
        let scalars: Vec<_> = (0..10).map(|_| Scalar::random(&mut rng)).collect();

        // Compute the naive expected result using individual scalar multiplications
        let expected: G2Projective = points
            .iter()
            .zip(&scalars)
            .map(|(p, s)| G2Projective::from(*p) * s)
            .sum();

        // Compute the result using the optimized linear combination function
        let result = g2_lincomb(&points, &scalars).expect("length mismatch");

        // Ensure the result matches the naive computation
        assert_eq!(result, expected);
    }
}
