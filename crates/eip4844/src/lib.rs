#[cfg(all(feature = "singlethreaded", feature = "multithreaded"))]
compile_error!("`singlethreaded` and `multithreaded` cannot be enabled simultaneously");

use serialization::constants::FIELD_ELEMENTS_PER_BLOB;

mod errors;
mod prover;
mod trusted_setup;
pub(crate) mod verifier;

pub use errors::{Error, SerializationError, VerifierError};
use kzg_single_open::{prover::Prover, verifier::Verifier};
pub use serialization::{constants, types::*};
pub use trusted_setup::TrustedSetup;
use trusted_setup::{commit_key_from_setup, verification_key_from_setup};

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
