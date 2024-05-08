use crate::{
    constants::{FIELD_ELEMENTS_PER_CELL, FIELD_ELEMENTS_PER_EXT_BLOB},
    serialization::{deserialize_cell_to_scalars, deserialize_compressed_g1},
    Bytes48, Bytes48Ref, Cell, CellID, CellRef, ColumnIndex, RowIndex,
};
use bls12_381::Scalar;
use kzg_multi_open::{
    create_eth_commit_opening_keys, opening_key::OpeningKey, proof::verify_multi_opening_naive,
    reverse_bit_order,
};
use polynomial::domain::Domain;

pub struct VerifierContext {
    opening_key: OpeningKey,
    /// The cosets that we want to verify evaluations against.
    bit_reversed_cosets: Vec<Vec<Scalar>>,
}

impl VerifierContext {
    pub fn new() -> VerifierContext {
        let (_, opening_key) = create_eth_commit_opening_keys();

        let domain_extended = Domain::new(FIELD_ELEMENTS_PER_EXT_BLOB);
        let mut domain_extended_roots = domain_extended.roots;
        reverse_bit_order(&mut domain_extended_roots);

        let cosets: Vec<_> = domain_extended_roots
            .chunks_exact(FIELD_ELEMENTS_PER_CELL)
            .into_iter()
            .map(|coset| coset.to_vec())
            .collect();

        VerifierContext {
            opening_key,
            bit_reversed_cosets: cosets,
        }
    }
    pub fn verify_cell_kzg_proof(
        &self,
        commitment_bytes: Bytes48Ref,
        cell_id: CellID,
        cell: CellRef,
        proof_bytes: Bytes48Ref,
    ) -> bool {
        let commitment = deserialize_compressed_g1(commitment_bytes);
        let proof = deserialize_compressed_g1(proof_bytes);

        let coset = &self.bit_reversed_cosets[cell_id as usize];

        let output_points = deserialize_cell_to_scalars(cell);

        verify_multi_opening_naive(&self.opening_key, commitment, proof, coset, &output_points)
    }

    pub fn verify_cell_kzg_proof_batch(
        &self,
        row_commitments_bytes: Vec<Bytes48>,
        row_indices: Vec<RowIndex>,
        column_indices: Vec<ColumnIndex>,
        cells: Vec<Cell>,
        proofs_bytes: Vec<Bytes48>,
    ) -> bool {
        // TODO: This currently uses the naive method
        //
        // All inputs must have the same length according to the specs.
        assert_eq!(row_commitments_bytes.len(), row_indices.len());
        assert_eq!(row_commitments_bytes.len(), column_indices.len());
        assert_eq!(row_commitments_bytes.len(), cells.len());
        assert_eq!(row_commitments_bytes.len(), proofs_bytes.len());

        for k in 0..row_commitments_bytes.len() {
            let row_index = row_indices[k];
            let row_commitment_bytes = row_commitments_bytes[row_index as usize];
            let column_index = column_indices[k];
            let cell = cells[k].clone();
            let proof_bytes = proofs_bytes[k];

            if !self.verify_cell_kzg_proof(
                &row_commitment_bytes,
                column_index as u64,
                &cell,
                &proof_bytes,
            ) {
                return false;
            }
        }

        true
    }

    pub fn recover_all_cells(&self, cell_ids: Vec<CellID>, cells: Vec<Cell>) -> Vec<Cell> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        consensus_specs_fixed_test_vector::{CELLS_STR, COMMITMENT_STR, PROOFS_STR},
        verifier::VerifierContext,
    };

    #[test]
    fn test_proofs_verify() {
        // Setup
        let ctx = VerifierContext::new();

        let commitment_str = COMMITMENT_STR;
        let commitment_bytes: [u8; 48] = hex::decode(commitment_str).unwrap().try_into().unwrap();

        let proofs_str = PROOFS_STR;
        let proofs_bytes: Vec<[u8; 48]> = proofs_str
            .iter()
            .map(|proof_str| hex::decode(proof_str).unwrap().try_into().unwrap())
            .collect();

        let cells_str = CELLS_STR;
        let cells_bytes: Vec<Vec<u8>> = cells_str
            .into_iter()
            .map(|cell_str| hex::decode(cell_str).unwrap())
            .collect();

        for k in 0..proofs_bytes.len() {
            let proof_bytes = proofs_bytes[k];
            let cell_bytes = cells_bytes[k].clone();
            let cell_id = k as u64;

            assert!(ctx.verify_cell_kzg_proof(
                &commitment_bytes,
                cell_id,
                &cell_bytes,
                &proof_bytes
            ));
        }

        assert!(ctx.verify_cell_kzg_proof_batch(
            vec![commitment_bytes; proofs_bytes.len()],
            vec![0; proofs_bytes.len()],
            (0..proofs_bytes.len()).map(|x| x as u64).collect(),
            cells_bytes,
            proofs_bytes,
        ));
    }
}
