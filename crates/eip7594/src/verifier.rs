use std::collections::HashMap;

pub use crate::errors::VerifierError;

use crate::{
    constants::{CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_EXT_BLOB},
    errors::Error,
    serialization::{deserialize_cells, deserialize_compressed_g1_points},
    trusted_setup::TrustedSetup,
    Bytes48Ref, CellIndex, CellRef, DASContext,
};
use kzg_multi_open::{verification_key::VerificationKey, Verifier};

/// The context object that is used to call functions in the verifier API.
#[derive(Debug)]
pub struct VerifierContext {
    kzg_multipoint_verifier: Verifier,
}

impl Default for VerifierContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        Self::new(&trusted_setup)
    }
}

impl VerifierContext {
    pub fn new(trusted_setup: &TrustedSetup) -> Self {
        let verification_key = VerificationKey::from(trusted_setup);

        let multipoint_verifier = Verifier::new(
            verification_key,
            FIELD_ELEMENTS_PER_EXT_BLOB,
            CELLS_PER_EXT_BLOB,
        );

        Self {
            kzg_multipoint_verifier: multipoint_verifier,
        }
    }
}

/// Deduplicates a vector and creates a mapping of original indices to deduplicated indices.
///
/// This function takes a vector of items and returns two vectors:
/// 1. A vector of unique items (deduplicated vector)
/// 2. A vector of indices that maps each item in the original vector to its position
///    in the deduplicated vector
fn deduplicate_with_indices<T: Eq + std::hash::Hash + Clone>(input: Vec<T>) -> (Vec<T>, Vec<u64>) {
    let mut unique = Vec::new();
    let mut map = HashMap::new();

    let indices = input
        .into_iter()
        .map(|item| {
            *map.entry(item.clone()).or_insert_with(|| {
                let idx = unique.len();
                unique.push(item);
                idx
            }) as u64
        })
        .collect();

    (unique, indices)
}

impl DASContext {
    /// Given a collection of commitments, cells and proofs, this functions verifies that
    /// the cells are consistent with the commitments using their respective KZG proofs.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#verify_cell_kzg_proof_batch
    pub fn verify_cell_kzg_proof_batch(
        &self,
        commitments: Vec<Bytes48Ref>,
        cell_indices: &[CellIndex],
        cells: Vec<CellRef>,
        proofs_bytes: Vec<Bytes48Ref>,
    ) -> Result<(), Error> {
        let (deduplicated_commitments, row_indices) = deduplicate_with_indices(commitments);

        // Validation
        validation::verify_cell_kzg_proof_batch(
            &deduplicated_commitments,
            &row_indices,
            cell_indices,
            &cells,
            &proofs_bytes,
        )?;

        // If there are no inputs, we return early with no error
        if cells.is_empty() {
            return Ok(());
        }

        // Deserialization
        let row_commitments_ = deserialize_compressed_g1_points(deduplicated_commitments)?;
        let proofs_ = deserialize_compressed_g1_points(proofs_bytes)?;
        let coset_evals = deserialize_cells(cells)?;

        // Computation
        self.verifier_ctx
            .kzg_multipoint_verifier
            .verify_multi_opening(
                &row_commitments_,
                &row_indices,
                cell_indices,
                &coset_evals,
                &proofs_,
            )
            .map_err(VerifierError::from)
            .map_err(Into::into)
    }
}

mod validation {
    use kzg_multi_open::CommitmentIndex;

    use crate::{
        constants::CELLS_PER_EXT_BLOB, verifier::VerifierError, Bytes48Ref, CellIndex, CellRef,
    };

    /// Validation logic for `verify_cell_kzg_proof_batch`
    pub fn verify_cell_kzg_proof_batch(
        deduplicated_commitments_bytes: &[Bytes48Ref],
        commitment_indices: &[CommitmentIndex],
        cell_indices: &[CellIndex],
        cells: &[CellRef],
        proofs_bytes: &[Bytes48Ref],
    ) -> Result<(), VerifierError> {
        // All inputs must have the same length according to the specs.
        let same_length = (commitment_indices.len() == cell_indices.len())
            & (commitment_indices.len() == cells.len())
            & (commitment_indices.len() == proofs_bytes.len());
        if !same_length {
            return Err(VerifierError::BatchVerificationInputsMustHaveSameLength {
                commitment_indices_len: commitment_indices.len(),
                cell_indices_len: cell_indices.len(),
                cells_len: cells.len(),
                proofs_len: proofs_bytes.len(),
            });
        }

        // Check that the commitment indices are within the correct range
        for commitment_index in commitment_indices {
            if *commitment_index >= deduplicated_commitments_bytes.len() as u64 {
                return Err(VerifierError::InvalidCommitmentIndex {
                    commitment_index: *commitment_index,
                    max_number_of_commitments: deduplicated_commitments_bytes.len() as u64,
                });
            }
        }

        // Check that cell indices are in the correct range
        for cell_index in cell_indices {
            if *cell_index >= CELLS_PER_EXT_BLOB as u64 {
                return Err(VerifierError::CellIndexOutOfRange {
                    cell_index: *cell_index,
                    max_number_of_cells: CELLS_PER_EXT_BLOB as u64,
                });
            }
        }

        Ok(())
    }
    #[cfg(test)]
    mod tests {

        #[test]
        fn test_deduplicate_with_indices() {
            let duplicated_vector: Vec<i32> = vec![0, 1, 0, 2, 3, 4, 0];

            let (deduplicated_vec, indices) =
                crate::verifier::deduplicate_with_indices(duplicated_vector);

            let expected_vec = vec![0, 1, 2, 3, 4];
            let expected_indices = vec![0, 1, 0, 2, 3, 4, 0];

            assert_eq!(expected_vec, deduplicated_vec);
            assert_eq!(expected_indices, indices);
        }
    }
}
