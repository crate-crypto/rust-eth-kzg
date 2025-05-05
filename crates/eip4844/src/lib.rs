use constants::{BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT};

mod errors;
mod prover;
mod verifier;

#[allow(dead_code)]
#[path = "../../eip7594/src/constants.rs"]
pub mod constants;
#[allow(dead_code)]
#[path = "../../eip7594/src/serialization.rs"]
mod serialization;
#[allow(dead_code)]
#[path = "../../eip7594/src/trusted_setup.rs"]
mod trusted_setup;

pub use errors::{Error, ProverError, SerializationError, VerifierError};
use prover::Prover;
pub use trusted_setup::TrustedSetup;
use verifier::Verifier;

/// BlobRef denotes a references to an opaque Blob.
///
/// Note: This library never returns a Blob, which is why we
/// do not have a Blob type.
pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];

/// KZGCommitment denotes a 48 byte commitment to a polynomial f(x)
/// that we would like to make and verify opening proofs about.
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];

// TODO: Remove me.
pub type Cell = Box<[u8; BYTES_PER_CELL]>;

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
            prover: Prover::new(trusted_setup),
            verifier: Verifier::new(trusted_setup),
        }
    }
}
