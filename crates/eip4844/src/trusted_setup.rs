pub use rust_eth_kzg::TrustedSetup;

use crate::kzg_open::{prover::CommitKey, verifier::VerificationKey};

impl From<&TrustedSetup> for VerificationKey {
    fn from(setup: &TrustedSetup) -> Self {
        Self {
            gen_g1: setup.g1_monomial[0],
            gen_g2: setup.g2_monomial[0],
            tau_g2: setup.g2_monomial[1],
        }
    }
}

impl From<&TrustedSetup> for CommitKey {
    fn from(setup: &TrustedSetup) -> Self {
        Self {
            g1s: setup.g1_monomial.clone(),
        }
    }
}
