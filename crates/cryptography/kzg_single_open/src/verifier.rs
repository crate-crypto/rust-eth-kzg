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
    // Precomputed G2Prepared values for efficiency
    pub gen_g2_prepared: G2Prepared,
    pub tau_g2_prepared: G2Prepared,
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

        // Compute [f(τ) - f(z) + z*q(τ)]G₁
        // This is equivalent to [f(τ) - y + z*q(τ)]G₁
        let lhs_g1 = {
            // First compute [y - z*q(τ)]G₁
            let y_minus_zq = vk.gen_g1 * y - proof * z;
            // Then compute [f(τ) - (y - z*q(τ))]G₁ = [f(τ) - y + z*q(τ)]G₁
            (commitment - y_minus_zq).to_affine()
        };

        // [-q(τ)]G₁
        let rhs_g1 = -proof;

        // Check whether e([f(τ) - f(z) + z*q(τ)]G₁, G₂) * e([-q(τ)]G₁, [τ]G₂) == 1
        // Use precomputed G2Prepared values
        multi_pairings(&[(&lhs_g1, &vk.gen_g2_prepared), (&rhs_g1, &vk.tau_g2_prepared)])
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

        // Compute \sum (r^i * [f_i(τ) - y_i + z_i * q_i(τ)]G₁)
        let lhs_g1 = {
            let points = chain![commitments, [&vk.gen_g1], proofs]
                .copied()
                .collect_vec();
            let scalars = {
                // \sum r^i * y_i
                let y_lincomb: Scalar = izip!(r_powers, ys).map(|(r_i, y_i)| r_i * y_i).sum();
                // r^i * z_i for each proof
                let r_z = r_powers.iter().zip(zs).map(|(r_i, z_i)| r_i * z_i);
                chain![cloned(r_powers), [-y_lincomb], r_z].collect_vec()
            };
            g1_lincomb(&points, &scalars)
                .expect("points.len() == scalars.len()")
                .to_affine()
        };

        // -\sum (r^i * [q_i(τ)]G₁)
        let rhs_g1 = {
            let neg_r_powers: Vec<Scalar> = r_powers.iter().map(|r| -r).collect();
            g1_lincomb(proofs, &neg_r_powers)
                .expect("proofs.len() == neg_r_powers.len()")
                .to_affine()
        };

        // Check the pairing equation using precomputed G2Prepared values
        multi_pairings(&[(&lhs_g1, &vk.gen_g2_prepared), (&rhs_g1, &vk.tau_g2_prepared)])
            .then_some(())
            .ok_or(VerifierError::InvalidProof)
    }
}
