use bls12_381::fixed_base_msm::UsePrecomp;
use erasure_codes::ReedSolomon;
use kzg_multi_open::{commit_key::CommitKey, Prover, ProverInput};

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, EXPANSION_FACTOR, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    errors::Error,
    recovery::recover_polynomial_coeff,
    serialization::{deserialize_blob_to_scalars, serialize_cells, serialize_cells_and_proofs},
    trusted_setup::TrustedSetup,
    BlobRef, Cell, CellIndex, CellRef, DASContext, KZGCommitment, KZGProof,
};

/// Context object that is used to call functions in the prover API.
/// This includes, computing the commitments, proofs and cells.
#[derive(Debug)]
pub struct ProverContext {
    kzg_multipoint_prover: Prover,
    rs: ReedSolomon,
}

impl Default for ProverContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        Self::new(&trusted_setup, UsePrecomp::No)
    }
}

impl ProverContext {
    pub fn new(trusted_setup: &TrustedSetup, use_precomp: UsePrecomp) -> Self {
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
            use_precomp,
        );

        let rs = ReedSolomon::new(
            FIELD_ELEMENTS_PER_BLOB,
            EXPANSION_FACTOR,
            CELLS_PER_EXT_BLOB,
        );

        Self {
            kzg_multipoint_prover,
            rs,
        }
    }
}

impl DASContext {
    /// Computes the KZG commitment to the polynomial represented by the blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/deneb/polynomial-commitments.md#blob_to_kzg_commitment
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, Error> {
        // Deserialize the blob into scalars.
        let scalars = deserialize_blob_to_scalars(blob)?;

        // Compute commitment
        let commitment = self
            .prover_ctx
            .kzg_multipoint_prover
            .commit(ProverInput::Data(scalars));

        // Serialize the commitment.
        Ok(commitment.to_compressed())
    }

    /// Computes the cells and the KZG proofs for the given blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#compute_cells_and_kzg_proofs
    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: BlobRef,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), Error> {
        #[cfg(feature = "tracing")]
        let _span = tracing::info_span!("compute_cells_and_kzg_proofs").entered();

        // Deserialization
        //
        let scalars = deserialize_blob_to_scalars(blob)?;

        // Computation
        //
        let (proofs, cells) = self
            .prover_ctx
            .kzg_multipoint_prover
            .compute_multi_opening_proofs(ProverInput::Data(scalars));

        Ok(serialize_cells_and_proofs(cells, &proofs))
    }

    /// Computes the cells for the given blob.
    pub fn compute_cells(&self, blob: BlobRef) -> Result<[Cell; CELLS_PER_EXT_BLOB], Error> {
        // Deserialization
        //
        let scalars = deserialize_blob_to_scalars(blob)?;

        // Computation
        //
        let extended_blob = self
            .prover_ctx
            .kzg_multipoint_prover
            .extend_polynomial(ProverInput::Data(scalars));

        Ok(serialize_cells(extended_blob))
    }

    /// Recovers the cells and computes the KZG proofs, given a subset of cells.
    ///
    /// Use erasure decoding to recover the polynomial corresponding to the cells
    /// that were provided as input.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#recover_cells_and_kzg_proofs
    pub fn recover_cells_and_kzg_proofs(
        &self,
        cell_indices: Vec<CellIndex>,
        cells: Vec<CellRef>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), Error> {
        // Recover polynomial
        //
        let poly_coeff = recover_polynomial_coeff(&self.prover_ctx.rs, cell_indices, cells)?;

        // Compute proofs and evaluation sets
        //
        let (proofs, coset_evaluations) = self
            .prover_ctx
            .kzg_multipoint_prover
            .compute_multi_opening_proofs(ProverInput::PolyCoeff(poly_coeff));

        Ok(serialize_cells_and_proofs(coset_evaluations, &proofs))
    }
}
