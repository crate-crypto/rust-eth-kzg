use bls12_381::{
    group::{Curve, Group},
    lincomb::g1_lincomb,
    G1Projective,
};

use crate::{
    kzg_open::divide_by_linear,
    serialization::{
        deserialize_blob_to_scalars, deserialize_bytes_to_scalar, deserialize_compressed_g1,
        serialize_g1_compressed,
    },
    verifier::{blob_scalar_to_polynomial, compute_fiat_shamir_challenge},
    BlobRef, Context, Error, KZGCommitment, KZGOpeningEvaluation, KZGOpeningPoint, KZGProof,
};

impl Context {
    /// Computes the KZG commitment to the polynomial represented by the blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/deneb/polynomial-commitments.md#blob_to_kzg_commitment
    pub fn blob_to_kzg_commitment(&self, blob: BlobRef) -> Result<KZGCommitment, Error> {
        // Deserialize the blob into scalars.
        let blob_scalar = deserialize_blob_to_scalars(blob)?;

        // Convert blob into monomial form.
        let polynomial = blob_scalar_to_polynomial(&self.prover.domain, &blob_scalar);

        // Compute commitment in monomial form.
        let commitment = g1_lincomb(&self.prover.commit_key.g1s, &polynomial)
            .expect("commit_key.g1s.len() == polynomial.len()")
            .to_affine();

        // Serialize the commitment.
        Ok(serialize_g1_compressed(&commitment))
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
        let blob_scalar = deserialize_blob_to_scalars(blob)?;

        // Convert blob into monomial form.
        let polynomial = blob_scalar_to_polynomial(&self.prover.domain, &blob_scalar);

        // Deserialize the point into scalar.
        let z = deserialize_bytes_to_scalar(&z)?;

        // Compute evaluation and quotient at challenge.
        let (quotient, y) = divide_by_linear(&polynomial, z);

        // Compute KZG opening proof.
        let proof = {
            #[cfg(feature = "tracing")]
            let _span = tracing::info_span!("commit quotient").entered();
            g1_lincomb(&self.prover.commit_key.g1s[..quotient.len()], &quotient)
                .expect("commit_key.g1s[..quotient.len()].len() == quotient.len()")
                .to_affine()
        };

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
        let blob_scalar = deserialize_blob_to_scalars(blob)?;

        // Convert blob into monomial form.
        let polynomial = blob_scalar_to_polynomial(&self.prover.domain, &blob_scalar);

        // Deserialize the KZG commitment.
        // We only do this to check if it is in the correct subgroup
        let _ = deserialize_compressed_g1(&commitment)?;

        // Compute Fiat-Shamir challenge
        let z = compute_fiat_shamir_challenge(blob, commitment);

        // Compute evaluation and quotient at z.
        // The quotient is returned in "normal order"
        let (quotient, _) = divide_by_linear(&polynomial, z);

        // Compute KZG opening proof.
        let proof = {
            #[cfg(feature = "tracing")]
            let _span = tracing::info_span!("commit quotient").entered();
            g1_lincomb(&self.prover.commit_key.g1s[..quotient.len()], &quotient)
                .expect("commit_key.g1s[..quotient.len()].len() == quotient.len()")
                .to_affine()
        };

        // Serialize the commitment.
        Ok(serialize_g1_compressed(&proof))
    }
}
