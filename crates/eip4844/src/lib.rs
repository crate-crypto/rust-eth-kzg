#[cfg(all(feature = "singlethreaded", feature = "multithreaded"))]
compile_error!("`singlethreaded` and `multithreaded` cannot be enabled simultaneously");

use constants::{
    BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB,
};

mod errors;
mod prover;
pub(crate) mod verifier;

pub mod constants;
mod kzg_open;
mod serialization;
mod trusted_setup;

pub use errors::{Error, SerializationError, VerifierError};
//
pub use rust_eth_kzg::TrustedSetup;

use crate::kzg_open::{prover::CommitKey, verifier::VerificationKey};
//
use crate::kzg_open::{prover::Prover, verifier::Verifier};

/// BlobRef denotes a references to an opaque Blob.
///
/// Note: This library never returns a Blob, which is why we
/// do not have a Blob type.
pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];

/// KZGCommitment denotes a 48 byte commitment to a polynomial f(x)
/// that we would like to make and verify opening proofs about.
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];

/// KZGProof denotes a 48 byte commitment to a polynomial
/// that one can use to prove that a polynomial f(x) was
/// correctly evaluated on a coset `H` and returned a set of points.
pub type KZGProof = [u8; BYTES_PER_COMMITMENT];

/// KZGOpeningPoint denotes a 32 byte of a scalar to be evaluated at.
pub type KZGOpeningPoint = [u8; BYTES_PER_FIELD_ELEMENT];

/// KZGOpeningEvaluation denotes a 32 byte of a scalar that evaluated at certain
/// point.
pub type KZGOpeningEvaluation = [u8; BYTES_PER_FIELD_ELEMENT];

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
            prover: Prover::new(FIELD_ELEMENTS_PER_BLOB, CommitKey::from(trusted_setup)),
            verifier: Verifier::new(
                FIELD_ELEMENTS_PER_BLOB,
                VerificationKey::from(trusted_setup),
            ),
        }
    }
}
