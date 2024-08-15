pub mod constants;
mod errors;
mod prover;
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
/// BlobRef denotes a references to an opaque Blob.
///
/// Note: This library never returns a Blob, which is why we
/// do not have a Blob type.
pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];

/// Bytes48Ref denotes a reference to an untrusted cryptographic type
/// that can be represented in 48 bytes. This will be either a
/// purported KZGProof or a purported KZGCommitment.
pub type Bytes48Ref<'a> = &'a [u8; 48];

/// Cell contains a group of evaluations on a coset that one would like to
/// make and verify opening proofs about.
///
/// Note: These are heap allocated.
pub type Cell = Box<[u8; BYTES_PER_CELL]>;

/// CellRef contains a reference to a Cell.
///
/// Note: Similar to Blob, the library takes in references
/// to Cell and returns heap allocated instances as return types.
pub type CellRef<'a> = &'a [u8; BYTES_PER_CELL];

/// KZGProof denotes a 48 byte commitment to a polynomial
/// that one can use to prove that a polynomial f(x) was
/// correctly evaluated on a coset `H` and returned a set of points.
pub type KZGProof = [u8; BYTES_PER_COMMITMENT];

/// KZGCommitment denotes a 48 byte commitment to a polynomial f(x)
/// that we would like to make and verify opening proofs about.
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];

/// CellIndex is reference to a Coset.
///
/// We are able to use CellIndex instead of the coset because
/// the prover and verifier both know what the cosets are that
/// we will be making and verifying opening proofs for.
pub type CellIndex = u64;

/// CommitmentIndex is a reference to a commitment.
///
/// In order to make verification cheaper, the verifier will
/// deduplicate the list of commitments that they need to verify opening proofs for.
/// They will then refer to a commitment via its position in an array of deduplicated commitments
/// with the CommitmentIndex.
///
/// Note: This is not exposed in the public API.
pub(crate) type CommitmentIndex = u64;

use constants::{BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT};
use prover::ProverContext;
use rayon::ThreadPool;
use std::sync::Arc;
use verifier::VerifierContext;

/// The context that will be used to create and verify opening proofs.
#[derive(Debug)]
pub struct DASContext {
    thread_pool: Arc<ThreadPool>,
    pub prover_ctx: ProverContext,
    pub verifier_ctx: VerifierContext,
}

impl Default for DASContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        const DEFAULT_NUM_THREADS: usize = 1;
        DASContext::with_threads(&trusted_setup, DEFAULT_NUM_THREADS, UsePrecomp::No)
    }
}

impl DASContext {
    pub fn with_threads(
        trusted_setup: &TrustedSetup,
        num_threads: usize,
        // This parameter indicates whether we should allocate memory
        // in order to speed up proof creation. Heuristics show that
        // if pre-computations are desired, one should set the
        // width value to `8` for optimal storage and performance tradeoffs.
        use_precomp: UsePrecomp,
    ) -> Self {
        let thread_pool = std::sync::Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );

        DASContext {
            thread_pool,
            prover_ctx: ProverContext::new(trusted_setup, use_precomp),
            verifier_ctx: VerifierContext::new(trusted_setup),
        }
    }

    pub fn prover_ctx(&self) -> &ProverContext {
        &self.prover_ctx
    }

    pub fn verifier_ctx(&self) -> &VerifierContext {
        &self.verifier_ctx
    }
}
