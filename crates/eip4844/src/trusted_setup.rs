use kzg_single_open::{prover::CommitKey, verifier::VerificationKey};
pub use trusted_setup::TrustedSetup;

pub fn commit_key_from_setup(setup: &TrustedSetup) -> CommitKey {
    CommitKey::new(setup.g1_monomial.clone())
}

pub fn verification_key_from_setup(setup: &TrustedSetup) -> VerificationKey {
    VerificationKey {
        gen_g1: setup.g1_monomial[0],
        gen_g2: setup.g2_monomial[0],
        tau_g2: setup.g2_monomial[1],
    }
}
