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
use group::{prime::PrimeCurveAffine, Curve};

pub type G1Point = blstrs::G1Affine;
pub type G1Projective = blstrs::G1Projective;

pub type G2Point = blstrs::G2Affine;
pub type G2Projective = blstrs::G2Projective;
// This is needed for pairings. We want to give downstream users
// the ability to cache this for subsequent pairings.
pub type G2Prepared = blstrs::G2Prepared;

pub type Scalar = blstrs::Scalar;

pub fn multi_pairings(pairs: &[(&G1Point, &blstrs::G2Prepared)]) -> bool {
    use group::Group;
    use pairing::{MillerLoopResult, MultiMillerLoop};
    let pairing_ = blstrs::Bls12::multi_miller_loop(pairs).final_exponentiation();
    pairing_.is_identity().into()
}

/// Converts Projective points to normalized points efficiently.
///
// Note: This efficient variation is needed here and not for G2 because it is called
// multiple times for MSM pre-computations.
pub fn g1_batch_normalize(projective_points: &[G1Projective]) -> Vec<G1Point> {
    use group::Group;

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
    let mut result: Vec<G1Point> = normalized
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
pub fn g2_batch_normalize(projective_points: &[G2Projective]) -> Vec<G2Point> {
    batch_normalize_points(projective_points)
}

pub fn batch_normalize_points<T: PrimeCurveAffine>(points: &[T::Curve]) -> Vec<T>
where
    T::Curve: Curve<AffineRepr = T>,
{
    let mut affine_points = vec![T::identity(); points.len()];
    T::Curve::batch_normalize(points, &mut affine_points);
    affine_points
}

// Reduces bytes to be a value less than the scalar modulus.
#[allow(clippy::borrow_as_ptr)]
pub fn reduce_bytes_to_scalar_bias(bytes: [u8; 32]) -> Scalar {
    let mut out = blst::blst_fr::default();

    unsafe {
        // Convert byte array into a scalar
        let mut s = blst::blst_scalar::default();
        blst::blst_scalar_from_bendian(&mut s, &bytes as *const u8);
        // Convert scalar into a `blst_fr` reducing the value along the way
        blst::blst_fr_from_scalar(&mut out, std::ptr::addr_of!(s));
    }

    Scalar::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ff::Field;
    use group::Group;

    // BLS12-381 scalar field modulus (r)
    const BLS12_381_R: [u8; 32] = [
        0x73, 0xED, 0xA7, 0x53, 0x29, 0x9D, 0x7D, 0x48, 0x33, 0x39, 0xD8, 0x08, 0x09, 0xA1, 0xD8,
        0x05, 0x53, 0xBD, 0xA4, 0x02, 0xFF, 0xFE, 0x5B, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
        0x00, 0x01,
    ];

    // 2^256 - 1 mod r
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
        let expected = Scalar::from_bytes_be(&TWO_256_MINUS_ONE_MOD_R).unwrap();
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
}
