use eip4844::{BlobRef, KZGProof, SerializedScalar};

use crate::{Bytes48Ref, DASContext, Error};

// EIP-4844 methods re-exported
//
// The exception is `blob_to_kzg_commitment` because that method is also
// in the eip7594 API.
impl DASContext {
    /// Computes the KZG proof given a blob and a point.
    ///
    /// Note: This method has been re-exported from the eip4844 crate.
    pub fn compute_kzg_proof(
        &self,
        blob: BlobRef,
        z: SerializedScalar,
    ) -> Result<(KZGProof, SerializedScalar), Error> {
        self.eip4844_ctx
            .compute_kzg_proof(blob, z)
            .map_err(Error::EIP4844)
    }

    /// Compute the KZG proof given a blob and its corresponding commitment.
    ///
    /// Note: This method has been re-exported from the eip4844 crate.
    pub fn compute_blob_kzg_proof(
        &self,
        blob: BlobRef,
        commitment: Bytes48Ref,
    ) -> Result<KZGProof, Error> {
        self.eip4844_ctx
            .compute_blob_kzg_proof(blob, commitment)
            .map_err(Error::EIP4844)
    }

    /// Verify the KZG proof to the commitment.
    ///
    /// Note: This method has been re-exported from the eip4844 crate.
    pub fn verify_kzg_proof(
        &self,
        commitment: Bytes48Ref,
        z: SerializedScalar,
        y: SerializedScalar,
        proof: Bytes48Ref,
    ) -> Result<(), Error> {
        self.eip4844_ctx
            .verify_kzg_proof(commitment, z, y, proof)
            .map_err(Error::EIP4844)
    }

    /// Verify the KZG proof to the commitment of a blob.
    ///
    /// Note: This method has been re-exported from the eip4844 crate.
    pub fn verify_blob_kzg_proof(
        &self,
        blob: BlobRef,
        commitment: Bytes48Ref,
        proof: Bytes48Ref,
    ) -> Result<(), Error> {
        self.eip4844_ctx
            .verify_blob_kzg_proof(blob, commitment, proof)
            .map_err(Error::EIP4844)
    }

    /// Verify a batch of KZG proof to a the commitment of a blob.
    ///
    /// Note: This method has been re-exported from the eip4844 crate.
    pub fn verify_blob_kzg_proof_batch(
        &self,
        blobs: Vec<BlobRef>,
        commitments: Vec<Bytes48Ref>,
        proofs: Vec<Bytes48Ref>,
    ) -> Result<(), Error> {
        self.eip4844_ctx
            .verify_blob_kzg_proof_batch(blobs, commitments, proofs)
            .map_err(Error::EIP4844)
    }
}
