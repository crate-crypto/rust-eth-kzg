use bls12_381::{ff::PrimeField, reduce_bytes_to_scalar_bias, G1Point, Scalar};
use sha2::{Digest, Sha256};

use crate::{BlobRef, KZGCommitment};

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

    let bytes_per_commitment = G1Point::compressed_size();
    let bytes_per_blob = blob.len();

    let bytes_per_field_element = Scalar::NUM_BITS.div_ceil(8) as usize;
    let field_elements_per_blob = blob.len() / bytes_per_field_element;

    let hash_input_size = DOMAIN_SEP.len()
            + 2 * size_of::<u64>() // polynomial bound
            + bytes_per_blob // blob
            + bytes_per_commitment // commitment
            ;

    let mut hash_input: Vec<u8> = Vec::with_capacity(hash_input_size);

    hash_input.extend(DOMAIN_SEP.as_bytes());
    hash_input.extend(u64_to_byte_array_16(field_elements_per_blob as u64));
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

pub mod verifier {
    use std::iter::successors;

    use bls12_381::{
        batch_inversion::batch_inverse,
        ff::{Field, PrimeField},
        group::Curve,
        lincomb::g1_lincomb,
        multi_pairings, reduce_bytes_to_scalar_bias, G1Point, G2Point, G2Prepared, Scalar,
    };
    use polynomial::domain::Domain;
    use sha2::{Digest, Sha256};

    use crate::{
        cryptography::bitreverse_slice, trusted_setup::TrustedSetup, KZGCommitment, KZGProof,
        VerifierError,
    };

    /// The key that is used to verify KZG single-point opening proofs.
    pub struct VerificationKey {
        pub gen_g1: G1Point,
        pub gen_g2: G2Point,
        pub tau_g2: G2Point,
    }

    pub struct Verifier {
        /// Domain used to create the opening proofs.
        pub domain: Domain,
        /// Verification key used to verify KZG single-point opening proofs.
        pub verification_key: VerificationKey,
    }

    impl Verifier {
        pub fn new(domain_size: usize, trusted_setup: &TrustedSetup) -> Self {
            Self {
                domain: Domain::new(domain_size),
                verification_key: VerificationKey::from(trusted_setup),
            }
        }

        pub fn verify_kzg_proof(
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

        pub fn verify_kzg_proof_batch(
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
        let domain_size = domain.roots.len();

        // Bit-reverse polynomial into normal order.
        // Note: This clone is okay because after eip7594, this crate is no longer on the critical path.
        let mut polynomial = polynomial.to_vec();
        bitreverse_slice(&mut polynomial);

        // 1 / (z - ω^i)
        let mut denoms = domain
            .roots
            .iter()
            .map(|root| z - *root)
            .collect::<Vec<_>>();
        batch_inverse(&mut denoms);

        // \sum (ω^i * f(ω^i) / (z - ω^i)) * ((z^n - 1) / n)
        let y = domain
            .roots
            .iter()
            .zip(&polynomial)
            .zip(&denoms)
            .map(|((root, f_root), denom)| root * *f_root * denom)
            .sum::<Scalar>()
            * (z.pow_vartime([domain_size as u64]) - Scalar::ONE)
            * domain.domain_size_inv;

        y
    }

    /// Compute random linear combination challenge scalars for batch verification.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#verify_kzg_proof_batch
    pub fn compute_r_powers_for_verify_kzg_proof_batch(
        domain_size: usize,
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

        let bytes_per_commitment = G1Point::compressed_size();
        let bytes_per_field_element = Scalar::NUM_BITS.div_ceil(8) as usize;

        let n = commitments.len();

        let hash_input_size = DOMAIN_SEP.len()
        + size_of::<u64>() // polynomial bound
        + size_of::<u64>() // batch size
        + n * (
            bytes_per_commitment // commitment
            + bytes_per_field_element // z 
            + bytes_per_field_element // y
            + bytes_per_commitment // proof
        );

        let mut hash_input: Vec<u8> = Vec::with_capacity(hash_input_size);

        hash_input.extend(DOMAIN_SEP.as_bytes());
        hash_input.extend((domain_size as u64).to_be_bytes());
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
}

pub mod prover {
    use bls12_381::{batch_inversion::batch_inverse, ff::Field, G1Point, Scalar};
    use maybe_rayon::prelude::*;
    use polynomial::domain::Domain;

    use crate::{cryptography::bitreverse_slice, TrustedSetup};

    /// The key that is used to commit to polynomials in lagrange form.
    pub struct CommitKey {
        pub g1_lagrange: Vec<G1Point>,
    }

    pub struct Prover {
        /// Domain used to create the opening proofs.
        pub domain: Domain,
        /// Commitment key used for committing to the polynomial
        /// in lagrange form
        pub commit_key: CommitKey,
    }

    impl Prover {
        pub fn new(domain_size: usize, trusted_setup: &TrustedSetup) -> Self {
            Self {
                domain: Domain::new(domain_size),
                commit_key: CommitKey::from(trusted_setup),
            }
        }
    }

    /// Compute evaluation and quotient of the given polynomial at the given point.
    ///
    /// Note: The quotient is returned in normal order.
    pub fn compute_evaluation_and_quotient(
        domain: &Domain,
        polynomial: &[Scalar],
        z: Scalar,
    ) -> (Scalar, Vec<Scalar>) {
        // Find the index of point if it's in the domain.
        let point_idx = domain.roots.iter().position(|root| *root == z);

        // Compute evaluation and quotient.
        let (z, quotient) = point_idx.map_or_else(
            || compute_evaluation_and_quotient_out_of_domain(domain, polynomial, z),
            |point_idx| {
                compute_evaluation_and_quotient_within_domain(domain, polynomial, point_idx)
            },
        );

        (z, quotient)
    }

    /// Compute evaluation and quotient of the given polynomial at the given point.
    /// The point is guaranteed to be out-of-domain.
    pub fn compute_evaluation_and_quotient_out_of_domain(
        domain: &Domain,
        polynomial: &[Scalar],
        z: Scalar,
    ) -> (Scalar, Vec<Scalar>) {
        // Bit-reverse polynomial into normal order.mal order.
        let mut polynomial = polynomial.to_vec();
        bitreverse_slice(&mut polynomial);

        // 1 / (z - ω^i)
        let mut denoms = (&domain.roots)
            .maybe_into_par_iter()
            .map(|root| z - *root)
            .collect::<Vec<_>>();
        batch_inverse(&mut denoms);

        let domain_size = domain.roots.len();

        // \sum (ω^i * f(ω^i) / (z - ω^i)) * ((z^n - 1) / n)
        let y = (&domain.roots)
            .maybe_into_par_iter()
            .zip(&polynomial)
            .zip(&denoms)
            .map(|((root, f_root), denom)| root * *f_root * denom)
            .sum::<Scalar>()
            * (z.pow_vartime([domain_size as u64]) - Scalar::ONE)
            * domain.domain_size_inv;

        // (y - f(ω^i)) / (z - ω^i)
        let quotient = denoms
            .maybe_into_par_iter()
            .zip(&polynomial)
            .map(|(denom, f_root)| (y - *f_root) * denom)
            .collect();

        (y, quotient)
    }

    /// Compute evaluation and quotient of the given polynomial at the given point
    /// index of the domain.
    ///
    /// For more details, read [PCS multiproofs using random evaluation] section
    /// "Dividing when one of the points is zero".
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/017a8495f7671f5fff2075a9bfc9238c1a0982f8/specs/deneb/polynomial-commitments.md#compute_quotient_eval_within_domain
    ///
    /// [PCS multiproofs using random evaluation]: https://dankradfeist.de/ethereum/2021/06/18/pcs-multiproofs.html
    pub fn compute_evaluation_and_quotient_within_domain(
        domain: &Domain,
        polynomial: &[Scalar],
        point_idx: usize,
    ) -> (Scalar, Vec<Scalar>) {
        let domain_size = domain.roots.len();

        // Bit-reverse polynomial into normal order.
        let mut polynomial = polynomial.to_vec();
        bitreverse_slice(&mut polynomial);

        // ω^m
        let z = domain.roots[point_idx];

        // f(ω^m)
        let y = polynomial[point_idx];

        // 1 / (ω^m - ω^j)
        // Note that we set (ω^m - ω^m) to be one to make the later `batch_inverse` work.
        let mut denoms = (&domain.roots)
            .maybe_into_par_iter()
            .enumerate()
            .map(|(idx, root)| {
                (idx == point_idx)
                    .then_some(Scalar::ONE)
                    .unwrap_or_else(|| z - root)
            })
            .collect::<Vec<_>>();
        batch_inverse(&mut denoms);

        // (f(ω^m) - f(ω^j)) / (ω^m - ω^j)
        let mut quotient = denoms
            .maybe_into_par_iter()
            .zip(polynomial)
            .map(|(denom, f_root)| (y - f_root) * denom)
            .collect::<Vec<_>>();

        // Compute q(ω^m) = \sum q(ω^j) * (A'(ω^m) / A'(ω^j)) = \sum q(ω^j) * ω^{j - m}
        quotient[point_idx] = Scalar::ZERO;
        quotient[point_idx] = -(&quotient)
            .maybe_into_par_iter()
            .enumerate()
            .map(|(idx, quotient)| {
                let root_j_mimus_m = domain.roots[(domain_size + idx - point_idx) % domain_size];
                *quotient * root_j_mimus_m
            })
            .sum::<Scalar>();

        (y, quotient)
    }
}
