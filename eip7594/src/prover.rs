use std::sync::Arc;

pub use crate::errors::ProverError;

use kzg_multi_open::{
    commit_key::{CommitKey, CommitKeyLagrange},
    fk20::FK20,
};
use rayon::ThreadPool;

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{self, serialize_g1_compressed},
    trusted_setup::TrustedSetup,
    verifier::VerifierContext,
    BlobRef, Cell, CellID, CellRef, KZGCommitment, KZGProof,
};

/// Context object that is used to call functions in the prover API.
/// This includes, computing the commitments, proofs and cells.
#[derive(Debug)]
pub struct ProverContext {
    thread_pool: Arc<ThreadPool>,

    fk20: FK20,
    // TODO: We don't need the commit key, since we use FK20 to compute the proofs
    // TODO: and we use the lagrange variant to compute the commitment to the polynomial.
    //
    // TODO: We can remove it in a later commit, once the API has settled.
    #[allow(dead_code)]
    commit_key: CommitKey,
    /// This is only used to save us from doing an IDFT when committing
    /// to the polynomial.
    commit_key_lagrange: CommitKeyLagrange,

    // Verifier context
    //
    // The prover needs the verifier context to recover the cells and then compute the proofs
    verifier_context: VerifierContext,
}

impl Default for ProverContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        Self::new(&trusted_setup)
    }
}

impl ProverContext {
    pub fn new(trusted_setup: &TrustedSetup) -> Self {
        const DEFAULT_NUM_THREADS: usize = 16;
        Self::with_num_threads(trusted_setup, DEFAULT_NUM_THREADS)
    }

    pub fn with_num_threads(trusted_setup: &TrustedSetup, num_threads: usize) -> Self {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();
        Self::from_threads_pool(trusted_setup, Arc::new(thread_pool))
    }

    pub(crate) fn from_threads_pool(
        trusted_setup: &TrustedSetup,
        thread_pool: Arc<ThreadPool>,
    ) -> Self {
        let commit_key = CommitKey::from(trusted_setup);
        // The number of points that we will make an opening proof for,
        // ie a proof will attest to the value of a polynomial at these points.
        let point_set_size = FIELD_ELEMENTS_PER_CELL;

        // The number of points that we will be making proofs for.
        //
        // Note: it is easy to calculate the number of proofs that we need to make
        // by doing number_of_points_to_open / point_set_size.
        let number_of_points_to_open = FIELD_ELEMENTS_PER_EXT_BLOB;

        let fk20 = FK20::new(
            &commit_key,
            FIELD_ELEMENTS_PER_BLOB,
            point_set_size,
            number_of_points_to_open,
        );

        let commit_key_lagrange = CommitKeyLagrange::from(trusted_setup);

        ProverContext {
            fk20,
            commit_key,
            commit_key_lagrange,
            verifier_context: VerifierContext::from_thread_pool(trusted_setup, thread_pool.clone()),
            thread_pool,
        }
    }

    /// Computes the KZG commitment to the polynomial represented by the blob.
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, ProverError> {
        self.thread_pool.install(|| {
            // Deserialize the blob into scalars.
            let scalars = serialization::deserialize_blob_to_scalars(blob)
                .map_err(ProverError::Serialization)?;

            // Compute commitment using FK20
            let commitment = FK20::commit_to_data(&self.commit_key_lagrange, scalars);

            // Serialize the commitment.
            Ok(serialize_g1_compressed(&commitment))
        })
    }

    /// Computes the cells and the KZG proofs for the given blob.
    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: BlobRef,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        self.thread_pool.install(|| {
            // Deserialize the blob into scalars.
            let scalars = serialization::deserialize_blob_to_scalars(blob)
                .map_err(ProverError::Serialization)?;

            let (proofs, cells) = self.fk20.compute_multi_opening_proofs_on_data(scalars);

            Ok(serialization::serialize_cells_and_proofs(cells, proofs))
        })
    }

    /// Recovers the cells and computes the KZG proofs, given a subset of cells.
    pub fn recover_cells_and_proofs(
        &self,
        cell_ids: Vec<CellID>,
        cells: Vec<CellRef>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        self.thread_pool.install(|| {
            // Use erasure decoding to recover the polynomial corresponding to the cells
            // that were generated from fk20.
            //
            // Note: The fact that this is the polynomial for the bit-reversed version of the blob
            // is irrelevant.
            let poly_coeff = self
                .verifier_context
                .recover_polynomial_coeff(cell_ids, cells)
                .map_err(ProverError::RecoveryFailure)?;

            // Check the degree of the polynomial.
            assert_eq!(FIELD_ELEMENTS_PER_BLOB, poly_coeff.len());

            // Compute the proofs and the evaluation sets for the polynomial.
            let (proofs, evaluation_sets) =
                self.fk20.compute_multi_opening_proofs(poly_coeff.clone());

            Ok(serialization::serialize_cells_and_proofs(
                evaluation_sets,
                proofs,
            ))
        })
    }
}
