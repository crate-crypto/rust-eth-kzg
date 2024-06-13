use bls12_381::G1Point;
use kzg_multi_open::{
    commit_key::{CommitKey, CommitKeyLagrange},
    create_eth_commit_opening_keys,
    fk20::FK20,
    polynomial::domain::Domain,
    reverse_bit_order,
};

use crate::{
    constants::{
        BYTES_PER_BLOB, CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{self, serialize_g1_compressed, SerializationError},
    verifier::{VerifierContext, VerifierError},
    BlobRef, Bytes48Ref, Cell, CellID, CellRef, KZGCommitment, KZGProof,
};

#[derive(Debug)]
pub enum ProverError {
    Serialization(SerializationError),
    RecoveryFailure(VerifierError),
}

pub struct ProverContext {
    fk20: FK20,
    // TODO: We don't need the commit key, since we use FK20 to compute the proofs
    // TODO: and we use the lagrange variant to compute the commitment to the polynomial.
    //
    // TODO: We can remove it in a later commit, once the API has settled.
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

impl ProverContext {
    pub fn new() -> Self {
        let (commit_key, _) = create_eth_commit_opening_keys();
        let point_set_size = FIELD_ELEMENTS_PER_CELL;
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

    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, ProverError> {
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;
        reverse_bit_order(&mut scalars);

        let commitment: G1Point = self.commit_key_lagrange.commit_g1(&scalars).into();
        Ok(serialize_g1_compressed(&commitment))
    }

    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: BlobRef,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        // Deserialize the blob into scalars (lagrange form)
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;
        reverse_bit_order(&mut scalars);

        let poly_coeff = self.poly_domain.ifft_scalars(scalars);
        let (proofs, evaluations) = self.fk20.compute_multi_opening_proofs(poly_coeff);

        let cells = evaluations
            .iter()
            .map(|eval| serialization::serialize_scalars_to_cell(eval))
            .collect::<Vec<_>>();
        let cells: [Cell; CELLS_PER_EXT_BLOB] = cells
            .try_into()
            .expect(&format!("expected {} number of cells", CELLS_PER_EXT_BLOB));

        let proofs: Vec<_> = proofs.iter().map(serialize_g1_compressed).collect();
        let proofs: [KZGProof; CELLS_PER_EXT_BLOB] = proofs
            .try_into()
            .expect(&format!("expected {} number of proofs", CELLS_PER_EXT_BLOB));

        Ok((cells, proofs))
    }

    pub fn compute_cells(&self, blob: BlobRef) -> Result<[Cell; CELLS_PER_EXT_BLOB], ProverError> {
        // Deserialize the blob into scalars (lagrange form)
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;
        reverse_bit_order(&mut scalars);

        let poly_coeff = self.poly_domain.ifft_scalars(scalars);
        let evaluations = self.fk20.compute_evaluation_sets(poly_coeff);

        let cells = evaluations
            .iter()
            .map(|eval| serialization::serialize_scalars_to_cell(eval))
            .collect::<Vec<_>>();
        let cells: [Cell; CELLS_PER_EXT_BLOB] = cells
            .try_into()
            .expect(&format!("expected {} number of cells", CELLS_PER_EXT_BLOB));

        Ok(cells)
    }

    pub fn recover_cells_and_proofs(
        &self,
        cell_ids: Vec<CellID>,
        cells: Vec<CellRef>,
        _proofs: Vec<Bytes48Ref>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        let cells = self
            .verifier_context
            .recover_all_cells(cell_ids, cells)
            .map_err(|err| ProverError::RecoveryFailure(err))?;

        // Concatenate the cells together
        let extension_blob = cells.into_iter().flatten().collect::<Vec<_>>();

        // We do not need the extension blob, only the blob
        // which is the first BYTES_PER_BLOB bytes.
        let blob = &extension_blob[..BYTES_PER_BLOB];

        self.compute_cells_and_kzg_proofs(blob)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        consensus_specs_fixed_test_vector::{eth_commitment, BLOB_STR, CELLS_STR, PROOFS_STR},
        prover::ProverContext,
    };

    #[test]
    fn test_polynomial_commitment_matches() {
        let ctx = ProverContext::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let got_commitment = ctx.blob_to_kzg_commitment(&blob_bytes).unwrap();
        let expected_commitment = eth_commitment().to_compressed();

        assert_eq!(got_commitment, expected_commitment);
    }

    #[test]
    fn test_computing_proofs() {
        // Setup
        let ctx = ProverContext::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let (got_cells, got_proofs) = ctx.compute_cells_and_kzg_proofs(&blob_bytes).unwrap();

        let expected_proofs = PROOFS_STR;
        let expected_cells = CELLS_STR;

        for k in 0..expected_proofs.len() {
            let expected_proof_str = expected_proofs[k];
            let expected_cell_str = expected_cells[k];

            let got_proof_str = hex::encode(&got_proofs[k]);
            let got_cells_str = hex::encode(&got_cells[k]);

            assert_eq!(got_cells_str, expected_cell_str);
            assert_eq!(got_proof_str, expected_proof_str);
        }
    }
}
