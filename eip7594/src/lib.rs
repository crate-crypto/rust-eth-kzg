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

// TODO: We require a bit of feedback re usage to know whether we should make
// TODO: Cell type just be Vec<u8> -- This would avoid accidental stack overflows.
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

#[cfg(test)]
mod tests {
    use super::*;
    use bls12_381::Scalar;
    #[test]
    fn stack_overflow_poc() {
        use rayon::prelude::*;

        fn compute_blob(offset: u64) -> Vec<u8> {
            let poly: Vec<_> = (0..4096).map(|i| -Scalar::from(i + offset)).collect();
            let blob: Vec<_> = poly
                .into_iter()
                .flat_map(|scalar| scalar.to_bytes_be())
                .collect();
            blob
        }

        const NUM_BLOBS: u64 = 100;
        let blobs = (0..NUM_BLOBS).map(compute_blob).collect::<Vec<_>>();

        let trusted_setup = TrustedSetup::default();

        let ctx = DASContext::with_threads(&trusted_setup, 8);
        let blob_cells_and_proofs_vec: Vec<_> = blobs
            .par_iter()
            .map(|blob_vec| {
                // let mut blob = [0; BYTES_PER_BLOB];
                // blob.copy_from_slice(&blob_vec);
                // let cells_and_proofs = ctx.compute_cells_and_kzg_proofs(&blob);

                let cells_and_proofs =
                    ctx.compute_cells_and_kzg_proofs(blob_vec.as_slice().try_into().unwrap());

                cells_and_proofs
            })
            .collect();

        std::hint::black_box(blob_cells_and_proofs_vec);
    }
}
