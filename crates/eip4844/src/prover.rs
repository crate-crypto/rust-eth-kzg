use bls12_381::lincomb::g1_lincomb;

use crate::{
    cryptography::{bitreverse_slice, prover::compute_evaluation_and_quotient},
    serialization::{
        deserialize_blob_to_scalars, deserialize_bytes_to_scalar, deserialize_compressed_g1,
        serialize_g1_compressed,
    },
    verifier::compute_fiat_shamir_challenge,
    BlobRef, Context, Error, KZGCommitment, KZGOpeningEvaluation, KZGOpeningPoint, KZGProof,
};

impl Context {
    /// Computes the KZG commitment to the polynomial represented by the blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/deneb/polynomial-commitments.md#blob_to_kzg_commitment
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, Error> {
        // Deserialize the blob into scalars.
        let mut polynomial = deserialize_blob_to_scalars(blob)?;

        bitreverse_slice(&mut polynomial);

        // Compute commitment in lagrange form.
        let commitment = g1_lincomb(&self.prover.commit_key.g1_lagrange, &polynomial)
            .expect("number of g1 points is equal to the number of coefficients in the polynomial")
            .into();

        // Serialize the commitment.
        Ok(serialize_g1_compressed(&commitment))
    }

    /// Compute the KZG proof given a blob and a point.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#compute_kzg_proof
    pub fn compute_kzg_proof(
        &self,
        blob: BlobRef,
        z: KZGOpeningPoint,
    ) -> Result<(KZGProof, KZGOpeningEvaluation), Error> {
        // Deserialize the blob into scalars.
        let polynomial = deserialize_blob_to_scalars(blob)?;

        // Deserialize the point into scalar.
        let z = deserialize_bytes_to_scalar(&z)?;

        // Compute evaluation and quotient at challenge.
        let (y, mut quotient) =
            compute_evaluation_and_quotient(&self.prover.domain, &polynomial, z);
        bitreverse_slice(&mut quotient);

        // Compute KZG opening proof.
        let proof = g1_lincomb(&self.prover.commit_key.g1_lagrange, &quotient)
            .expect("number of g1 points is equal to the number of coefficients in the polynomial")
            .into();

        // Serialize the commitment.
        Ok((serialize_g1_compressed(&proof), y.to_bytes_be()))
    }

    /// Compute the KZG proof given a blob and its corresponding commitment.
    ///
    /// Note: This method does not check that the commitment corresponds to the
    /// blob. The method does still check that the commitment is a valid
    /// commitment.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#compute_kzg_proof
    pub fn compute_blob_kzg_proof(
        &self,
        blob: BlobRef,
        commitment: KZGCommitment,
    ) -> Result<KZGProof, Error> {
        // Deserialize the blob into scalars.
        let polynomial = deserialize_blob_to_scalars(blob)?;

        // Deserialize the KZG commitment.
        // We only do this to check if it is in the correct subgroup
        let _ = deserialize_compressed_g1(&commitment)?;

        // Compute Fiat-Shamir challenge
        let z = compute_fiat_shamir_challenge(blob, commitment);

        // Compute evaluation and quotient at z.
        let (_, mut quotient) =
            compute_evaluation_and_quotient(&self.prover.domain, &polynomial, z);
        bitreverse_slice(&mut quotient);

        // Compute KZG opening proof.
        let proof = g1_lincomb(&self.prover.commit_key.g1_lagrange, &quotient)
            .expect("number of g1 points is equal to the number of coefficients in the polynomial")
            .into();

        // Serialize the commitment.
        Ok(serialize_g1_compressed(&proof))
    }
}
