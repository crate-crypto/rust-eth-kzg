mod errors;
mod prover;
mod trusted_setup;
mod verifier;

/// Re-exported types
pub use errors::{Error, SerializationError, VerifierError};
pub use serialization::{constants, types::*};
pub use trusted_setup::TrustedSetup;

#[rustfmt::skip]
// Note: adding rustfmt::skip so that `cargo fmt` does not mix the
// public re-exported types with the following private imports.
use kzg_single_open::{prover::Prover, verifier::Verifier};
use serialization::constants::FIELD_ELEMENTS_PER_BLOB;
use trusted_setup::{commit_key_from_setup, verification_key_from_setup};

#[derive(Debug)]
pub struct Context {
    prover: Prover,
    verifier: Verifier,
}

impl Default for Context {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();

        Self::new(&trusted_setup)
    }
}

impl Context {
    pub fn new(trusted_setup: &TrustedSetup) -> Self {
        Self {
            prover: Prover::new(
                FIELD_ELEMENTS_PER_BLOB,
                commit_key_from_setup(trusted_setup),
            ),
            verifier: Verifier::new(
                FIELD_ELEMENTS_PER_BLOB,
                verification_key_from_setup(trusted_setup),
            ),
        }
    }
}
