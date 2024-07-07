use std::{collections::HashSet, sync::Arc};

pub use crate::errors::VerifierError;

use crate::{
    constants::{
        BYTES_PER_CELL, CELLS_PER_EXT_BLOB, EXTENSION_FACTOR, FIELD_ELEMENTS_PER_BLOB,
        FIELD_ELEMENTS_PER_CELL, FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{deserialize_cells, deserialize_compressed_g1_points},
    trusted_setup::TrustedSetup,
    Bytes48Ref, CellIndex, CellRef, RowIndex,
};
use bls12_381::Scalar;
use erasure_codes::{reed_solomon::Erasures, ReedSolomon};
use kzg_multi_open::{
    fk20::{self, verify::verify_multi_opening, FK20},
    opening_key::OpeningKey,
};
use rayon::ThreadPool;

/// The context object that is used to call functions in the verifier API.
#[derive(Debug)]
pub struct VerifierContext {
    thread_pool: Arc<ThreadPool>,
    opening_key: OpeningKey,
    // TODO: This can be moved into FK20 verification procedure
    coset_shifts: Vec<Scalar>,
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
        const DEFAULT_NUM_THREADS: usize = 16;
        Self::with_num_threads(trusted_setup, DEFAULT_NUM_THREADS)
    }

    pub fn with_num_threads(trusted_setup: &TrustedSetup, num_threads: usize) -> VerifierContext {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();
        Self::from_thread_pool(trusted_setup, Arc::new(thread_pool))
    }

    pub(crate) fn from_thread_pool(
        trusted_setup: &TrustedSetup,
        thread_pool: Arc<ThreadPool>,
    ) -> VerifierContext {
        let opening_key = OpeningKey::from(trusted_setup);
        let coset_shifts = fk20::coset_gens(FIELD_ELEMENTS_PER_EXT_BLOB, CELLS_PER_EXT_BLOB, true);

        VerifierContext {
            thread_pool,
            opening_key,
            rs: ReedSolomon::new(FIELD_ELEMENTS_PER_BLOB, EXTENSION_FACTOR),
            coset_shifts,
        }
    }

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

            // If there are no inputs, we return early with no error
            //
            // Note: We do not check that the commitments are valid in this scenario.
            // It is possible to "misuse" the API, by passing in invalid commitments
            // with no cells, here.
            if cells.is_empty() {
                return Ok(());
            }

            // Check that the row indices are within the correct range
            for row_index in &row_indices {
                if *row_index >= row_commitments_bytes.len() as u64 {
                    return Err(VerifierError::InvalidRowIndex {
                        row_index: *row_index,
                        max_number_of_rows: row_commitments_bytes.len() as u64,
                    });
                }
            }

            // Check that column indices are in the correct range
            for column_index in &cell_indices {
                if *column_index >= CELLS_PER_EXT_BLOB as u64 {
                    return Err(VerifierError::CellIndexOutOfRange {
                        cell_index: *column_index,
                        max_number_of_cells: CELLS_PER_EXT_BLOB as u64,
                    });
                }
            }

            // Deserialization
            //
            let row_commitment_ = deserialize_compressed_g1_points(row_commitments_bytes)?;
            let proofs_ = deserialize_compressed_g1_points(proofs_bytes)?;
            let coset_evals = deserialize_cells(cells)?;

            let ok = verify_multi_opening(
                &self.opening_key,
                &row_commitment_,
                &row_indices,
                &cell_indices,
                &self.coset_shifts,
                &coset_evals,
                &proofs_,
            );

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
        if !are_cell_indices_unique(&cell_indices) {
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

        // Deserialization
        //
        let coset_evaluations = deserialize_cells(cells)?;
        let cell_indices: Vec<usize> = cell_indices
            .into_iter()
            .map(|index| index as usize)
            .collect();

        let (cell_indices_normal_order, flattened_coset_evaluations_normal_order) =
            FK20::recover_evaluations_in_domain_order(
                FIELD_ELEMENTS_PER_EXT_BLOB,
                cell_indices,
                coset_evaluations,
            )
            .expect("could not recover evaluations in domain order"); // TODO: We could make this an error instead of panic

        let missing_cell_indices = find_missing_cell_indices(&cell_indices_normal_order);

        let recovered_polynomial_coeff = self.rs.recover_polynomial_coefficient(
            flattened_coset_evaluations_normal_order,
            Erasures::Cells {
                cell_size: FIELD_ELEMENTS_PER_CELL,
                cells: missing_cell_indices,
            },
        )?;

        Ok(recovered_polynomial_coeff)
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

/// Check if all of the cell indices are unique
fn are_cell_indices_unique(cell_indices: &[CellIndex]) -> bool {
    let len_cell_indices_non_dedup = cell_indices.len();
    let cell_indices_dedup: HashSet<_> = cell_indices.iter().collect();
    cell_indices_dedup.len() == len_cell_indices_non_dedup
}

#[cfg(test)]
mod tests {

    use crate::verifier::are_cell_indices_unique;

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
