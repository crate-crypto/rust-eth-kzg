pub mod batch_inversion;

// Re-exporting the blstrs crate
// TODO: We could feature flag the different bls12-381 implementations

// Re-export ff and group, so other crates do not need to directly import(and independently version) them
pub use group;
pub use ff;

pub type G1Point = blstrs::G1Affine;
pub type G1Projective = blstrs::G1Projective;

pub type G2Point = blstrs::G2Affine;
pub type G2Projective = blstrs::G2Projective;

pub type Scalar = blstrs::Scalar;

pub type KZGCommitment = G1Point;

/// The number of bytes needed to represent a scalar
pub const SCALAR_SERIALIZED_SIZE: usize = 32;
/// The number of bytes needed to represent a compressed G1 point
pub const G1_POINT_SERIALIZED_SIZE: usize = 48;
/// The number of bytes needed to represent a compressed G2 point
pub const G2_POINT_SERIALIZED_SIZE: usize = 96;
