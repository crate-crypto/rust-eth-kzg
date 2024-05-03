use bls12_381::G1Point;
use kzg_multi_open::{commit_key::{CommitKey, CommitKeyLagrange}, create_eth_commit_opening_keys, fk20::FK20, opening_key::OpeningKey, proof::verify_multi_opening_naive, reverse_bit_order};
use polynomial::domain::Domain;

use crate::{constants::{CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL, FIELD_ELEMENTS_PER_EXT_BLOB}, serialization::{self, deserialize_cell_to_scalars, deserialize_compressed_g1, serialize_g1_compressed}, Blob, Bytes48, Cell, CellID, ColumnIndex, KZGCommitment, KZGProof, RowIndex};

pub struct ProverContext {
    fk20: FK20,
    // TODO: We don't need the commit key, since we use FK20 to compute the proofs
    // TODO: and we use the lagrange variant to compute the commitment to the polynomial.
    //
    // TODO: We can remove it in a later commit, once the API has settled.
    commit_key: CommitKey,
    /// This is only used to save us from doing an IDFT when committing
    /// to the polynomial.
    commit_key_lagrange: CommitKeyLagrange,

    /// Domain used for converting the polynomial to the monomial form.
    poly_domain: Domain,
}

impl ProverContext {
    pub fn new() -> Self {
        let (commit_key, _) = create_eth_commit_opening_keys();
        let point_set_size = FIELD_ELEMENTS_PER_CELL;
        let number_of_points_to_open = FIELD_ELEMENTS_PER_EXT_BLOB;
        let fk20 = FK20::new(&commit_key, point_set_size, number_of_points_to_open);

        let poly_domain = Domain::new(FIELD_ELEMENTS_PER_BLOB);

        // TODO: We can just deserialize these instead of doing this ifft
        let commit_key_lagrange = commit_key.clone().into_lagrange(&poly_domain);

        ProverContext {
            fk20,
            commit_key,
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
