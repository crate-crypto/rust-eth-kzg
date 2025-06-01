use kzg_multi_open::{commit_key::CommitKey, verification_key::VerificationKey};
pub use trusted_setup::TrustedSetup;

use crate::constants::{FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL};

pub fn commit_key_from_setup(setup: &TrustedSetup) -> CommitKey {
    CommitKey::new(setup.g1_monomial.clone())
}

pub fn verification_key_from_setup(setup: &TrustedSetup) -> VerificationKey {
    let g2_points = setup.g2_monomial.clone();
    let num_g2_points = g2_points.len();
    // The setup needs as many g1 elements for the verification key as g2 elements, in order
    // to commit to the remainder/interpolation polynomial.
    let g1_points = setup.g1_monomial[..num_g2_points].to_vec();

    VerificationKey::new(
        g1_points,
        g2_points,
        FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_BLOB,
    )
}
