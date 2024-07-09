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
