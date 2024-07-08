pub use crate::errors::ProverError;

use kzg_multi_open::{
    commit_key::{CommitKey, CommitKeyLagrange},
    fk20::FK20,
};

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{self, serialize_g1_compressed},
    trusted_setup::TrustedSetup,
    BlobRef, Cell, CellIndex, CellRef, KZGCommitment, KZGProof, PeerDASContext,
};

/// Context object that is used to call functions in the prover API.
/// This includes, computing the commitments, proofs and cells.
#[derive(Debug)]
pub struct ProverContext {
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
}

impl Default for ProverContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        Self::new(&trusted_setup)
    }
}

impl ProverContext {
    pub fn new(trusted_setup: &TrustedSetup) -> Self {
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
        }
    }
}

impl PeerDASContext {
    /// Computes the KZG commitment to the polynomial represented by the blob.
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, ProverError> {
        self.thread_pool.install(|| {
            // Deserialize the blob into scalars.
            let scalars = serialization::deserialize_blob_to_scalars(blob)?;

            // Compute commitment using FK20
            let commitment = FK20::commit_to_data(&self.prover_ctx.commit_key_lagrange, scalars);

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
            // Deserialization
            //
            let scalars = serialization::deserialize_blob_to_scalars(blob)?;

            // Computation
            //
            let (proofs, cells) = self
                .prover_ctx
                .fk20
                .compute_multi_opening_proofs_on_data(scalars);

            Ok(serialization::serialize_cells_and_proofs(cells, proofs))
        })
    }

    /// Recovers the cells and computes the KZG proofs, given a subset of cells.
    ///
    /// Use erasure decoding to recover the polynomial corresponding to the cells
    /// that were generated from fk20.
    ///
    // Note: The fact that we recover the polynomial for the bit-reversed version of the blob
    // is irrelevant.
    pub fn recover_cells_and_proofs(
        &self,
        cell_indices: Vec<CellIndex>,
        cells: Vec<CellRef>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        self.thread_pool.install(|| {
            // Recover polynomial
            //
            let poly_coeff = self.recover_polynomial_coeff(cell_indices, cells)?;

            // Compute proofs and evaluation sets
            //
            let (proofs, evaluation_sets) = self
                .prover_ctx
                .fk20
                .compute_multi_opening_proofs_poly_coeff(poly_coeff);

            Ok(serialization::serialize_cells_and_proofs(
                evaluation_sets,
                proofs,
            ))
        })
    }
}
