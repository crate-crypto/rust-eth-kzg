use pairing::{MillerLoopResult, MultiMillerLoop};
use traits::*;

pub mod batch_addition;
pub mod batch_inversion;
mod booth_encoding;
pub mod fixed_base_msm;
pub mod fixed_base_msm_window;
pub mod lincomb;

// Re-exporting the blstrs crate

// Re-export ff and group, so other crates do not need to directly import(and independently version) them
pub use ff;
pub use group;

pub mod traits {
    pub use ff::{Field, PrimeField};
    pub use group::{prime::PrimeCurveAffine, Curve, Group};
}

/// Affine representation of a point in the BLS12-381 G1 curve group.
pub type G1Point = blstrs::G1Affine;

/// Projective representation of a point in the BLS12-381 G1 curve group.
pub type G1Projective = blstrs::G1Projective;

/// Affine representation of a point in the BLS12-381 G2 curve group.
pub type G2Point = blstrs::G2Affine;

/// Projective representation of a point in the BLS12-381 G2 curve group.
pub type G2Projective = blstrs::G2Projective;

/// Precomputed G2 point for efficient pairing computations.
///
/// This representation allows reusing expensive Miller loop setup across multiple pairings.
pub type G2Prepared = blstrs::G2Prepared;

/// Scalar field element for the BLS12-381 curve.
///
/// Used as exponents in scalar multiplication and other finite field operations.
pub type Scalar = blstrs::Scalar;

/// Checks whether the product of pairings over the given G1 × G2 pairs equals the identity.
pub fn multi_pairings(pairs: &[(&G1Point, &G2Prepared)]) -> bool {
    blstrs::Bls12::multi_miller_loop(pairs)
        .final_exponentiation()
        .is_identity()
        .into()
}

/// Converts Projective points to normalized points efficiently.
///
// Note: This efficient variation is needed here and not for G2 because it is called
// multiple times for MSM pre-computations.
pub fn g1_batch_normalize(projective_points: &[G1Projective]) -> Vec<G1Point> {
    if projective_points.is_empty() {
        return Vec::new();
    }

    // Track which points are identity and create a filtered vec without them
    //
    // This is because blst will convert all points into the identity point
    // if even one of them is the identity point.
    let mut identity_positions = Vec::new();
    let mut non_identity_points = Vec::new();

    for (idx, point) in projective_points.iter().enumerate() {
        if point.is_identity().into() {
            identity_positions.push(idx);
        } else {
            non_identity_points.push(*point);
        }
    }

    // If all points are identity, return a vector of identity points
    if non_identity_points.is_empty() {
        return vec![G1Point::identity(); projective_points.len()];
    }

    // Convert non-identity points to BLST representation and normalize
    let points = unsafe {
        std::slice::from_raw_parts(
            non_identity_points.as_ptr().cast::<blst::blst_p1>(),
            non_identity_points.len(),
        )
    };

    let normalized = blst::p1_affines::from(points);

    // Convert normalized points back to G1Points
    let mut result: Vec<_> = normalized
        .as_slice()
        .iter()
        .map(|p| G1Point::from_raw_unchecked(p.x.into(), p.y.into(), false))
        .collect();

    // Reinsert identity points at their original positions
    for pos in identity_positions {
        result.insert(pos, G1Point::identity());
    }

    result
}

/// Efficiently batch-normalizes a slice of G2 projective points to their affine representation.
///
/// This uses the generic `batch_normalize` method from the `Curve` trait to convert all
/// projective points into affine points in a single pass.
pub fn g2_batch_normalize(projective_points: &[G2Projective]) -> Vec<G2Point> {
    batch_normalize_points(projective_points)
}

/// Generic utility to batch-normalize projective points for any curve implementing `PrimeCurveAffine`.
///
/// Converts a slice of projective curve points into a vector of their affine form using
/// `Curve::batch_normalize`.
pub fn batch_normalize_points<T: PrimeCurveAffine>(points: &[T::Curve]) -> Vec<T>
where
    T::Curve: Curve<AffineRepr = T>,
{
    let mut affine_points = vec![T::identity(); points.len()];
    T::Curve::batch_normalize(points, &mut affine_points);
    affine_points
}

/// Reduces bytes to be a value less than the scalar modulus.
pub fn reduce_bytes_to_scalar_bias(bytes: [u8; 32]) -> Scalar {
    let mut out = blst::blst_fr::default();

    unsafe {
        // Convert byte array into a scalar
        let mut s = blst::blst_scalar::default();
        blst::blst_scalar_from_bendian(&raw mut s, bytes.as_ptr());
        // Convert scalar into a `blst_fr` reducing the value along the way
        blst::blst_fr_from_scalar(&raw mut out, std::ptr::addr_of!(s));
    }

    Scalar::from(out)
}

#[cfg(test)]
mod tests {
    use rand::rngs::OsRng;

    use super::*;
    use crate::ff::Field;

    /// BLS12-381 scalar field modulus (r)
    const BLS12_381_R: [u8; 32] = [
        0x73, 0xED, 0xA7, 0x53, 0x29, 0x9D, 0x7D, 0x48, 0x33, 0x39, 0xD8, 0x08, 0x09, 0xA1, 0xD8,
        0x05, 0x53, 0xBD, 0xA4, 0x02, 0xFF, 0xFE, 0x5B, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
        0x00, 0x01,
    ];

    /// 2^256 - 1 mod r
    const TWO_256_MINUS_ONE_MOD_R: [u8; 32] = [
        0x18, 0x24, 0xB1, 0x59, 0xAC, 0xC5, 0x05, 0x6F, 0x99, 0x8C, 0x4F, 0xEF, 0xEC, 0xBC, 0x4F,
        0xF5, 0x58, 0x84, 0xB7, 0xFA, 0x00, 0x03, 0x48, 0x02, 0x00, 0x00, 0x00, 0x01, 0xFF, 0xFF,
        0xFF, 0xFD,
    ];

    #[test]
    fn test_reduce_bytes_to_scalar_edge_cases() {
        // Test case 1: Zero
        let zero_bytes = [0u8; 32];
        let result = reduce_bytes_to_scalar_bias(zero_bytes);
        assert_eq!(
            result,
            Scalar::ZERO,
            "Zero input should result in zero scalar"
        );

        // Test case 2: One
        let one_bytes = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ];
        let result = reduce_bytes_to_scalar_bias(one_bytes);
        assert_eq!(result, Scalar::ONE, "One input should result in one scalar");

        // Test case 3: r - 1 (maximum value in the field)
        let max_bytes = [
            0x73, 0xED, 0xA7, 0x53, 0x29, 0x9D, 0x7D, 0x48, 0x33, 0x39, 0xD8, 0x08, 0x09, 0xA1,
            0xD8, 0x05, 0x53, 0xBD, 0xA4, 0x02, 0xFF, 0xFE, 0x5B, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF,
            0x00, 0x00, 0x00, 0x00,
        ];

        let result = reduce_bytes_to_scalar_bias(max_bytes);
        assert_ne!(result, Scalar::ZERO, "r - 1 should not reduce to zero");
        assert_eq!(result, -Scalar::ONE, "r - 1 should equal -1 in the field");

        // Test case 4: r (should reduce to zero)
        let r_bytes = BLS12_381_R;
        let result = reduce_bytes_to_scalar_bias(r_bytes);
        assert_eq!(result, Scalar::ZERO, "r should reduce to zero");

        // Test case 5: r + 1 (should reduce to 1)
        let mut r_plus_one = BLS12_381_R;
        r_plus_one[31] += 1;
        let result = reduce_bytes_to_scalar_bias(r_plus_one);
        assert_eq!(result, Scalar::ONE, "r + 1 should reduce to 1");

        // Test case 6: 2^256 - 1 (maximum 32-byte value)
        let max_32_bytes = [0xFF; 32];
        let result = reduce_bytes_to_scalar_bias(max_32_bytes);
        let expected =
            Scalar::from_bytes_be(&TWO_256_MINUS_ONE_MOD_R).expect("value must be canonical");
        assert_eq!(
            result, expected,
            "2^256 - 1 should reduce to (2^256 - 1) mod r"
        );
    }

    #[test]
    fn test_batch_normalize_empty() {
        let empty: Vec<G1Projective> = vec![];
        let result = g1_batch_normalize(&empty);
        assert!(result.is_empty());
    }

    #[test]
    fn test_batch_normalize_identity() {
        let identity: Vec<G1Projective> = vec![
            G1Projective::identity(),
            G1Projective::generator(),
            G1Projective::identity(),
        ];
        let result = g1_batch_normalize(&identity);
        assert!(bool::from(result[0].is_identity()));
        assert!(bool::from(!result[1].is_identity()));
        assert!(bool::from(result[2].is_identity()));
    }

    #[test]
    fn test_batch_normalize_multiple() {
        use rand::thread_rng;
        let mut rng = thread_rng();
        let points: Vec<G1Projective> = (0..100).map(|_| G1Projective::random(&mut rng)).collect();

        let normalized = g1_batch_normalize(&points);

        assert_eq!(normalized.len(), points.len());
        for (norm, proj) in normalized.iter().zip(points.iter()) {
            assert_eq!(*norm, G1Point::from(*proj));
        }
    }

    #[test]
    fn test_pairing_with_negation_false() {
        let g1 = G1Point::generator();
        let g2 = G2Point::generator();
        let g2_prep = G2Prepared::from(g2);
        let g1_neg = -g1;

        // e(g1, g2) * e(-g1, g2) => check returns true
        assert!(multi_pairings(&[(&g1, &g2_prep), (&g1_neg, &g2_prep)]));

        // e(g1, g2)^2 != identity => check returns false
        assert!(!multi_pairings(&[(&g1, &g2_prep), (&g1, &g2_prep)]));
    }

    #[test]
    fn test_identity_pairing_true() {
        let id_g1 = G1Point::identity();
        let g2 = G2Prepared::from(G2Point::generator());

        assert!(multi_pairings(&[(&id_g1, &g2)]));
    }

    #[test]
    fn test_g2_batch_normalize_empty() {
        let input: Vec<G2Projective> = vec![];
        let result = g2_batch_normalize(&input);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_g2_batch_normalize_only_identities() {
        let input = vec![G2Projective::identity(); 5];
        let result = g2_batch_normalize(&input);
        assert_eq!(result.len(), 5);
        assert!(result.iter().all(|p| p.is_identity().into()));
    }

    #[test]
    fn test_g2_batch_normalize_mixed_points() {
        let input = vec![
            G2Projective::identity(),
            G2Projective::generator(),
            G2Projective::identity(),
            G2Projective::generator() * Scalar::from(2u64),
            G2Projective::identity(),
        ];

        let result = g2_batch_normalize(&input);

        assert_eq!(result.len(), input.len());

        assert!(bool::from(result[0].is_identity()));
        assert!(!bool::from(result[1].is_identity()));
        assert!(bool::from(result[2].is_identity()));
        assert!(!bool::from(result[3].is_identity()));
        assert!(bool::from(result[4].is_identity()));

        // Verify correctness of actual non-identity normalization
        assert_eq!(result[1], G2Point::from(G2Projective::generator()));
        assert_eq!(
            result[3],
            G2Point::from(G2Projective::generator() * Scalar::from(2u64))
        );
    }

    #[test]
    fn test_g2_batch_normalize_random_points() {
        let mut rng = OsRng;
        let projective_points: Vec<G2Projective> =
            (0..10).map(|_| G2Projective::random(&mut rng)).collect();
        let result = g2_batch_normalize(&projective_points);

        assert_eq!(result.len(), projective_points.len());
        for (proj, affine) in projective_points.iter().zip(result.iter()) {
            assert_eq!(G2Point::from(*proj), *affine);
        }
    }
}
