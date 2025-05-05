use std::iter::successors;

use bls12_381::{
    batch_inversion::batch_inverse, ff::Field, group::Curve, lincomb::g1_lincomb, multi_pairings,
    reduce_bytes_to_scalar_bias, G1Point, G2Point, G2Prepared, Scalar,
};
use polynomial::domain::Domain;
use sha2::{Digest, Sha256};

use crate::{
    constants::{
        BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB,
    },
    serialization::{
        deserialize_blob_to_scalars, deserialize_bytes_to_scalar, deserialize_compressed_g1,
    },
    trusted_setup::{deserialize_g1_points, deserialize_g2_points, SubgroupCheck, TrustedSetup},
    BlobRef, Context, Error, KZGCommitment, KZGOpeningEvaluation, KZGOpeningPoint, KZGProof,
    VerifierError,
};

/// The key that is used to verify KZG single-point opening proofs.
pub struct VerificationKey {
    gen_g1: G1Point,
    gen_g2: G2Point,
    tau_g2: G2Point,
}

impl From<&TrustedSetup> for VerificationKey {
    fn from(setup: &TrustedSetup) -> Self {
        let g1_monomial = deserialize_g1_points(&setup.g1_monomial, SubgroupCheck::NoCheck);
        let g2_monomial = deserialize_g2_points(&setup.g2_monomial, SubgroupCheck::NoCheck);
        let gen_g1 = g1_monomial[0];
        let gen_g2 = g2_monomial[0];
        let tau_g2 = g2_monomial[1];
        Self {
            gen_g1,
            gen_g2,
            tau_g2,
        }
    }
}

pub struct Verifier {
    /// Domain used to create the opening proofs.
    domain: Domain,
    /// Verification key used to verify KZG single-point opening proofs.
    verification_key: VerificationKey,
}

impl Verifier {
    pub fn new(trusted_setup: &TrustedSetup) -> Self {
        Self {
            domain: Domain::new(FIELD_ELEMENTS_PER_BLOB),
            verification_key: VerificationKey::from(trusted_setup),
        }
    }

    fn verify_kzg_proof(
        &self,
        commitment: G1Point,
        z: Scalar,
        y: Scalar,
        proof: G1Point,
    ) -> Result<(), VerifierError> {
        let vk = &self.verification_key;

        // [f(τ) - f(z)]G₁
        let commitment_minus_z = (commitment - vk.gen_g1 * y).into();

        // [-1]G₂
        let neg_gen_g2 = G2Prepared::from(-vk.gen_g2);

        // [τ - z]G₂
        let tau_minus_challenge_g2 = G2Prepared::from((vk.tau_g2 - vk.gen_g2 * z).to_affine());

        // Check whether `f(X) - f(z) == q(X) * (X - z)`
        let proof_valid = multi_pairings(&[
            (&commitment_minus_z, &neg_gen_g2),
            (&proof, &tau_minus_challenge_g2),
        ]);
        if proof_valid {
            Ok(())
        } else {
            Err(VerifierError::InvalidProof)
        }
    }

    fn verify_kzg_proof_batch(
        &self,
        commitments: &[G1Point],
        zs: &[Scalar],
        ys: &[Scalar],
        proofs: &[G1Point],
        r_powers: &[Scalar],
    ) -> Result<(), VerifierError> {
        assert!(
            commitments.len() == zs.len()
                && commitments.len() == ys.len()
                && commitments.len() == proofs.len()
                && commitments.len() == r_powers.len()
        );

        let vk = &self.verification_key;

        // \sum r^i * [f_i(τ)] - (\sum r^i * y_i) * [1] + \sum r^i * z_i * [q(τ)]
        let lhs_g1 = {
            let points = commitments
                .iter()
                .chain(proofs)
                .chain([&vk.gen_g1])
                .copied()
                .collect::<Vec<_>>();
            let scalars = r_powers
                .iter()
                .copied()
                .chain(r_powers.iter().zip(zs).map(|(r_i, z_i)| *r_i * z_i))
                .chain([-r_powers
                    .iter()
                    .zip(ys)
                    .map(|(r_i, y_i)| *r_i * y_i)
                    .sum::<Scalar>()])
                .collect::<Vec<_>>();
            g1_lincomb(&points, &scalars)
                .expect("points and scalars have same length")
                .into()
        };

        // \sum r^i * [q(τ)]
        let rhs_g1 = g1_lincomb(proofs, r_powers)
            .expect("points and scalars have same length")
            .into();

        // [-1]G₂
        let lhs_g2 = G2Prepared::from(-vk.gen_g2);

        // [τ]G₂
        let rhs_g2 = G2Prepared::from(vk.tau_g2);

        let proof_valid = multi_pairings(&[(&lhs_g1, &lhs_g2), (&rhs_g1, &rhs_g2)]);
        if proof_valid {
            Ok(())
        } else {
            Err(VerifierError::InvalidProof)
        }
    }
}

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

        // Compute powers Fiat-Shamir challenge for KZG batch verification.
        let r_powers = compute_r_powers_for_verify_kzg_proof_batch(commitments, &zs, &ys, proofs);

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

/// Compute random linear combination challenge scalars for batch verification.
///
/// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#verify_kzg_proof_batch
fn compute_r_powers_for_verify_kzg_proof_batch(
    commitments: &[KZGCommitment],
    zs: &[Scalar],
    ys: &[Scalar],
    proofs: &[KZGProof],
) -> Vec<Scalar> {
    // DomSepProtocol is a Domain Separator to identify the protocol.
    //
    // It matches [RANDOM_CHALLENGE_KZG_BATCH_DOMAIN] in the spec.
    //
    // [RANDOM_CHALLENGE_KZG_BATCH_DOMAIN]: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#blob
    const DOMAIN_SEP: &str = "RCKZGBATCH___V1_";

    let n = commitments.len();

    let hash_input_size = DOMAIN_SEP.len()
        + size_of::<u64>() // polynomial bound
        + size_of::<u64>() // batch size
        + n * (
            BYTES_PER_COMMITMENT // commitment
            + BYTES_PER_FIELD_ELEMENT // z 
            + BYTES_PER_FIELD_ELEMENT // y
            + BYTES_PER_COMMITMENT // proof
        );

    let mut hash_input: Vec<u8> = Vec::with_capacity(hash_input_size);

    hash_input.extend(DOMAIN_SEP.as_bytes());
    hash_input.extend((FIELD_ELEMENTS_PER_BLOB as u64).to_be_bytes());
    hash_input.extend((n as u64).to_be_bytes());
    commitments
        .iter()
        .zip(zs)
        .zip(ys)
        .zip(proofs)
        .for_each(|(((commitment, z), y), proof)| {
            hash_input.extend(commitment);
            hash_input.extend(z.to_bytes_be());
            hash_input.extend(y.to_bytes_be());
            hash_input.extend(proof);
        });

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
    let r = reduce_bytes_to_scalar_bias(result);

    successors(Some(Scalar::ONE), |power| Some(*power * r))
        .take(n)
        .collect()
}

/// Converts a u64 to a byte array of length 16 in big endian format.
/// This implies that the first 8 bytes of the result are always 0.
fn u64_to_byte_array_16(number: u64) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[8..].copy_from_slice(&number.to_be_bytes());
    bytes
}

/// Compute evaluation of the given polynomial at the given point.
pub(crate) fn compute_evaluation(domain: &Domain, polynomial: &[Scalar], z: Scalar) -> Scalar {
    domain.roots.iter().position(|root| *root == z).map_or_else(
        || compute_evaluation_out_of_domain(domain, polynomial, z),
        |position| polynomial[position],
    )
}

/// Compute evaluation of the given polynomial at the given point.
/// The point is guaranteed to be out-of-domain.
pub(crate) fn compute_evaluation_out_of_domain(
    domain: &Domain,
    polynomial: &[Scalar],
    z: Scalar,
) -> Scalar {
    let mut roots_brp = domain.roots.clone();
    bitreverse_slice(&mut roots_brp);

    // 1 / (z - ω^i)
    let mut denoms = roots_brp.iter().map(|root| z - *root).collect::<Vec<_>>();
    batch_inverse(&mut denoms);

    // \sum (ω^i * f(ω^i) / (z - ω^i)) * ((z^n - 1) / n)
    let y = roots_brp
        .iter()
        .zip(polynomial)
        .zip(&denoms)
        .map(|((root, f_root), denom)| root * *f_root * denom)
        .sum::<Scalar>()
        * (z.pow_vartime([FIELD_ELEMENTS_PER_BLOB as u64]) - Scalar::ONE)
        * domain.domain_size_inv;

    y
}

pub(crate) fn bitreverse(mut n: u32, l: u32) -> u32 {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

pub(crate) fn bitreverse_slice<T>(a: &mut [T]) {
    if a.is_empty() {
        return;
    }

    let n = a.len();
    let log_n = n.ilog2();
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k as u32, log_n) as usize;
        if k < rk {
            a.swap(rk, k);
        }
    }
}
