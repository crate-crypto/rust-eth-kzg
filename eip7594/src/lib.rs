use constants::CELLS_PER_EXT_BLOB;

// TODO: We can remove this once we hook up the consensus-specs fixed test vectors.
pub mod consensus_specs_fixed_test_vector;

pub mod constants;
mod serialization;

pub type Blob = Vec<u8>;
pub type Cell = Vec<u8>;
pub type KZGProof = Vec<u8>;
pub type CellID = u64;
pub type RowIndex = u64;
pub type ColumnIndex = u64;
pub type Bytes48 = Vec<u8>;

pub fn compute_cells_and_kzg_proofs(
    blob: Blob,
) -> ([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]) {
    todo!()
}

pub fn compute_cells(blob: Blob) -> [Cell; CELLS_PER_EXT_BLOB] {
    todo!()
}

pub fn verify_cell_kzg_proof(
    commitment_bytes: Bytes48,
    cell_id: CellID,
    cell: Cell,
    proof_bytes: Bytes48,
) -> bool {
    todo!()
}

pub fn verify_cell_kzg_proof_batch(
    row_commitments_bytes: Vec<Bytes48>,
    row_indices: Vec<RowIndex>,
    column_indices: Vec<ColumnIndex>,
    cells: Vec<Cell>,
    proofs_bytes: Vec<Bytes48>,
) -> bool {
    todo!()
}

pub fn recover_all_cells(cell_ids: Vec<CellID>, cells: Vec<Cell>) -> Vec<Cell> {
    todo!()
}

#[cfg(test)]
mod tests {
    use bls12_381::G1Point;
    use kzg_multi_open::{
        create_eth_commit_opening_keys,
        fk20::naive,
        proof::{compute_multi_opening_naive, verify_multi_opening_naive},
        reverse_bit_order,
    };
    use polynomial::domain::Domain;

    use crate::consensus_specs_fixed_test_vector::{
        eth_cells, eth_commitment, eth_polynomial, eth_proofs,
    };

    #[test]
    fn test_polynomial_commitment_matches() {
        // Setup
        let (ck, _) = create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        let domain = Domain::new(POLYNOMIAL_LEN);
        let mut ck_lagrange = ck.into_lagrange(&domain);
        // We need to apply the reverse bit order permutation to the g1s
        // in order for it match the specs.
        // TODO: Apply it to the polynomial instead (time to do it is about 26 microseconds)
        reverse_bit_order(&mut ck_lagrange.g1s);

        let polynomial = eth_polynomial();
        let expected_commitment = eth_commitment();
        let got_commitment = ck_lagrange.commit_g1(&polynomial);

        assert_eq!(got_commitment, expected_commitment);
    }

    #[test]
    fn test_proofs_verify() {
        // Setup
        let (_, vk) = create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

        const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
            .chunks(NUMBER_OF_POINTS_PER_PROOF)
            .collect();

        let commitment: G1Point = eth_commitment().into();
        let proofs = eth_proofs();
        let cells = eth_cells();

        for k in 0..proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            let proof: G1Point = proofs[k].into();
            let coset_eval = &cells[k];

            assert!(verify_multi_opening_naive(
                &vk,
                commitment,
                proof,
                &input_points,
                coset_eval
            ));
        }
    }

    #[test]
    fn test_computing_proofs() {
        // Setup
        let (ck, _) = create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;
        let domain = Domain::new(POLYNOMIAL_LEN);

        const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
            .chunks(NUMBER_OF_POINTS_PER_PROOF)
            .collect();

        let mut polynomial = eth_polynomial();
        // Polynomial really corresponds to the evaluation form, so we need
        // to apply bit reverse order and then IFFT to get the coefficients
        reverse_bit_order(&mut polynomial);
        let poly_coeff = domain.ifft_scalars(polynomial);

        let proofs = eth_proofs();
        let cells = eth_cells();
        for k in 0..proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            let proof: G1Point = proofs[k].clone().into();
            let (quotient_comm, output_points) =
                compute_multi_opening_naive(&ck, &poly_coeff, input_points);

            assert_eq!(cells[k], output_points);
            assert_eq!(proof, quotient_comm);
        }
    }

    // This test becomes redundant once we have consensus-specs fixed test vectors
    // added. Although, it may be beneficial to test consensus-specs fixed test vectors against
    // the naive implementation, if it doesn't add too much overhead.
    #[test]
    fn test_consistency_between_naive_kzg_naive_fk20() {
        // Setup
        let (ck, _) = create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;
        let domain = Domain::new(POLYNOMIAL_LEN);

        const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
            .chunks(NUMBER_OF_POINTS_PER_PROOF)
            .collect();

        const NUMBER_OF_PROOFS: usize = NUMBER_OF_POINTS_TO_EVALUATE / NUMBER_OF_POINTS_PER_PROOF;
        let proof_domain = Domain::new(NUMBER_OF_PROOFS);
        let mut polynomial = eth_polynomial();
        // Polynomial really corresponds to the evaluation form, so we need
        // to apply bit reverse order and then IFFT to get the coefficients
        reverse_bit_order(&mut polynomial);
        let poly_coeff = domain.ifft_scalars(polynomial);

        let (got_proofs, got_set_of_output_points) = naive::fk20_open_multi_point(
            &ck,
            &proof_domain,
            &domain_extended,
            &poly_coeff,
            NUMBER_OF_POINTS_PER_PROOF,
        );

        for k in 0..got_proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            let (expected_quotient_comm, expected_output_points) =
                compute_multi_opening_naive(&ck, &poly_coeff, input_points);
            assert_eq!(expected_output_points, got_set_of_output_points[k]);
            assert_eq!(expected_quotient_comm, got_proofs[k]);
        }
    }
}
