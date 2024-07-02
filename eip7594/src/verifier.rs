use std::{collections::HashSet, sync::Arc};

pub use crate::errors::VerifierError;

use crate::{
    constants::{
        BYTES_PER_CELL, CELLS_PER_EXT_BLOB, EXTENSION_FACTOR, FIELD_ELEMENTS_PER_BLOB,
        FIELD_ELEMENTS_PER_CELL, FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    prover::evaluation_sets_to_cells,
    serialization::{deserialize_cell_to_scalars, deserialize_compressed_g1},
    trusted_setup::TrustedSetup,
    Bytes48Ref, Cell, CellID, CellRef, ColumnIndex, RowIndex,
};
use bls12_381::Scalar;
use erasure_codes::{reed_solomon::Erasures, ReedSolomon};
use kzg_multi_open::{
    opening_key::OpeningKey, polynomial::domain::Domain, proof::verify_multi_opening_naive,
    reverse_bit_order,
};
use rayon::ThreadPool;

/// The context object that is used to call functions in the verifier API.
#[derive(Debug)]
pub struct VerifierContext {
    thread_pool: Arc<ThreadPool>,
    opening_key: OpeningKey,
    /// The cosets that we want to verify evaluations against.
    bit_reversed_cosets: Vec<Vec<Scalar>>,

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
        let domain_extended = Domain::new(FIELD_ELEMENTS_PER_EXT_BLOB);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let cosets: Vec<_> = domain_extended_roots
            .chunks_exact(FIELD_ELEMENTS_PER_CELL)
            .map(|coset| coset.to_vec())
            .collect();

        VerifierContext {
            thread_pool,
            opening_key,
            bit_reversed_cosets: cosets,
            rs: ReedSolomon::new(FIELD_ELEMENTS_PER_BLOB, EXTENSION_FACTOR),
        }
    }

    /// Verify that a cell is consistent with a commitment using a KZG proof.
    pub fn verify_cell_kzg_proof(
        &self,
        commitment_bytes: Bytes48Ref,
        cell_id: CellID,
        cell: CellRef,
        proof_bytes: Bytes48Ref,
    ) -> Result<(), VerifierError> {
        self.thread_pool.install(|| {
            sanity_check_cells_and_cell_ids(&[cell_id], &[cell])?;

            let commitment = deserialize_compressed_g1(commitment_bytes)
                .map_err(VerifierError::Serialization)?;
            let proof =
                deserialize_compressed_g1(proof_bytes).map_err(VerifierError::Serialization)?;

            let coset = &self.bit_reversed_cosets[cell_id as usize];

            let output_points =
                deserialize_cell_to_scalars(cell).map_err(VerifierError::Serialization)?;

            let ok = verify_multi_opening_naive(
                &self.opening_key,
                commitment,
                proof,
                coset,
                &output_points,
            );
            if ok {
                Ok(())
            } else {
                Err(VerifierError::InvalidProof)
            }
        })
    }

    /// This is the batch version of `verify_cell_kzg_proof`.
    ///
    /// Given a collection of commitments, cells and proofs, this functions verifies that
    /// the cells are consistent with the commitments using the KZG proofs.
    pub fn verify_cell_kzg_proof_batch(
        &self,
        // This is a deduplicated list of row commitments
        // It is not indicative of the total number of commitments in the batch.
        // This is what row_indices is used for.
        row_commitments_bytes: Vec<Bytes48Ref>,
        row_indices: Vec<RowIndex>,
        column_indices: Vec<ColumnIndex>,
        cells: Vec<CellRef>,
        proofs_bytes: Vec<Bytes48Ref>,
    ) -> Result<(), VerifierError> {
        self.thread_pool.install(|| {
            // TODO: This currently uses the naive method
            //
            // All inputs must have the same length according to the specs.
            let same_length = (row_indices.len() == column_indices.len())
                & (row_indices.len() == cells.len())
                & (row_indices.len() == proofs_bytes.len());
            if !same_length {
                return Err(VerifierError::BatchVerificationInputsMustHaveSameLength {
                    row_indices_len: row_indices.len(),
                    column_indices_len: column_indices.len(),
                    cells_len: cells.len(),
                    proofs_len: proofs_bytes.len(),
                });
            }

            // If there are no inputs, we return early with no error
            if cells.is_empty() {
                return Ok(());
            }

            // Check that the row indices are within the correct range
            for row_index in &row_indices {
                if *row_index >= row_commitments_bytes.len() as u64 {
                    return Err(VerifierError::InvalidRowID {
                        row_id: *row_index,
                        max_number_of_rows: row_commitments_bytes.len() as u64,
                    });
                }
            }

            let row_commitments_bytes: Vec<_> = row_indices
                .iter()
                .map(|row_index| row_commitments_bytes[*row_index as usize])
                .collect();

            for (k, row_commitment) in row_commitments_bytes.into_iter().enumerate() {
                let column_index = column_indices[k];
                let cell = cells[k];
                let proof_bytes = proofs_bytes[k];

                // Verify and return early if the proof is invalid
                self.verify_cell_kzg_proof(row_commitment, column_index, cell, proof_bytes)?;
            }

            Ok(())
        })
    }

    pub(crate) fn recover_polynomial_coeff(
        &self,
        cell_ids: Vec<CellID>,
        cells: Vec<CellRef>,
    ) -> Result<Vec<Scalar>, VerifierError> {
        // TODO: We should check that code does not assume that the CellIDs are sorted.

        sanity_check_cells_and_cell_ids(&cell_ids, &cells)?;

        // Check that we have no duplicate cell IDs
        if !is_cell_ids_unique(&cell_ids) {
            return Err(VerifierError::CellIDsNotUnique);
        }

        // Check that we have enough cells to perform a reconstruction
        if cell_ids.len() < CELLS_PER_EXT_BLOB / EXTENSION_FACTOR {
            return Err(VerifierError::NotEnoughCellsToReconstruct {
                num_cells_received: cell_ids.len(),
                min_cells_needed: CELLS_PER_EXT_BLOB / EXTENSION_FACTOR,
            });
        }

        // Check that we don't have too many cells
        // ie more than we initially generated from the blob
        if cell_ids.len() > CELLS_PER_EXT_BLOB {
            return Err(VerifierError::TooManyCellsHaveBeenGiven {
                num_cells_received: cell_ids.len(),
                max_cells_needed: CELLS_PER_EXT_BLOB,
            });
        }

        fn bit_reverse_spec_compliant(n: u32, l: u32) -> u32 {
            let num_bits = l.trailing_zeros();
            n.reverse_bits() >> (32 - num_bits)
        }

        // Find out what cells are missing and bit reverse their index
        // so we can figure out what cells are missing in the "normal order"
        let cell_ids_received: HashSet<_> = cell_ids.iter().collect();
        let mut missing_cell_ids = Vec::new();
        for i in 0..CELLS_PER_EXT_BLOB {
            if !cell_ids_received.contains(&(i as u64)) {
                missing_cell_ids
                    .push(bit_reverse_spec_compliant(i as u32, CELLS_PER_EXT_BLOB as u32) as usize);
            }
        }

        let coset_evaluations: Result<Vec<_>, _> = cells
            .into_iter()
            .map(AsRef::as_ref)
            .map(deserialize_cell_to_scalars)
            .collect();
        let coset_evaluations = coset_evaluations.map_err(VerifierError::Serialization)?;

        // Fill in the missing coset_evaluation_sets in bit-reversed order
        // and flatten the evaluations
        //
        let mut coset_evaluations_flattened_rbo =
            vec![Scalar::from(0); FIELD_ELEMENTS_PER_EXT_BLOB];

        for (cell_id, coset_evals) in cell_ids.into_iter().zip(coset_evaluations) {
            let start = (cell_id as usize) * FIELD_ELEMENTS_PER_CELL;
            let end = start + FIELD_ELEMENTS_PER_CELL;

            coset_evaluations_flattened_rbo[start..end].copy_from_slice(&coset_evals);
        }

        let mut coset_evaluations_flattened = coset_evaluations_flattened_rbo;
        reverse_bit_order(&mut coset_evaluations_flattened);

        // We now have the evaluations in normal order and we know the indices/erasures that are missing
        // in normal order.
        let recovered_polynomial_coeff = self.rs.recover_polynomial_coefficient(
            coset_evaluations_flattened,
            Erasures::Cells {
                cell_size: FIELD_ELEMENTS_PER_CELL,
                cells: missing_cell_ids,
            },
        );
        
        // TODO: We could move this code into the ReedSolomon crate
        // We extended our original data by EXTENSION_FACTOR
        // The recovered polynomial in monomial and lagrange form
        // should have the same length as the original data.
        // All of the coefficients after the original data should be zero.
        for i in FIELD_ELEMENTS_PER_BLOB..FIELD_ELEMENTS_PER_EXT_BLOB {
            if recovered_polynomial_coeff[i] != Scalar::from(0u64) {
                return Err(VerifierError::PolynomialHasInvalidLength {
                    num_coefficients: i,
                    expected_num_coefficients: FIELD_ELEMENTS_PER_BLOB,
                });
            }
        }

        Ok(recovered_polynomial_coeff[0..FIELD_ELEMENTS_PER_BLOB].to_vec())
    }
}

/// Check if all of the cell ids are unique
fn is_cell_ids_unique(cell_ids: &[CellID]) -> bool {
    let len_cell_ids_non_dedup = cell_ids.len();
    let cell_ids_dedup: HashSet<_> = cell_ids.iter().collect();
    cell_ids_dedup.len() == len_cell_ids_non_dedup
}

fn sanity_check_cells_and_cell_ids(
    cell_ids: &[CellID],
    cells: &[CellRef],
) -> Result<(), VerifierError> {
    // Check that the number of cell IDs is equal to the number of cells
    if cell_ids.len() != cells.len() {
        return Err(VerifierError::CellIDsNotEqualToNumberOfCells {
            num_cell_ids: cell_ids.len(),
            num_cells: cells.len(),
        });
    }

    // Check that the Cell IDs are within the expected range
    for cell_id in cell_ids.iter() {
        if *cell_id >= (CELLS_PER_EXT_BLOB as u64) {
            return Err(VerifierError::CellIDOutOfRange {
                cell_id: *cell_id,
                max_number_of_cells: CELLS_PER_EXT_BLOB as u64,
            });
        }
    }

    // Check that each cell has the right amount of bytes
    for (i, cell) in cells.iter().enumerate() {
        if cell.len() != BYTES_PER_CELL {
            // TODO: This check should always be true
            return Err(VerifierError::CellDoesNotContainEnoughBytes {
                cell_id: cell_ids[i],
                num_bytes: cell.len(),
                expected_num_bytes: BYTES_PER_CELL,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::verifier::is_cell_ids_unique;

    #[test]
    fn test_cell_ids_unique() {
        let cell_ids = vec![1, 2, 3];
        assert!(is_cell_ids_unique(&cell_ids));
        let cell_ids = vec![];
        assert!(is_cell_ids_unique(&cell_ids));
        let cell_ids = vec![1, 1, 2, 3];
        assert!(!is_cell_ids_unique(&cell_ids));
        let cell_ids = vec![0, 0, 0];
        assert!(!is_cell_ids_unique(&cell_ids));
    }
}
