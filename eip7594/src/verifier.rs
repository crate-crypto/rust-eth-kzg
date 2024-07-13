use std::collections::HashSet;

pub use crate::errors::VerifierError;

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, EXTENSION_FACTOR, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{deserialize_cells, deserialize_compressed_g1_points},
    trusted_setup::TrustedSetup,
    Bytes48Ref, CellIndex, CellRef, PeerDASContext, RowIndex,
};
use bls12_381::Scalar;
use erasure_codes::{BlockErasureIndices, ReedSolomon};
use kzg_multi_open::{
    opening_key::OpeningKey,
    {Prover, Verifier},
};

/// The context object that is used to call functions in the verifier API.
#[derive(Debug)]
pub struct VerifierContext {
    kzg_multipoint_verifier: Verifier,
    rs: ReedSolomon,
}

impl Default for VerifierContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        Self::new(&trusted_setup)
    }
}

impl VerifierContext {
    pub fn new(trusted_setup: &TrustedSetup) -> VerifierContext {
        let opening_key = OpeningKey::from(trusted_setup);

        let multipoint_verifier =
            Verifier::new(opening_key, FIELD_ELEMENTS_PER_EXT_BLOB, CELLS_PER_EXT_BLOB);

        VerifierContext {
            rs: ReedSolomon::new(
                FIELD_ELEMENTS_PER_BLOB,
                EXTENSION_FACTOR,
                CELLS_PER_EXT_BLOB,
            ),
            kzg_multipoint_verifier: multipoint_verifier,
        }
    }
}

fn find_missing_cell_indices(present_cell_indices: &[usize]) -> Vec<usize> {
    let cell_indices: HashSet<_> = present_cell_indices.iter().cloned().collect();

    let mut missing = Vec::new();

    for i in 0..CELLS_PER_EXT_BLOB {
        if !cell_indices.contains(&i) {
            missing.push(i);
        }
    }

    missing
}

impl PeerDASContext {
    /// Verify that a cell is consistent with a commitment using a KZG proof.
    pub fn verify_cell_kzg_proof(
        &self,
        commitment_bytes: Bytes48Ref,
        cell_index: CellIndex,
        cell: CellRef,
        proof_bytes: Bytes48Ref,
    ) -> Result<(), VerifierError> {
        self.thread_pool.install(|| {
            self.verify_cell_kzg_proof_batch(
                vec![commitment_bytes],
                vec![0],
                vec![cell_index],
                vec![cell],
                vec![proof_bytes],
            )
        })
    }

    /// Given a collection of commitments, cells and proofs, this functions verifies that
    /// the cells are consistent with the commitments using their respective KZG proofs.
    pub fn verify_cell_kzg_proof_batch(
        &self,
        // This is a deduplicated list of row commitments
        // It is not indicative of the total number of commitments in the batch.
        // This is what row_indices is used for.
        row_commitments_bytes: Vec<Bytes48Ref>,
        row_indices: Vec<RowIndex>,
        cell_indices: Vec<CellIndex>,
        cells: Vec<CellRef>,
        proofs_bytes: Vec<Bytes48Ref>,
    ) -> Result<(), VerifierError> {
        self.thread_pool.install(|| {
            // Validation
            //
            validation::verify_cell_kzg_proof_batch(
                &row_commitments_bytes,
                &row_indices,
                &cell_indices,
                &cells,
                &proofs_bytes,
            )?;

            // If there are no inputs, we return early with no error
            //
            // Note: We do not check that the commitments are valid in this scenario.
            // It is possible to "misuse" the API, by passing in invalid commitments
            // with no cells, here.
            //
            // TODO: This is only true while we have the `row_indices` API
            // TODO: which will be getting removed soon.
            if cells.is_empty() {
                return Ok(());
            }

            // Deserialization
            //
            let row_commitment_ = deserialize_compressed_g1_points(row_commitments_bytes)?;
            let proofs_ = deserialize_compressed_g1_points(proofs_bytes)?;
            let coset_evals = deserialize_cells(cells)?;

            // Computation
            //
            let ok = self
                .verifier_ctx
                .kzg_multipoint_verifier
                .verify_multi_opening(
                    &row_commitment_,
                    &row_indices,
                    &cell_indices,
                    &coset_evals,
                    &proofs_,
                );

            // Convert the boolean value into a Result
            if ok {
                Ok(())
            } else {
                Err(VerifierError::InvalidProof)
            }
        })
    }

    pub(crate) fn recover_polynomial_coeff(
        &self,
        cell_indices: Vec<CellIndex>,
        cells: Vec<CellRef>,
    ) -> Result<Vec<Scalar>, VerifierError> {
        // Validation
        //
        validation::recover_polynomial_coeff(&cell_indices, &cells)?;

        // Deserialization
        //
        let coset_evaluations = deserialize_cells(cells)?;
        let cell_indices: Vec<usize> = cell_indices
            .into_iter()
            .map(|index| index as usize)
            .collect();

        // Computation
        //
        // Permute the cells, so they are in the order that you would expect, if you were
        // to compute an fft on the monomial form of the polynomial.
        //
        // This comment does leak the fact that the cells are not in the "correct" order,
        // which the API tries to hide.
        let (cell_indices_normal_order, flattened_coset_evaluations_normal_order) =
            Prover::recover_evaluations_in_domain_order(
                FIELD_ELEMENTS_PER_EXT_BLOB,
                cell_indices,
                coset_evaluations,
            )
            .expect("could not recover evaluations in domain order"); // TODO: We could make this an error instead of panic

        // Find all of the missing cell indices. This is needed for recovery.
        let missing_cell_indices = find_missing_cell_indices(&cell_indices_normal_order);

        // Recover the polynomial in monomial form, that one can use to generate the cells.
        let recovered_polynomial_coeff = self.verifier_ctx.rs.recover_polynomial_coefficient(
            flattened_coset_evaluations_normal_order,
            BlockErasureIndices(missing_cell_indices),
        )?;

        Ok(recovered_polynomial_coeff)
    }
}

mod validation {
    use std::collections::HashSet;

    use crate::{
        constants::{BYTES_PER_CELL, CELLS_PER_EXT_BLOB, EXTENSION_FACTOR},
        verifier::VerifierError,
        Bytes48Ref, CellIndex, CellRef, RowIndex,
    };

    /// Validation logic for `verify_cell_kzg_proof_batch`
    pub fn verify_cell_kzg_proof_batch(
        row_commitments_bytes: &[Bytes48Ref],
        row_indices: &[RowIndex],
        cell_indices: &[CellIndex],
        cells: &[CellRef],
        proofs_bytes: &[Bytes48Ref],
    ) -> Result<(), VerifierError> {
        // All inputs must have the same length according to the specs.
        let same_length = (row_indices.len() == cell_indices.len())
            & (row_indices.len() == cells.len())
            & (row_indices.len() == proofs_bytes.len());
        if !same_length {
            return Err(VerifierError::BatchVerificationInputsMustHaveSameLength {
                row_indices_len: row_indices.len(),
                cell_indices_len: cell_indices.len(),
                cells_len: cells.len(),
                proofs_len: proofs_bytes.len(),
            });
        }

        // Check that the row indices are within the correct range
        for row_index in row_indices {
            if *row_index >= row_commitments_bytes.len() as u64 {
                return Err(VerifierError::InvalidRowIndex {
                    row_index: *row_index,
                    max_number_of_rows: row_commitments_bytes.len() as u64,
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

    /// Validation logic for `recover_polynomial_coeff`
    pub(crate) fn recover_polynomial_coeff(
        cell_indices: &[CellIndex],
        cells: &[CellRef],
    ) -> Result<(), VerifierError> {
        // Check that the number of cell indices is equal to the number of cells
        if cell_indices.len() != cells.len() {
            return Err(VerifierError::NumCellIndicesNotEqualToNumCells {
                num_cell_indices: cell_indices.len(),
                num_cells: cells.len(),
            });
        }

        // Check that the Cell indices are within the expected range
        for cell_index in cell_indices.iter() {
            if *cell_index >= (CELLS_PER_EXT_BLOB as u64) {
                return Err(VerifierError::CellIndexOutOfRange {
                    cell_index: *cell_index,
                    max_number_of_cells: CELLS_PER_EXT_BLOB as u64,
                });
            }
        }

        // Check that each cell has the right amount of bytes
        for (i, cell) in cells.iter().enumerate() {
            if cell.len() != BYTES_PER_CELL {
                // TODO: This check should always be true
                return Err(VerifierError::CellDoesNotContainEnoughBytes {
                    cell_index: cell_indices[i],
                    num_bytes: cell.len(),
                    expected_num_bytes: BYTES_PER_CELL,
                });
            }
        }

        // Check that we have no duplicate cell indices
        if !are_cell_indices_unique(cell_indices) {
            return Err(VerifierError::CellIndicesNotUnique);
        }

        // Check that we have enough cells to perform a reconstruction
        if cell_indices.len() < CELLS_PER_EXT_BLOB / EXTENSION_FACTOR {
            return Err(VerifierError::NotEnoughCellsToReconstruct {
                num_cells_received: cell_indices.len(),
                min_cells_needed: CELLS_PER_EXT_BLOB / EXTENSION_FACTOR,
            });
        }

        // Check that we don't have too many cells
        // ie more than we initially generated from the blob
        if cell_indices.len() > CELLS_PER_EXT_BLOB {
            return Err(VerifierError::TooManyCellsReceived {
                num_cells_received: cell_indices.len(),
                max_cells_needed: CELLS_PER_EXT_BLOB,
            });
        }

        Ok(())
    }

    /// Check if all of the cell indices are unique
    fn are_cell_indices_unique(cell_indices: &[CellIndex]) -> bool {
        let len_cell_indices_non_dedup = cell_indices.len();
        let cell_indices_dedup: HashSet<_> = cell_indices.iter().collect();
        cell_indices_dedup.len() == len_cell_indices_non_dedup
    }

    #[cfg(test)]
    mod tests {

        use super::are_cell_indices_unique;

        #[test]
        fn test_cell_indices_unique() {
            let cell_indices = vec![1, 2, 3];
            assert!(are_cell_indices_unique(&cell_indices));
            let cell_indices = vec![];
            assert!(are_cell_indices_unique(&cell_indices));
            let cell_indices = vec![1, 1, 2, 3];
            assert!(!are_cell_indices_unique(&cell_indices));
            let cell_indices = vec![0, 0, 0];
            assert!(!are_cell_indices_unique(&cell_indices));
        }
    }
}
