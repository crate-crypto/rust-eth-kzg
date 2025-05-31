pub use rust_eth_kzg::TrustedSetup;
use rust_eth_kzg::{CommitKey as DASCommitKey, VerificationKey as DASVerifyingKey};

use crate::kzg_open::{prover::CommitKey, verifier::VerificationKey};

impl From<&DASVerifyingKey> for VerificationKey {
    fn from(verifying_key: &DASVerifyingKey) -> Self {
        Self {
            gen_g1: verifying_key.g1s[0],
            gen_g2: verifying_key.g2s[0],
            tau_g2: verifying_key.g2s[1],
        }
    }
}

impl From<&DASCommitKey> for CommitKey {
    fn from(commit_key: &DASCommitKey) -> Self {
        Self {
            g1s: commit_key.g1s.clone(),
        }
    }
}
