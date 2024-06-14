use bls12_381::{G1Point, Scalar};
use kzg_multi_open::{
    commit_key::{CommitKey, CommitKeyLagrange},
    create_eth_commit_opening_keys,
    fk20::FK20,
    polynomial::domain::Domain,
    reverse_bit_order,
};

use crate::{
    constants::{
        CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
        FIELD_ELEMENTS_PER_EXT_BLOB,
    },
    serialization::{self, serialize_g1_compressed, SerializationError},
    verifier::{VerifierContext, VerifierError},
    BlobRefFixed, Bytes48RefFixed, Cell, CellID, CellRefFixed, KZGCommitment, KZGProof,
};

/// Errors that can occur while calling a method in the Prover API
#[derive(Debug)]
pub enum ProverError {
    Serialization(SerializationError),
    RecoveryFailure(VerifierError),
}

/// Context object that is used to call functions in the prover API.
/// This includes, computing the commitments, proofs and cells.
pub struct ProverContext {
    fk20: FK20,
    // TODO: We don't need the commit key, since we use FK20 to compute the proofs
    // TODO: and we use the lagrange variant to compute the commitment to the polynomial.
    //
    // TODO: We can remove it in a later commit, once the API has settled.
    #[allow(dead_code)]
    commit_key: CommitKey,
    /// This is only used to save us from doing an IDFT when committing
    /// to the polynomial.
    commit_key_lagrange: CommitKeyLagrange,

    /// Domain used for converting the polynomial to the monomial form.
    poly_domain: Domain,
    // Verifier context
    //
    // The prover needs the verifier context to recover the cells and then compute the proofs
    verifier_context: VerifierContext,
}

impl Default for ProverContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ProverContext {
    pub fn new() -> Self {
        let (commit_key, _) = create_eth_commit_opening_keys();
        // The number of points that we will make an opening proof for,
        // ie a proof will attest to the value of the polynomial at these points.
        let point_set_size = FIELD_ELEMENTS_PER_CELL;

        // The number of points that we will be making proofs for.
        //
        // Note: it is easy to calculate the number of proofs that we need to make
        // by doing number_of_points_to_open / point_set_size.
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
            verifier_context: VerifierContext::new(),
        }
    }

    /// Computes the KZG commitment to the polynomial represented by the blob.
    pub fn blob_to_kzg_commitment(&self, blob: BlobRefFixed) -> Result<KZGCommitment, ProverError> {
        // Deserialize the blob into scalars. The blob is in lagrange form.
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;

        // Reverse the order of the scalars, so that they are in normal order.
        // ie not in bit-reversed order.
        reverse_bit_order(&mut scalars);

        // Commit to the polynomial.
        let commitment: G1Point = self.commit_key_lagrange.commit_g1(&scalars).into();

        // Serialize the commitment.
        Ok(serialize_g1_compressed(&commitment))
    }

    /// Computes the cells and the KZG proofs for the given blob.
    pub fn compute_cells_and_kzg_proofs(
        &self,
        blob: BlobRefFixed,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        // Deserialize the blob into scalars. The blob is in lagrange form.
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;

        // Reverse the order of the scalars, so that they are in normal order.
        // ie not in bit-reversed order.
        reverse_bit_order(&mut scalars);

        // Convert the polynomial from lagrange to monomial form.
        let poly_coeff = self.poly_domain.ifft_scalars(scalars);

        // Compute the proofs and the evaluations of the polynomial.
        let (proofs, evaluations) = self.fk20.compute_multi_opening_proofs(poly_coeff);

        // Serialize the evaluations into `Cell`s.
        let cells = evaluations_to_cells(evaluations.into_iter());

        // Serialize the proofs into `KZGProof`s.
        let proofs: Vec<_> = proofs.iter().map(serialize_g1_compressed).collect();
        let proofs: [KZGProof; CELLS_PER_EXT_BLOB] = proofs
            .try_into()
            .unwrap_or_else(|_| panic!("expected {} number of proofs", CELLS_PER_EXT_BLOB));

        Ok((cells, proofs))
    }

    #[deprecated(note = "This function is deprecated, use `compute_cells_and_kzg_proofs` instead")]
    pub fn compute_cells(
        &self,
        blob: BlobRefFixed,
    ) -> Result<[Cell; CELLS_PER_EXT_BLOB], ProverError> {
        // Deserialize the blob into scalars. The blob is in lagrange form.
        let mut scalars =
            serialization::deserialize_blob_to_scalars(blob).map_err(ProverError::Serialization)?;

        // Reverse the order of the scalars, so that they are in normal order.
        // ie not in bit-reversed order.
        reverse_bit_order(&mut scalars);

        // Convert the polynomial from lagrange to monomial form.
        let poly_coeff = self.poly_domain.ifft_scalars(scalars);

        // Compute the evaluations of the polynomial at the points that we need to make proofs for.
        let evaluations = self.fk20.compute_evaluation_sets(poly_coeff);

        // Serialize the evaluations into cells.
        let cells = evaluations_to_cells(evaluations.into_iter());

        Ok(cells)
    }

    /// Recovers the cells and computes the KZG proofs, given a subset of cells.
    #[allow(deprecated)]
    pub fn recover_cells_and_proofs(
        &self,
        cell_ids: Vec<CellID>,
        cells: Vec<CellRefFixed>,
        _proofs: Vec<Bytes48RefFixed>,
    ) -> Result<([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]), ProverError> {
        // Use erasure decoding to recover the codeword.
        // TODO: Make this return the polynomial coeff and then just copy-paste the rest of
        // TODO the code from compute_cells_and_kzg_proofs
        let recovered_codeword = self
            .verifier_context
            .recover_polynomial(cell_ids, cells)
            .map_err(ProverError::RecoveryFailure)?;

        // The first FIELD_ELEMENTS_PER_BLOB elements correspond to the polynomial
        // represented by the blob.
        let blob_polynomial = &recovered_codeword[..FIELD_ELEMENTS_PER_BLOB];

        // To compute the proofs, we need the Blob.
        // The blob will be the first BYTES_PER_BLOB bytes from the extension blob.
        let blob: Vec<_> = blob_polynomial
            .into_iter()
            .map(Scalar::to_bytes_be)
            .flatten()
            .collect();

        // Compute the cells and the proofs for the given blob.
        self.compute_cells_and_kzg_proofs(&blob.try_into().unwrap())
    }
}

pub(crate) fn evaluations_to_cells<T: AsRef<[Scalar]>>(
    evaluations: impl Iterator<Item = T>,
) -> [Cell; CELLS_PER_EXT_BLOB] {
    let cells: Vec<Cell> = evaluations
        .map(|eval| serialization::serialize_scalars_to_cell(eval.as_ref()))
        .map(|cell| {
            cell.into_boxed_slice()
                .try_into()
                .expect("infallible: Vec<u8> should have length equal to BYTES_PER_CELL")
        })
        .collect();

    cells
        .try_into()
        .unwrap_or_else(|_| panic!("expected {} number of cells", CELLS_PER_EXT_BLOB))
}

#[cfg(test)]
mod tests {
    use crate::{
        consensus_specs_fixed_test_vector::{eth_commitment, BLOB_STR, CELLS_STR, PROOFS_STR},
        prover::ProverContext,
    };

    #[test]
    fn test_polynomial_commitment_matches() {
        let ctx = ProverContext::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let got_commitment = ctx
            .blob_to_kzg_commitment(&blob_bytes.try_into().unwrap())
            .unwrap();
        let expected_commitment = eth_commitment().to_compressed();

        assert_eq!(got_commitment, expected_commitment);
    }

    #[test]
    fn test_computing_proofs() {
        // Setup
        let ctx = ProverContext::new();

        let blob_bytes = hex::decode(BLOB_STR).unwrap();

        let (got_cells, got_proofs) = ctx
            .compute_cells_and_kzg_proofs(&blob_bytes.try_into().unwrap())
            .unwrap();

        let expected_proofs = PROOFS_STR;
        let expected_cells = CELLS_STR;

        for k in 0..expected_proofs.len() {
            let expected_proof_str = expected_proofs[k];
            let expected_cell_str = expected_cells[k];

            let got_proof_str = hex::encode(&got_proofs[k]);
            let got_cells_str = hex::encode(&*got_cells[k]);

            assert_eq!(got_cells_str, expected_cell_str);
            assert_eq!(got_proof_str, expected_proof_str);
        }
    }
}
