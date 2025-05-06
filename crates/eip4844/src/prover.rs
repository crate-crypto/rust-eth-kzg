use bls12_381::{lincomb::g1_lincomb, G1Point};

use crate::{
    cryptography::{
        bitreverse_slice, compute_fiat_shamir_challenge, prover::compute_evaluation_and_quotient,
    },
    serialization::{
        deserialize_blob_to_scalars, deserialize_bytes_to_scalar, deserialize_compressed_g1,
    },
    BlobRef, Context, Error, KZGCommitment, KZGOpeningEvaluation, KZGOpeningPoint, KZGProof,
};

impl Context {
    /// Computes the KZG commitment to the polynomial represented by the blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/deneb/polynomial-commitments.md#blob_to_kzg_commitment
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, Error> {
        // Deserialize the blob into scalars.
        let mut polynomial = deserialize_blob_to_scalars(blob)?;

        // Bit-reverse polynomial into normal order.
        bitreverse_slice(&mut polynomial);

        // Compute commitment in lagrange form.
        let commitment: G1Point = g1_lincomb(&self.prover.commit_key.g1_lagrange, &polynomial)
            .expect("commit_key.g1_lagrange.len() == polynomial.len()")
            .into();

        // Serialize the commitment.
        Ok(commitment.to_compressed())
    }

    /// Compute the KZG proof given a blob and a point.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#compute_kzg_proof
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
        let (y, quotient) = compute_evaluation_and_quotient(&self.prover.domain, &polynomial, z);

        // Compute KZG opening proof.
        let proof: G1Point = {
            #[cfg(feature = "tracing")]
            let _span = tracing::info_span!("commit quotient").entered();
            g1_lincomb(&self.prover.commit_key.g1_lagrange, &quotient)
                .expect("commit_key.g1_lagrange.len() == quotient.len()")
                .into()
        };

        // Serialize the commitment.
        Ok((proof.to_compressed(), y.to_bytes_be()))
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
        // The quotient is returned in "normal order"
        let (_, quotient) = compute_evaluation_and_quotient(&self.prover.domain, &polynomial, z);

        // Compute KZG opening proof.
        let proof: G1Point = {
            #[cfg(feature = "tracing")]
            let _span = tracing::info_span!("commit quotient").entered();
            g1_lincomb(&self.prover.commit_key.g1_lagrange, &quotient)
                .expect("commit_key.g1_lagrange.len() == quotient.len()")
                .into()
        };

        // Serialize the commitment.
        Ok(proof.to_compressed())
    }
}
