pub use crate::errors::ProverError;

use bls12_381::{G1Point, Scalar};
use kzg_multi_open::{
    commit_key::{CommitKey, CommitKeyLagrange},
    create_eth_commit_opening_keys,
    fk20::FK20,
    polynomial::domain::Domain,
    reverse_bit_order,
};

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{self, serialize_g1_compressed},
    verifier::VerifierContext,
    BlobRef, Bytes48Ref, Cell, CellID, CellRef, KZGCommitment, KZGProof,
};

/// Context object that is used to call functions in the prover API.
/// This includes, computing the commitments, proofs and cells.
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

    /// Domain used for converting the polynomial to the monomial form.
    poly_domain: Domain,
    // Verifier context
    //
    // The prover needs the verifier context to recover the cells and then compute the proofs
    verifier_context: VerifierContext,
}

impl Default for ProverContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ProverContext {
    pub fn new() -> Self {
        let (commit_key, _) = create_eth_commit_opening_keys();
        // The number of points that we will make an opening proof for,
        // ie a proof will attest to the value of the polynomial at these points.
        let point_set_size = FIELD_ELEMENTS_PER_CELL;

        // The number of points that we will be making proofs for.
        //
        // Note: it is easy to calculate the number of proofs that we need to make
        // by doing number_of_points_to_open / point_set_size.
        let number_of_points_to_open = FIELD_ELEMENTS_PER_EXT_BLOB;

        let fk20 = FK20::new(&commit_key, point_set_size, number_of_points_to_open);

        let poly_domain = Domain::new(FIELD_ELEMENTS_PER_BLOB);

        // TODO: We can just deserialize these instead of doing this ifft
        let commit_key_lagrange = commit_key.clone().into_lagrange(&poly_domain);

        ProverContext {
            fk20,
            commit_key,
            poly_domain,
            commit_key_lagrange,
            verifier_context: VerifierContext::new(),
        }
    }

    /// Computes the KZG commitment to the polynomial represented by the blob.
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, ProverError> {
        // Deserialize the blob into scalars. The blob is in lagrange form.
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;

        // Reverse the order of the scalars, so that they are in normal order.
        // ie not in bit-reversed order.
        reverse_bit_order(&mut scalars);

        // Commit to the polynomial.
        let commitment: G1Point = self.commit_key_lagrange.commit_g1(&scalars).into();

        // Serialize the commitment.
        Ok(serialize_g1_compressed(&commitment))
    }

    /// Computes the cells and the KZG proofs for the given blob.
    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: BlobRef,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        // Deserialize the blob into scalars. The blob is in lagrange form.
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;

        // Reverse the order of the scalars, so that they are in normal order.
        // ie not in bit-reversed order.
        reverse_bit_order(&mut scalars);

        // Convert the polynomial from lagrange to monomial form.
        let poly_coeff = self.poly_domain.ifft_scalars(scalars);

        self.compute_cells_and_kzg_proofs_from_poly_coeff(poly_coeff)
    }

    fn compute_cells_and_kzg_proofs_from_poly_coeff(
        &self,
        poly_coeff: Vec<Scalar>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        // Compute the proofs and the evaluations of the polynomial.
        let (proofs, evaluation_sets) = self.fk20.compute_multi_opening_proofs(poly_coeff);

        // Serialize the evaluations into `Cell`s.
        let cells = evaluation_sets_to_cells(evaluation_sets.into_iter());

        // Serialize the proofs into `KZGProof`s.
        let proofs: Vec<_> = proofs.iter().map(serialize_g1_compressed).collect();
        let proofs: [KZGProof; CELLS_PER_EXT_BLOB] = proofs
            .try_into()
            .unwrap_or_else(|_| panic!("expected {} number of proofs", CELLS_PER_EXT_BLOB));

        Ok((cells, proofs))
    }

    #[deprecated(note = "This function is deprecated, use `compute_cells_and_kzg_proofs` instead")]
    pub fn compute_cells(&self, blob: BlobRef) -> Result<[Cell; CELLS_PER_EXT_BLOB], ProverError> {
        // Deserialize the blob into scalars. The blob is in lagrange form.
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;

        // Reverse the order of the scalars, so that they are in normal order.
        // ie not in bit-reversed order.
        reverse_bit_order(&mut scalars);

        // Convert the polynomial from lagrange to monomial form.
        let poly_coeff = self.poly_domain.ifft_scalars(scalars);

        // Compute the evaluations of the polynomial at the points that we need to make proofs for.
        let evaluation_sets = self.fk20.compute_evaluation_sets(poly_coeff);

        // Serialize the evaluations into cells.
        let cells = evaluation_sets_to_cells(evaluation_sets.into_iter());

        Ok(cells)
    }

    /// Recovers the cells and computes the KZG proofs, given a subset of cells.
    #[allow(deprecated)]
    pub fn recover_cells_and_proofs(
        &self,
        cell_ids: Vec<CellID>,
        cells: Vec<CellRef>,
        _proofs: Vec<Bytes48Ref>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        // Use erasure decoding to recover the polynomial corresponding to the blob in monomial form
        let poly_coeff = self
            .verifier_context
            .recover_polynomial_coeff(cell_ids, cells)
            .map_err(ProverError::RecoveryFailure)?;

        self.compute_cells_and_kzg_proofs_from_poly_coeff(poly_coeff)
    }
}

/// Converts a a set of scalars (evaluations) to the `Cell` type
pub(crate) fn evaluation_sets_to_cells<T: AsRef<[Scalar]>>(
    evaluations: impl Iterator<Item = T>,
) -> [Cell; CELLS_PER_EXT_BLOB] {
    let cells: Vec<Cell> = evaluations
        .map(|eval| serialization::serialize_scalars_to_cell(eval.as_ref()))
        .map(|cell| {
            cell.into_boxed_slice()
                .try_into()
                .expect("infallible: Vec<u8> should have length equal to BYTES_PER_CELL")
        })
        .collect();

    cells
        .try_into()
        .unwrap_or_else(|_| panic!("expected {} number of cells", CELLS_PER_EXT_BLOB))
}
