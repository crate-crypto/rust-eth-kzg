use crate::{
    cryptography::{
        compute_fiat_shamir_challenge,
        verifier::{compute_evaluation, compute_r_powers_for_verify_kzg_proof_batch},
    },
    serialization::{
        deserialize_blob_to_scalars, deserialize_bytes_to_scalar, deserialize_compressed_g1,
    },
    BlobRef, Context, Error, KZGCommitment, KZGOpeningEvaluation, KZGOpeningPoint, KZGProof,
    VerifierError,
};

impl Context {
    /// Verify the KZG proof to the commitment.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#verify_kzg_proof
    pub fn verify_kzg_proof(
        &self,
        commitment: KZGCommitment,
        z: KZGOpeningPoint,
        y: KZGOpeningEvaluation,
        proof: KZGProof,
    ) -> Result<(), Error> {
        // Deserialize the KZG commitment.
        let commitment = deserialize_compressed_g1(&commitment)?;

        // Deserialize the KZG proof.
        let proof = deserialize_compressed_g1(&proof)?;

        // Deserialize the point into scalar.
        let z = deserialize_bytes_to_scalar(&z)?;

        // Deserialize the evaluation into scalar.
        let y = deserialize_bytes_to_scalar(&y)?;

        // Verify KZG proof.
        self.verifier.verify_kzg_proof(commitment, z, y, proof)?;

        Ok(())
    }

    /// Verify the KZG proof to the commitment of a blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#verify_blob_kzg_proof
    pub fn verify_blob_kzg_proof(
        &self,
        blob: BlobRef,
        commitment: KZGCommitment,
        proof: KZGProof,
    ) -> Result<(), Error> {
        // Deserialize the blob into scalars.
        let polynomial = deserialize_blob_to_scalars(blob)?;

        // Deserialize the KZG commitment.
        let commitment_g1 = deserialize_compressed_g1(&commitment)?;

        // Deserialize the KZG proof.
        let proof = deserialize_compressed_g1(&proof)?;

        // Compute Fiat-Shamir challenge
        let z = compute_fiat_shamir_challenge(blob, commitment);

        // Compute evaluation at z.
        let y = compute_evaluation(&self.verifier.domain, &polynomial, z);

        // Verify KZG proof.
        self.verifier.verify_kzg_proof(commitment_g1, z, y, proof)?;

        Ok(())
    }

    /// Verify a batch of KZG proof to a the commitment of a blob.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#verify_blob_kzg_proof_batch
    pub fn verify_blob_kzg_proof_batch(
        &self,
        blobs: &[BlobRef],
        commitments: &[KZGCommitment],
        proofs: &[KZGProof],
    ) -> Result<(), Error> {
        let same_length = (blobs.len() == commitments.len()) & (blobs.len() == proofs.len());
        if !same_length {
            return Err(VerifierError::BatchVerificationInputsMustHaveSameLength {
                blobs_len: blobs.len(),
                commitments_len: commitments.len(),
                proofs_len: proofs.len(),
            }
            .into());
        }

        // Deserialize the blobs into scalars.
        let polynomials = blobs
            .iter()
            .map(|blob| deserialize_blob_to_scalars(*blob))
            .collect::<Result<Vec<_>, _>>()?;

        // Deserialize the KZG commitments.
        let commitments_g1 = commitments
            .iter()
            .map(|commitment| deserialize_compressed_g1(commitment))
            .collect::<Result<Vec<_>, _>>()?;

        // Deserialize the KZG proofs.
        let proofs_g1 = proofs
            .iter()
            .map(|proof| deserialize_compressed_g1(proof))
            .collect::<Result<Vec<_>, _>>()?;

        // Compute each Fiat-Shamir challenge and evaluation for each proof.
        let (zs, ys) = blobs
            .iter()
            .zip(&polynomials)
            .zip(commitments)
            .map(|((blob, polynomial), commitment)| {
                // Compute Fiat-Shamir challenge
                let z = compute_fiat_shamir_challenge(blob, *commitment);

                // Compute evaluation at z.
                let y = compute_evaluation(&self.verifier.domain, polynomial, z);

                (z, y)
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let domain_size = self.verifier.domain.roots.len();

        // Compute powers Fiat-Shamir challenge for KZG batch verification.
        let r_powers =
            compute_r_powers_for_verify_kzg_proof_batch(domain_size, commitments, &zs, &ys, proofs);

        // Verify KZG proof in batch.
        self.verifier
            .verify_kzg_proof_batch(&commitments_g1, &zs, &ys, &proofs_g1, &r_powers)?;

        Ok(())
    }
}
