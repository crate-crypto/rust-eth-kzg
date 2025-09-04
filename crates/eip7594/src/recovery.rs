use std::collections::HashSet;

use bls12_381::Scalar;
use erasure_codes::{BlockErasureIndices, ReedSolomon};
use kzg_multi_open::recover_evaluations_in_domain_order;
use serialization::deserialize_cells;

use crate::{
    constants::{
        BYTES_PER_CELL, CELLS_PER_EXT_BLOB, EXPANSION_FACTOR, FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    errors::{Error, RecoveryError},
    CellIndex, CellRef,
};

/// Recovers the original polynomial coefficients from a subset of encoded cells.
///
/// Takes a list of cell indices and their corresponding data, verifies the inputs,
/// reorders the evaluations, and performs Reed-Solomon recovery to reconstruct the polynomial.
///
/// Returns the full set of coefficients if successful, or an error if validation or recovery fails.
pub(crate) fn recover_polynomial_coeff(
    rs: &ReedSolomon,
    cell_indices: Vec<CellIndex>,
    cells: Vec<CellRef>,
) -> Result<Vec<Scalar>, Error> {
    // Validation
    validate_recovery_inputs(&cell_indices, &cells)?;

    // Deserialization
    let coset_evaluations = deserialize_cells(cells)?;
    let cell_indices: Vec<_> = cell_indices
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
    let recovered_polynomial_coeff = rs.recover_polynomial_coefficient(
        flattened_coset_evaluations_normal_order,
        BlockErasureIndices(missing_cell_indices),
    )?;

    Ok(recovered_polynomial_coeff.0)
}

#[inline]
fn find_missing_cell_indices(present_cell_indices: &[usize]) -> Vec<usize> {
    let cell_indices: HashSet<_> = present_cell_indices.iter().copied().collect();

    (0..CELLS_PER_EXT_BLOB)
        .filter(|i| !cell_indices.contains(i))
        .collect()
}

/// Validates that the given cell indices and cell data are suitable for polynomial recovery.
///
/// Checks the following:
/// - Each index has a corresponding cell (`len(indices) == len(cells)`).
/// - All indices are within `[0, CELLS_PER_EXT_BLOB)`.
/// - Each cell has exactly `BYTES_PER_CELL` bytes.
/// - No duplicate indices are present and the cells are in ascending order.
/// - There are enough cells to reconstruct the data (`≥ CELLS_PER_EXT_BLOB / EXPANSION_FACTOR`).
/// - There are not too many cells (`≤ CELLS_PER_EXT_BLOB`).
///
/// Returns `Ok(())` if all checks pass, or a `RecoveryError` if any condition fails.
///
/// Panics if a cell does not have the expected byte length (infallible under correct construction).
pub(crate) fn validate_recovery_inputs(
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
    for &cell_index in cell_indices {
        if cell_index >= (CELLS_PER_EXT_BLOB as u64) {
            return Err(RecoveryError::CellIndexOutOfRange {
                cell_index,
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

    // Check that cells are ordered (ascending)
    if !are_cell_indices_ordered(cell_indices) {
        return Err(RecoveryError::CellIndicesNotUniquelyOrdered);
    }

    // Check that we have enough cells to perform a reconstruction
    if cell_indices.len() < CELLS_PER_EXT_BLOB / EXPANSION_FACTOR {
        return Err(RecoveryError::NotEnoughCellsToReconstruct {
            num_cells_received: cell_indices.len(),
            min_cells_needed: CELLS_PER_EXT_BLOB / EXPANSION_FACTOR,
        });
    }

    // Check that we don't have too many cells,
    // i.e. more than we initially generated from the blob.
    //
    // Note: Since we check that there are no duplicates and that all `cell_indices`
    // are between 0 and `CELLS_PER_EXT_BLOB`, this check should never fail.
    // It is kept here to be compliant with the specs.
    if cell_indices.len() > CELLS_PER_EXT_BLOB {
        return Err(RecoveryError::TooManyCellsReceived {
            num_cells_received: cell_indices.len(),
            max_cells_needed: CELLS_PER_EXT_BLOB,
        });
    }

    Ok(())
}

/// Check if all of the cell indices are sorted in ascending order
fn are_cell_indices_ordered(cell_indices: &[CellIndex]) -> bool {
    cell_indices.is_sorted_by(|a, b| a < b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Leak a zero-initialized cell and return a reference with fixed size.
    fn zeroed_cell_ref() -> CellRef<'static> {
        let boxed: Box<[u8; BYTES_PER_CELL]> = Box::new([0u8; BYTES_PER_CELL]);
        Box::leak(boxed)
    }

    /// Helper: create a valid list of unique indices and cells
    fn make_valid_inputs(min_cells: usize) -> (Vec<CellIndex>, Vec<CellRef<'static>>) {
        let indices: Vec<CellIndex> = (0..min_cells as u64).collect();
        let cells: Vec<CellRef> = (0..min_cells).map(|_| zeroed_cell_ref()).collect();
        (indices, cells)
    }

    #[test]
    fn test_cell_indices_ordered() {
        let cell_indices = vec![1, 2, 3];
        assert!(are_cell_indices_ordered(&cell_indices));

        let cell_indices = vec![3, 2, 1];
        assert!(!are_cell_indices_ordered(&cell_indices));

        let cell_indices = vec![1, 2, 3, 1];
        assert!(!are_cell_indices_ordered(&cell_indices));

        let cell_indices = vec![];
        assert!(are_cell_indices_ordered(&cell_indices));

        let cell_indices = vec![1, 1, 2, 3]; // duplicates should return false
        assert!(!are_cell_indices_ordered(&cell_indices));

        let cell_indices = vec![0, 0, 0];
        assert!(!are_cell_indices_ordered(&cell_indices));
    }

    #[test]
    fn test_validation_success() {
        let min_cells = CELLS_PER_EXT_BLOB / EXPANSION_FACTOR;
        let (indices, cells) = make_valid_inputs(min_cells);
        assert!(validate_recovery_inputs(&indices, &cells).is_ok());
    }

    #[test]
    fn test_mismatched_lengths() {
        let (mut indices, cells) = make_valid_inputs(4);
        indices.pop(); // now indices.len() != cells.len()
        let err = validate_recovery_inputs(&indices, &cells).unwrap_err();
        assert!(matches!(
            err,
            RecoveryError::NumCellIndicesNotEqualToNumCells { .. }
        ));
    }

    #[test]
    fn test_out_of_range_index() {
        let (mut indices, cells) = make_valid_inputs(4);
        indices[1] = CELLS_PER_EXT_BLOB as u64; // out-of-range index
        let err = validate_recovery_inputs(&indices, &cells).unwrap_err();
        assert!(matches!(err, RecoveryError::CellIndexOutOfRange { .. }));
    }

    #[test]
    fn test_not_enough_cells() {
        let too_few = CELLS_PER_EXT_BLOB / EXPANSION_FACTOR - 1;
        let (indices, cells) = make_valid_inputs(too_few);
        let err = validate_recovery_inputs(&indices, &cells).unwrap_err();
        assert!(matches!(
            err,
            RecoveryError::NotEnoughCellsToReconstruct { .. }
        ));
    }

    #[test]
    fn test_duplicate_cell_indices() {
        let indices = vec![0, 1, 2, 2]; // duplicate
        let cells: Vec<CellRef> = (0..indices.len()).map(|_| zeroed_cell_ref()).collect();
        let err = validate_recovery_inputs(&indices, &cells).unwrap_err();
        assert!(matches!(err, RecoveryError::CellIndicesNotUniquelyOrdered));
    }

    #[test]
    fn test_empty_input_should_fail_not_enough() {
        let indices = vec![];
        let cells: Vec<CellRef> = vec![];
        let err = validate_recovery_inputs(&indices, &cells).unwrap_err();
        assert!(matches!(
            err,
            RecoveryError::NotEnoughCellsToReconstruct { .. }
        ));
    }

    #[test]
    fn test_max_cells_is_valid() {
        let (indices, cells) = make_valid_inputs(CELLS_PER_EXT_BLOB);
        assert!(validate_recovery_inputs(&indices, &cells).is_ok());
    }
}
