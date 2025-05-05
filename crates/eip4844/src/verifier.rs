use bls12_381::{reduce_bytes_to_scalar_bias, Scalar};
use sha2::{Digest, Sha256};

use crate::{
    constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT, FIELD_ELEMENTS_PER_BLOB},
    cryptography::verifier::{compute_evaluation, compute_r_powers_for_verify_kzg_proof_batch},
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

/// Compute Fiat-Shamir challenge of a blob KZG proof.
///
/// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#compute_challenge
pub(crate) fn compute_fiat_shamir_challenge(blob: BlobRef, commitment: KZGCommitment) -> Scalar {
    // DomSepProtocol is a Domain Separator to identify the protocol.
    //
    // It matches [FIAT_SHAMIR_PROTOCOL_DOMAIN] in the spec.
    //
    // [FIAT_SHAMIR_PROTOCOL_DOMAIN]: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#blob
    const DOMAIN_SEP: &str = "FSBLOBVERIFY_V1_";

    let hash_input_size = DOMAIN_SEP.len()
            + 2 * size_of::<u64>() // polynomial bound
            + BYTES_PER_BLOB // blob
            + BYTES_PER_COMMITMENT // commitment
            ;

    let mut hash_input: Vec<u8> = Vec::with_capacity(hash_input_size);

    hash_input.extend(DOMAIN_SEP.as_bytes());
    hash_input.extend(u64_to_byte_array_16(FIELD_ELEMENTS_PER_BLOB as u64));
    hash_input.extend(blob);
    hash_input.extend(commitment);

    assert_eq!(hash_input.len(), hash_input_size);
    let mut hasher = Sha256::new();
    hasher.update(hash_input);
    let result: [u8; 32] = hasher.finalize().into();

    // For randomization, we only need a 128 bit scalar, since this is used for batch verification.
    // See for example, the randomizers section in : https://cr.yp.to/badbatch/badbatch-20120919.pdf
    //
    // This is noted because when we convert a 256 bit hash to a scalar, a bias will be introduced.
    // This however does not affect our security guarantees because the bias is negligible given we
    // want a uniformly random 128 bit integer.
    //
    // Also there is a negligible probably that the scalar is zero, so we do not handle this case here.
    reduce_bytes_to_scalar_bias(result)
}

/// Converts a u64 to a byte array of length 16 in big endian format.
/// This implies that the first 8 bytes of the result are always 0.
fn u64_to_byte_array_16(number: u64) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[8..].copy_from_slice(&number.to_be_bytes());
    bytes
}
