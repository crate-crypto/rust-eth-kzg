pub mod batch_inversion;
pub mod fixed_base_msm;
pub mod lincomb;

// Re-exporting the blstrs crate

// Re-export ff and group, so other crates do not need to directly import(and independently version) them
pub use ff;
pub use group;

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

// TODO: Use batch_inversion trick to speed this up
pub fn g1_batch_normalize(projective_points: &[G1Projective]) -> Vec<G1Point> {
    use group::prime::PrimeCurveAffine;
    use group::Curve;

    let mut affine_points = vec![G1Point::identity(); projective_points.len()];
    G1Projective::batch_normalize(projective_points, &mut affine_points);

    affine_points
}

// TODO: instead of truncation, we can use blst's api to
// convert 32 bytes to a blst scalar and then convert from a scalar to an Fr
pub fn reduce_bytes_to_scalar_bias(mut bytes: [u8; 32]) -> Scalar {
    bytes[0] = (bytes[0] << 2) >> 2;
    Scalar::from_bytes_be(&bytes).expect("254 bit integer should have been reducible to a scalar")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ff::Field;

    #[test]
    fn test_reduce_bytes_to_scalar_edge_cases() {
        // We essentially are testing edge cases to ensure that the reduction works.

        // Test case 1: Normal case
        let input_bytes = [
            0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let result = reduce_bytes_to_scalar_bias(input_bytes);
        let expected = Scalar::from_bytes_be(&[
            0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ])
        .unwrap();
        assert_eq!(result, expected);

        // Test case 2: All zeros
        let input_bytes = [0u8; 32];
        let result = reduce_bytes_to_scalar_bias(input_bytes);
        assert_eq!(result, Scalar::ZERO);

        // Test case 3: Maximum value after reduction
        let input_bytes = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let result = reduce_bytes_to_scalar_bias(input_bytes);
        let expected = Scalar::from_bytes_be(&[
            0x3F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF,
        ])
        .unwrap();
        assert_eq!(result, expected);
    }
}
