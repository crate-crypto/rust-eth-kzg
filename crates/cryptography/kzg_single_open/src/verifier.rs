use bls12_381::{
    lincomb::g1_lincomb, multi_pairings, traits::*, G1Point, G2Point, G2Prepared, Scalar,
};
use itertools::{chain, cloned, izip, Itertools};
use polynomial::domain::Domain;

use crate::VerifierError;

/// The key that is used to verify KZG single-point opening proofs.
#[derive(Debug)]
pub struct VerificationKey {
    pub gen_g1: G1Point,
    pub gen_g2: G2Point,
    pub tau_g2: G2Point,
}

#[derive(Debug)]
pub struct Verifier {
    /// Domain used to create the opening proofs.
    pub domain: Domain,
    /// Verification key used to verify KZG single-point opening proofs.
    pub verification_key: VerificationKey,
}

impl Verifier {
    pub fn new(domain_size: usize, verification_key: VerificationKey) -> Self {
        Self {
            domain: Domain::new(domain_size),
            verification_key,
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
        let lhs_g1 = (commitment - vk.gen_g1 * y).to_affine();

        // [-1]G₂
        let lhs_g2 = G2Prepared::from(-vk.gen_g2);

        // [q(τ)]G₁
        let rhs_g1 = proof;

        // [τ - z]G₂
        let rhs_g2 = G2Prepared::from((vk.tau_g2 - vk.gen_g2 * z).to_affine());

        // Check whether `f(τ) - f(z) == q(τ) * (τ - z)`
        multi_pairings(&[(&lhs_g1, &lhs_g2), (&rhs_g1, &rhs_g2)])
            .then_some(())
            .ok_or(VerifierError::InvalidProof)
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

        // \sum (r^i * [f_i(τ)]G₁) - [\sum (r^i * y_i)]G₁ + \sum (r^i * z_i * [q(τ)]G₁)
        let lhs_g1 = {
            let points = chain![commitments, [&vk.gen_g1], proofs]
                .copied()
                .collect_vec();
            let scalars = {
                // \sum r^i * y_i
                let y_lincomb: Scalar = izip!(r_powers, ys).map(|(r_i, y_i)| r_i * y_i).sum();
                let r_z = r_powers.iter().zip(zs).map(|(r_i, z_i)| r_i * z_i);
                chain![cloned(r_powers), [-y_lincomb], r_z].collect_vec()
            };
            g1_lincomb(&points, &scalars)
                .expect("points.len() == scalars.len()")
                .to_affine()
        };

        // \sum r^i * [q(τ)]G₁
        let rhs_g1 = g1_lincomb(proofs, r_powers)
            .expect("proofs.len() == r_powers.len()")
            .to_affine();

        // [-1]G₂
        let lhs_g2 = G2Prepared::from(-vk.gen_g2);

        // [τ]G₂
        let rhs_g2 = G2Prepared::from(vk.tau_g2);

        // Check whether `\sum (r^i * (f_i(τ) - y_i)) + \sum (r^i * z_i * q(τ)) == \sum (r^i * τ * q(τ))`
        multi_pairings(&[(&lhs_g1, &lhs_g2), (&rhs_g1, &rhs_g2)])
            .then_some(())
            .ok_or(VerifierError::InvalidProof)
    }
}
