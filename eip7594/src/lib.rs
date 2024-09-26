#[cfg(all(feature = "singlethreaded", feature = "multithreaded"))]
compile_error!("feature_a and feature_b cannot be enabled simultaneously");

pub mod constants;
mod errors;
mod prover;
mod recovery;
mod serialization;
mod trusted_setup;
mod verifier;
#[macro_use]
pub(crate) mod macros;

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

/// CellIndex is reference to the coset/set of points that were used to create that Cell,
/// on a particular polynomial, f(x).
///
/// Note: Since the verifier and prover both know what cosets will be used
/// to evaluate the polynomials being used in opening proofs, the protocol
/// only requires an index to reference them.
pub type CellIndex = kzg_multi_open::CosetIndex;

use constants::{BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT};
use prover::ProverContext;
use verifier::VerifierContext;

#[cfg(feature = "multithreaded")]
use rayon::ThreadPool;
#[cfg(feature = "multithreaded")]
use std::sync::Arc;

/// ThreadCount indicates whether we want to use a single thread or multiple threads
#[derive(Debug, Copy, Clone)]
pub enum ThreadCount {
    /// Initializes the threadpool with a single thread
    Single,
    /// Initializes the threadpool with the number of threads
    /// denoted by this enum variant.
    #[cfg(feature = "multithreaded")]
    Multi(usize),
    /// Initializes the threadpool with a sensible default number of
    /// threads. This is currently set to `RAYON_NUM_THREADS`.
    #[cfg(feature = "multithreaded")]
    SensibleDefault,
}

impl From<ThreadCount> for usize {
    fn from(value: ThreadCount) -> Self {
        match value {
            ThreadCount::Single => 1,
            #[cfg(feature = "multithreaded")]
            ThreadCount::Multi(num_threads) => num_threads,
            // Setting this to `0` will tell ThreadPool to use
            // `RAYON_NUM_THREADS`.
            #[cfg(feature = "multithreaded")]
            ThreadCount::SensibleDefault => 0,
        }
    }
}

/// The context that will be used to create and verify opening proofs.
#[derive(Debug)]
pub struct DASContext {
    #[cfg(feature = "multithreaded")]
    thread_pool: Arc<ThreadPool>,
    pub prover_ctx: ProverContext,
    pub verifier_ctx: VerifierContext,
}

#[cfg(feature = "multithreaded")]
impl Default for DASContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        const DEFAULT_NUM_THREADS: ThreadCount = ThreadCount::Single;
        DASContext::with_threads(&trusted_setup, DEFAULT_NUM_THREADS, UsePrecomp::No)
    }
}
#[cfg(not(feature = "multithreaded"))]
impl Default for DASContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();

        DASContext::new(&trusted_setup, UsePrecomp::No)
    }
}

impl DASContext {
    #[cfg(feature = "multithreaded")]
    pub fn with_threads(
        trusted_setup: &TrustedSetup,
        num_threads: ThreadCount,
        use_precomp: UsePrecomp,
    ) -> Self {
        #[cfg(feature = "multithreaded")]
        let thread_pool = std::sync::Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads.into())
                .build()
                .unwrap(),
        );

        DASContext {
            #[cfg(feature = "multithreaded")]
            thread_pool,
            prover_ctx: ProverContext::new(trusted_setup, use_precomp),
            verifier_ctx: VerifierContext::new(trusted_setup),
        }
    }

    #[cfg(not(feature = "multithreaded"))]
    pub fn new(
        trusted_setup: &TrustedSetup,
        // This parameter indicates whether we should allocate memory
        // in order to speed up proof creation. Heuristics show that
        // if pre-computations are desired, one should set the
        // width value to `8` for optimal storage and performance tradeoffs.
        use_precomp: UsePrecomp,
    ) -> Self {
        DASContext {
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
