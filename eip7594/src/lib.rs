use constants::BYTES_PER_COMMITMENT;

// TODO: We can remove this once we hook up the consensus-specs fixed test vectors.
pub mod consensus_specs_fixed_test_vector;

pub mod constants;
mod serialization;
pub mod prover;
pub mod verifier;

pub type Blob = Vec<u8>;
pub type Cell = Vec<u8>;
pub type KZGProof = [u8; BYTES_PER_COMMITMENT];
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];
pub type CellID = u64;
pub type RowIndex = u64;
pub type ColumnIndex = u64;
pub type Bytes48 = [u8; 48];

#[cfg(test)]
mod tests {
    use kzg_multi_open::{
        create_eth_commit_opening_keys,
        fk20::naive,
        proof::compute_multi_opening_naive,
        reverse_bit_order,
    };
    use polynomial::domain::Domain;

    use crate::{
        consensus_specs_fixed_test_vector::{
            eth_commitment, eth_polynomial,  BLOB_STR, CELLS_STR,
            COMMITMENT_STR, PROOFS_STR,
        }, prover::ProverContext, verifier::VerifierContext,
    };

    #[test]
    fn test_polynomial_commitment_matches() {
        let ctx = ProverContext::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let got_commitment = ctx.blob_to_kzg_commitment(blob_bytes);
        let expected_commitment = eth_commitment().to_compressed();

        assert_eq!(got_commitment, expected_commitment);
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

            assert!(ctx.verify_cell_kzg_proof(commitment_bytes, cell_id, cell_bytes, proof_bytes));
        }

        assert!(ctx.verify_cell_kzg_proof_batch(
            vec![commitment_bytes; proofs_bytes.len()],
            vec![0; proofs_bytes.len()],
            (0..proofs_bytes.len()).map(|x| x as u64).collect(),
            cells_bytes,
            proofs_bytes,
        ));
    }

    #[test]
    fn test_computing_proofs() {
        // Setup
        let ctx = ProverContext::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let (got_cells, got_proofs) = ctx.compute_cells_and_kzg_proofs(blob_bytes);

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
