use kzg_multi_open::{
    commit_key::CommitKey,
    {Prover, ProverInput},
};

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    errors::Error,
    serialization::{
        deserialize_blob_to_scalars, serialize_cells_and_proofs, serialize_g1_compressed,
    },
    trusted_setup::TrustedSetup,
    BlobRef, Cell, CellIndex, CellRef, KZGCommitment, KZGProof, PeerDASContext,
};

/// Context object that is used to call functions in the prover API.
/// This includes, computing the commitments, proofs and cells.
#[derive(Debug)]
pub struct ProverContext {
    kzg_multipoint_prover: Prover,
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

        let kzg_multipoint_prover = Prover::new(
            commit_key,
            FIELD_ELEMENTS_PER_BLOB,
            point_set_size,
            number_of_points_to_open,
        );

        ProverContext {
            kzg_multipoint_prover,
        }
    }
}

impl PeerDASContext {
    /// Computes the KZG commitment to the polynomial represented by the blob.
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, Error> {
        self.thread_pool.install(|| {
            // Deserialize the blob into scalars.
            let scalars = deserialize_blob_to_scalars(blob)?;

            // Compute commitment
            let commitment = self
                .prover_ctx
                .kzg_multipoint_prover
                .commit(ProverInput::Data(scalars));

            // Serialize the commitment.
            Ok(serialize_g1_compressed(&commitment))
        })
    }

    /// Computes the cells and the KZG proofs for the given blob.
    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: BlobRef,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), Error> {
        self.thread_pool.install(|| {
            // Deserialization
            //
            let scalars = deserialize_blob_to_scalars(blob)?;

            // Computation
            //
            let (proofs, cells) = self
                .prover_ctx
                .kzg_multipoint_prover
                .compute_multi_opening_proofs(ProverInput::Data(scalars));

            Ok(serialize_cells_and_proofs(cells, proofs))
        })
    }

    /// Recovers the cells and computes the KZG proofs, given a subset of cells.
    ///
    /// Use erasure decoding to recover the polynomial corresponding to the cells
    /// that were generated from KZG multi point prover.
    ///
    // Note: The fact that we recover the polynomial for the bit-reversed version of the blob
    // is irrelevant.
    pub fn recover_cells_and_proofs(
        &self,
        cell_indices: Vec<CellIndex>,
        cells: Vec<CellRef>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), Error> {
        self.thread_pool.install(|| {
            // Recover polynomial
            //
            let poly_coeff = self.recover_polynomial_coeff(cell_indices, cells)?;

            // Compute proofs and evaluation sets
            //
            let (proofs, coset_evaluations) = self
                .prover_ctx
                .kzg_multipoint_prover
                .compute_multi_opening_proofs(ProverInput::PolyCoeff(poly_coeff));

            Ok(serialize_cells_and_proofs(coset_evaluations, proofs))
        })
    }
}
