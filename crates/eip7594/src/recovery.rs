use std::collections::HashSet;

use bls12_381::Scalar;
use erasure_codes::{BlockErasureIndices, ReedSolomon};
use kzg_multi_open::recover_evaluations_in_domain_order;

use crate::{
    constants::{CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_EXT_BLOB},
    errors::{Error, RecoveryError},
    serialization::deserialize_cells,
    CellIndex, CellRef,
};

pub(crate) fn recover_polynomial_coeff(
    rs: &ReedSolomon,
    cell_indices: Vec<CellIndex>,
    cells: Vec<CellRef>,
) -> Result<Vec<Scalar>, Error> {
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
        recover_evaluations_in_domain_order(
            FIELD_ELEMENTS_PER_EXT_BLOB,
            cell_indices,
            coset_evaluations,
        )
        // This should never trigger since:
        // - cell_indices is non-empty
        // - all coset evaluations are checked to have the same size
        // - all coset indices are checked to be valid
        .expect("infallible: could not recover evaluations in domain order");

    // Find all of the missing cell indices. This is needed for recovery.
    let missing_cell_indices = find_missing_cell_indices(&cell_indices_normal_order);

    // Recover the polynomial in monomial form, that one can use to generate the cells.
    let recovered_polynomial_coeff = rs
        .recover_polynomial_coefficient(
            flattened_coset_evaluations_normal_order,
            BlockErasureIndices(missing_cell_indices),
        )
        .map_err(RecoveryError::from)?;

    Ok(recovered_polynomial_coeff)
}

fn find_missing_cell_indices(present_cell_indices: &[usize]) -> Vec<usize> {
    let cell_indices: HashSet<_> = present_cell_indices.iter().copied().collect();

    (0..CELLS_PER_EXT_BLOB)
        .filter(|i| !cell_indices.contains(i))
        .collect()
}

mod validation {
    use std::collections::HashSet;

    use crate::{
        constants::{BYTES_PER_CELL, CELLS_PER_EXT_BLOB, EXPANSION_FACTOR},
        errors::RecoveryError,
        CellIndex, CellRef,
    };

    /// Validation logic for `recover_polynomial_coeff`
    pub(crate) fn recover_polynomial_coeff(
        cell_indices: &[CellIndex],
        cells: &[CellRef],
    ) -> Result<(), RecoveryError> {
        // Check that the number of cell indices is equal to the number of cells
        if cell_indices.len() != cells.len() {
            return Err(RecoveryError::NumCellIndicesNotEqualToNumCells {
                num_cell_indices: cell_indices.len(),
                num_cells: cells.len(),
            });
        }

        // Check that the Cell indices are within the expected range
        for cell_index in cell_indices {
            if *cell_index >= (CELLS_PER_EXT_BLOB as u64) {
                return Err(RecoveryError::CellIndexOutOfRange {
                    cell_index: *cell_index,
                    max_number_of_cells: CELLS_PER_EXT_BLOB as u64,
                });
            }
        }

        // Check that each cell has the right amount of bytes
        //
        // This should be infallible.
        for (i, cell) in cells.iter().enumerate() {
            assert_eq!(cell.len(), BYTES_PER_CELL, "the number of bytes in a cell should always equal {BYTES_PER_CELL} since the type is a reference to an array. Check cell at index {i}");
        }

        // Check that we have no duplicate cell indices
        if !are_cell_indices_unique(cell_indices) {
            return Err(RecoveryError::CellIndicesNotUnique);
        }

        // Check that we have enough cells to perform a reconstruction
        if cell_indices.len() < CELLS_PER_EXT_BLOB / EXPANSION_FACTOR {
            return Err(RecoveryError::NotEnoughCellsToReconstruct {
                num_cells_received: cell_indices.len(),
                min_cells_needed: CELLS_PER_EXT_BLOB / EXPANSION_FACTOR,
            });
        }

        // Check that we don't have too many cells
        // ie more than we initially generated from the blob
        //
        // Note: Since we check that there are no duplicates and that all cell_indices
        // are between 0 and CELLS_PER_EXT_BLOB. This check should never fail.
        // It is kept here to be compliant with the specs.
        if cell_indices.len() > CELLS_PER_EXT_BLOB {
            return Err(RecoveryError::TooManyCellsReceived {
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
