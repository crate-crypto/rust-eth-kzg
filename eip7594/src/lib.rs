use bls12_381::G1Point;
use constants::{
    BYTES_PER_COMMITMENT, CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
    FIELD_ELEMENTS_PER_EXT_BLOB,
};
use kzg_multi_open::{
    commit_key::{CommitKey, CommitKeyLagrange},
    create_eth_commit_opening_keys,
    fk20::FK20,
    opening_key::OpeningKey,
    reverse_bit_order,
};
use polynomial::domain::Domain;

use crate::serialization::serialize_g1_compressed;

// TODO: We can remove this once we hook up the consensus-specs fixed test vectors.
pub mod consensus_specs_fixed_test_vector;

pub mod constants;
mod serialization;

pub type Blob = Vec<u8>;
pub type Cell = Vec<u8>;
pub type KZGProof = [u8; BYTES_PER_COMMITMENT];
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];
pub type CellID = u64;
pub type RowIndex = u64;
pub type ColumnIndex = u64;
pub type Bytes48 = Vec<u8>;

// TODO: Split this struct into `ProvingContext` and `VerifyingContext`
// TODO: and rename FK20 into `FK20Prover`
pub struct EIP7594Context {
    fk20: FK20,
    // TODO: We don't need the commit key, since we use FK20 to compute the proofs
    // TODO: and we use the lagrange variant to compute the commitment to the polynomial.
    //
    // TODO: We can remove it in a later commit, once the API has settled.
    commit_key: CommitKey,
    /// This is only used to save us from doing an IDFT when committing
    /// to the polynomial.
    commit_key_lagrange: CommitKeyLagrange,
    opening_key: OpeningKey,

    /// Domain used for converting the polynomial to the monomial form.
    poly_domain: Domain,
}

impl EIP7594Context {
    pub fn new() -> Self {
        let (commit_key, opening_key) = create_eth_commit_opening_keys();
        let point_set_size = FIELD_ELEMENTS_PER_CELL;
        let number_of_points_to_open = FIELD_ELEMENTS_PER_EXT_BLOB;
        let fk20 = FK20::new(&commit_key, point_set_size, number_of_points_to_open);

        let poly_domain = Domain::new(FIELD_ELEMENTS_PER_BLOB);

        // TODO: We can just deserialize these instead of doing this ifft
        let commit_key_lagrange = commit_key.clone().into_lagrange(&poly_domain);

        EIP7594Context {
            fk20,
            commit_key,
            opening_key,
            poly_domain,
            commit_key_lagrange,
        }
    }

    pub fn blob_to_kzg_commitment(&self, blob: Blob) -> KZGCommitment {
        let mut scalars = serialization::deserialize_blob_to_scalars(&blob);
        reverse_bit_order(&mut scalars);

        let commitment: G1Point = self.commit_key_lagrange.commit_g1(&scalars).into();
        serialize_g1_compressed(&commitment)
    }

    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: Blob,
    ) -> ([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]) {
        // Deserialize the blob into scalars (lagrange form)
        let mut scalars = serialization::deserialize_blob_to_scalars(&blob);
        reverse_bit_order(&mut scalars);

        let poly_coeff = self.poly_domain.ifft_scalars(scalars);
        let (proofs, evaluations) = self.fk20.compute_multi_opening_proofs(poly_coeff);

        let cells = evaluations
            .iter()
            .map(|eval| serialization::serialize_scalars_to_cell(eval))
            .collect::<Vec<_>>();
        let cells: [Cell; CELLS_PER_EXT_BLOB] = cells
            .try_into()
            .expect(&format!("expected {} number of cells", CELLS_PER_EXT_BLOB));

        let proofs: Vec<_> = proofs.iter().map(serialize_g1_compressed).collect();
        let proofs: [KZGProof; CELLS_PER_EXT_BLOB] = proofs
            .try_into()
            .expect(&format!("expected {} number of proofs", CELLS_PER_EXT_BLOB));

        (cells, proofs)
    }

    pub fn compute_cells(&self, blob: Blob) -> [Cell; CELLS_PER_EXT_BLOB] {
        // Deserialize the blob into scalars (lagrange form)
        let mut scalars = serialization::deserialize_blob_to_scalars(&blob);
        reverse_bit_order(&mut scalars);

        let poly_coeff = self.poly_domain.ifft_scalars(scalars);
        let evaluations = self.fk20.compute_evaluation_sets(poly_coeff);

        let cells = evaluations
            .iter()
            .map(|eval| serialization::serialize_scalars_to_cell(eval))
            .collect::<Vec<_>>();
        let cells: [Cell; CELLS_PER_EXT_BLOB] = cells
            .try_into()
            .expect(&format!("expected {} number of cells", CELLS_PER_EXT_BLOB));

        cells
    }
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

    use crate::{
        consensus_specs_fixed_test_vector::{
            eth_cells, eth_commitment, eth_polynomial, eth_proofs, BLOB_STR, CELLS_STR, PROOFS_STR,
        },
        EIP7594Context,
    };

    #[test]
    fn test_polynomial_commitment_matches() {
        let ctx = EIP7594Context::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let got_commitment = ctx.blob_to_kzg_commitment(blob_bytes);
        let expected_commitment = eth_commitment().to_compressed();

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
        let ctx = EIP7594Context::new();

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
