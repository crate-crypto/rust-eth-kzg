use std::collections::HashSet;

use crate::{
    constants::{
        BYTES_PER_CELL, CELLS_PER_EXT_BLOB, EXTENSION_FACTOR, FIELD_ELEMENTS_PER_BLOB,
        FIELD_ELEMENTS_PER_CELL, FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{
        deserialize_cell_to_scalars, deserialize_compressed_g1, serialize_scalars_to_cell,
        SerializationError,
    },
    Bytes48Ref, Cell, CellID, CellRef, ColumnIndex, RowIndex,
};
use bls12_381::Scalar;
use erasure_codes::{reed_solomon::Erasures, ReedSolomon};
use kzg_multi_open::{
    create_eth_commit_opening_keys, opening_key::OpeningKey, polynomial::domain::Domain,
    proof::verify_multi_opening_naive, reverse_bit_order,
};

#[derive(Debug)]
pub enum VerifierError {
    Serialization(SerializationError),
    CellIDsNotEqualToNumberOfCells {
        num_cell_ids: usize,
        num_cells: usize,
    },
    CellIDsNotUnique,
    NotEnoughCellsToReconstruct {
        num_cells_received: usize,
        min_cells_needed: usize,
    },
    TooManyCellsHaveBeenGiven {
        num_cells_received: usize,
        max_cells_needed: usize,
    },
    CellDoesNotContainEnoughBytes {
        cell_id: CellID,
        num_bytes: usize,
        expected_num_bytes: usize,
    },
    CellIDOutOfRange {
        cell_id: CellID,
        max_number_of_cells: u64,
    },
    InvalidRowID {
        row_id: u64,
        max_number_of_rows: u64,
    },
    InvalidProof,
    BatchVerificationInputsMustHaveSameLength {
        row_indices_len: usize,
        column_indices_len: usize,
        cells_len: usize,
        proofs_len: usize,
    },
}

pub struct VerifierContext {
    opening_key: OpeningKey,
    /// The cosets that we want to verify evaluations against.
    bit_reversed_cosets: Vec<Vec<Scalar>>,

    rs: ReedSolomon,
}

impl VerifierContext {
    pub fn new() -> VerifierContext {
        let (_, opening_key) = create_eth_commit_opening_keys();

        let domain_extended = Domain::new(FIELD_ELEMENTS_PER_EXT_BLOB);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let cosets: Vec<_> = domain_extended_roots
            .chunks_exact(FIELD_ELEMENTS_PER_CELL)
            .into_iter()
            .map(|coset| coset.to_vec())
            .collect();

        VerifierContext {
            opening_key,
            bit_reversed_cosets: cosets,
            rs: ReedSolomon::new(FIELD_ELEMENTS_PER_BLOB, EXTENSION_FACTOR),
        }
    }
    pub fn verify_cell_kzg_proof(
        &self,
        commitment_bytes: Bytes48Ref,
        cell_id: CellID,
        cell: CellRef,
        proof_bytes: Bytes48Ref,
    ) -> Result<(), VerifierError> {
        sanity_check_cells_and_cell_ids(&[cell_id], &[cell])?;

        let commitment =
            deserialize_compressed_g1(commitment_bytes).map_err(VerifierError::Serialization)?;
        let proof = deserialize_compressed_g1(proof_bytes).map_err(VerifierError::Serialization)?;

        let coset = &self.bit_reversed_cosets[cell_id as usize];

        let output_points =
            deserialize_cell_to_scalars(cell).map_err(VerifierError::Serialization)?;

        let ok =
            verify_multi_opening_naive(&self.opening_key, commitment, proof, coset, &output_points);
        if ok {
            Ok(())
        } else {
            Err(VerifierError::InvalidProof)
        }
    }

    // TODO: take a slice instead of vectors here or something opaque like impl Iterator<Item = &[u8]>
    pub fn verify_cell_kzg_proof_batch<T: AsRef<[u8]> + Clone>(
        &self,
        // This is a deduplicated list of row commitments
        // It is not indicative of the total number of commitments in the batch.
        // This is what row_indices is used for.
        row_commitments_bytes: Vec<T>,
        row_indices: Vec<RowIndex>,
        column_indices: Vec<ColumnIndex>,
        cells: Vec<CellRef>,
        proofs_bytes: Vec<T>,
    ) -> Result<(), VerifierError> {
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
            .map(|row_index| row_commitments_bytes[*row_index as usize].clone())
            .collect();

        for k in 0..row_commitments_bytes.len() {
            let row_index = row_indices[k];
            let row_commitment_bytes = row_commitments_bytes[row_index as usize].as_ref();
            let column_index = column_indices[k];
            let cell = cells[k];
            let proof_bytes = proofs_bytes[k].as_ref();

            if let Err(err) = self.verify_cell_kzg_proof(
                &row_commitment_bytes,
                column_index as u64,
                &cell,
                &proof_bytes,
            ) {
                return Err(err);
            }
        }

        Ok(())
    }

    pub fn recover_all_cells(
        &self,
        cell_ids: Vec<CellID>,
        cells: Vec<CellRef>, // TODO: We could use an AsRef here or use a strongly typed array
    ) -> Result<Vec<Cell>, VerifierError> {
        // TODO: We should check that code does not assume that the CellIDs are sorted.

        sanity_check_cells_and_cell_ids(&cell_ids, &cells)?;

        // Check that we have no duplicate cell IDs
        if !is_cell_ids_unique(&cell_ids) {
            return Err(VerifierError::CellIDsNotUnique);
        }

        // Check that we have enough cells to perform a reconstruction
        if !(CELLS_PER_EXT_BLOB / EXTENSION_FACTOR <= cell_ids.len()) {
            return Err(VerifierError::NotEnoughCellsToReconstruct {
                num_cells_received: cell_ids.len(),
                min_cells_needed: CELLS_PER_EXT_BLOB / EXTENSION_FACTOR,
            });
        }

        // Check that we don't have too many cells
        // ie more than we initially generated
        if cell_ids.len() > CELLS_PER_EXT_BLOB {
            return Err(VerifierError::TooManyCellsHaveBeenGiven {
                num_cells_received: cell_ids.len(),
                max_cells_needed: CELLS_PER_EXT_BLOB,
            });
        }

        pub fn bit_reverse_spec_compliant(n: u32, l: u32) -> u32 {
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
            .map(|cell| deserialize_cell_to_scalars(&cell))
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

            coset_evaluations_flattened_rbo[start..end].copy_from_slice(coset_evals.as_slice());
        }

        let mut coset_evaluations_flattened = coset_evaluations_flattened_rbo;
        reverse_bit_order(&mut coset_evaluations_flattened);

        // We now have the evaluations in normal order and we know the indices/erasures that are missing
        // in normal order.
        let mut recovered_codeword = self.rs.recover_polynomial_codeword(
            coset_evaluations_flattened,
            Erasures::Cells {
                cell_size: FIELD_ELEMENTS_PER_CELL,
                cells: missing_cell_ids,
            },
        );

        // Reverse the order of the recovered points to be in bit-reversed order
        reverse_bit_order(&mut recovered_codeword);

        Ok(recovered_codeword
            .chunks_exact(FIELD_ELEMENTS_PER_CELL)
            .map(|chunk| serialize_scalars_to_cell(chunk))
            .collect())
    }
}

/// Check if all of the cell ids are unique
fn is_cell_ids_unique(cell_ids: &[CellID]) -> bool {
    let len_cell_ids_non_dedup = cell_ids.len();
    let cell_ids_dedup: HashSet<_> = cell_ids.into_iter().collect();
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

    use std::ops::Range;

    use crate::{
        consensus_specs_fixed_test_vector::{CELLS_STR, COMMITMENT_STR, PROOFS_STR},
        constants::CELLS_PER_EXT_BLOB,
        verifier::{is_cell_ids_unique, VerifierContext},
    };

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

            assert!(ctx
                .verify_cell_kzg_proof(&commitment_bytes, cell_id, &cell_bytes, &proof_bytes)
                .is_ok());
        }

        assert!(ctx
            .verify_cell_kzg_proof_batch(
                vec![commitment_bytes; proofs_bytes.len()],
                vec![0; proofs_bytes.len()],
                (0..proofs_bytes.len()).map(|x| x as u64).collect(),
                cells_bytes.iter().map(|cell| cell.as_slice()).collect(),
                proofs_bytes,
            )
            .is_ok());
    }

    #[test]
    fn test_recover_all_cells() {
        let ctx = VerifierContext::new();
        let num_cells_to_keep = CELLS_PER_EXT_BLOB / 2;

        fn generate_unique_random_numbers(range: Range<u64>, n: usize) -> Vec<u64> {
            use rand::prelude::SliceRandom;
            let mut numbers: Vec<_> = range.into_iter().collect();
            numbers.shuffle(&mut rand::thread_rng());
            numbers.into_iter().take(n).collect()
        }

        let cell_ids_to_keep = generate_unique_random_numbers(0..128, num_cells_to_keep);
        let cells_as_hex_strings: Vec<_> = cell_ids_to_keep
            .iter()
            .map(|cell_id| CELLS_STR[*cell_id as usize])
            .collect();
        let cells_to_keep: Vec<_> = cells_as_hex_strings
            .into_iter()
            .map(|cell_str| hex::decode(cell_str).unwrap())
            .collect();

        let all_cells: Vec<_> = CELLS_STR
            .into_iter()
            .map(|cell_str| hex::decode(cell_str).unwrap())
            .collect();

        let recovered_cells = ctx
            .recover_all_cells(
                cell_ids_to_keep,
                cells_to_keep.iter().map(|v| v.as_slice()).collect(),
            )
            .unwrap();

        assert_eq!(recovered_cells, all_cells);
    }
}
