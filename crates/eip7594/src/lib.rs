#[cfg(all(feature = "singlethreaded", feature = "multithreaded"))]
compile_error!("`singlethreaded` and `multithreaded` cannot be enabled simultaneously");

pub mod constants;
mod errors;
mod prover;
mod recovery;
mod serialization;
mod trusted_setup;
mod verifier;

pub use bls12_381::fixed_base_msm::UsePrecomp;
// Exported types
//
pub use errors::Error;
/// TrustedSetup contains the Structured Reference String(SRS)
/// needed to make and verify proofs.
pub use trusted_setup::TrustedSetup;

/// `BlobRef` denotes a references to an opaque Blob.
///
/// Note: This library never returns a Blob, which is why we
/// do not have a Blob type.
pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];

/// `Bytes48Ref` denotes a reference to an untrusted cryptographic type
/// that can be represented in 48 bytes. This will be either a
/// purported `KZGProof` or a purported `KZGCommitment`.
pub type Bytes48Ref<'a> = &'a [u8; 48];

/// Cell contains a group of evaluations on a coset that one would like to
/// make and verify opening proofs about.
///
/// Note: These are heap allocated.
pub type Cell = Box<[u8; BYTES_PER_CELL]>;

/// `CellRef` contains a reference to a Cell.
///
/// Note: Similar to Blob, the library takes in references
/// to Cell and returns heap allocated instances as return types.
pub type CellRef<'a> = &'a [u8; BYTES_PER_CELL];

/// `KZGProof` denotes a 48 byte commitment to a polynomial
/// that one can use to prove that a polynomial f(x) was
/// correctly evaluated on a coset `H` and returned a set of points.
pub type KZGProof = [u8; BYTES_PER_COMMITMENT];

/// `KZGCommitment` denotes a 48 byte commitment to a polynomial f(x)
/// that we would like to make and verify opening proofs about.
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];

/// `CellIndex` is reference to the coset/set of points that were used to create that Cell,
/// on a particular polynomial, f(x).
///
/// Note: Since the verifier and prover both know what cosets will be used
/// to evaluate the polynomials being used in opening proofs, the protocol
/// only requires an index to reference them.
pub type CellIndex = kzg_multi_open::CosetIndex;

use constants::{BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT};
use prover::ProverContext;
use verifier::VerifierContext;

/// DASContext manages the shared environment for creating and
/// verifying KZG cell proofs used in PeerDAS (EIP-7594).
///
/// It holds:
/// - The prover context (for generating proofs),
/// - The verifier context (for checking proofs),
///
/// both initialized from the same trusted setup (SRS). This context is required
/// for sampling and validating data availability across blobs and cells without downloading all data.
#[derive(Debug)]
pub struct DASContext {
    /// Prover-side context:
    /// prepares and generates KZG cell proofs for blobs and cells.
    pub prover_ctx: ProverContext,

    /// Verifier-side context:
    /// verifies KZG cell proofs and ensures data integrity in PeerDAS.
    pub verifier_ctx: VerifierContext,
}

impl Default for DASContext {
    fn default() -> Self {
        Self::new(&TrustedSetup::default(), UsePrecomp::No)
    }
}

impl DASContext {
    /// Creates a new DASContext with both prover and verifier
    /// initialized from the given trusted setup (SRS).
    ///
    /// This context is used for generating and verifying KZG cell
    /// proofs as part of PeerDAS (EIP-7594), which enables
    /// data availability sampling without downloading all blob data.
    ///
    /// The `use_precomp` parameter controls whether prover-side
    /// precomputations are enabled. Enabling precomputations
    /// (typically with width 8) increases memory use but improves
    /// proof generation speed, making it suitable for performance-sensitive
    /// environments.
    ///
    /// # Arguments
    /// * `trusted_setup` — The shared structured reference string (SRS)
    ///   used to configure both prover and verifier contexts.
    /// * `use_precomp` — Whether to enable prover-side precomputations
    ///   for faster proof creation at the cost of extra memory.
    pub fn new(trusted_setup: &TrustedSetup, use_precomp: UsePrecomp) -> Self {
        Self {
            prover_ctx: ProverContext::new(trusted_setup, use_precomp),
            verifier_ctx: VerifierContext::new(trusted_setup),
        }
    }
}
