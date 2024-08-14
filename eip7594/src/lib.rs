pub mod constants;
mod errors;
mod prover;
mod serialization;
mod trusted_setup;
mod verifier;

// Exported types
//
pub use errors::Error;
pub use trusted_setup::TrustedSetup;
pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];
pub type Bytes48Ref<'a> = &'a [u8; 48];

pub type Cell = Box<[u8; BYTES_PER_CELL]>;
pub type CellRef<'a> = &'a [u8; BYTES_PER_CELL];

pub type KZGProof = [u8; BYTES_PER_COMMITMENT];
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];
pub type CellIndex = u64;
pub type RowIndex = u64;

use constants::{BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT};
use prover::ProverContext;
use rayon::ThreadPool;
use std::sync::Arc;
use verifier::VerifierContext;

/// The context that will be used to create and verify proofs.
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
        DASContext::with_threads(&trusted_setup, DEFAULT_NUM_THREADS)
    }
}

impl DASContext {
    pub fn with_threads(trusted_setup: &TrustedSetup, num_threads: usize) -> Self {
        let thread_pool = std::sync::Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );

        DASContext {
            thread_pool,
            prover_ctx: ProverContext::new(trusted_setup),
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
