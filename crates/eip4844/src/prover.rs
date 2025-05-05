use bls12_381::{batch_inversion::batch_inverse, ff::Field, lincomb::g1_lincomb, G1Point, Scalar};
use maybe_rayon::prelude::*;
use polynomial::domain::Domain;

use crate::{
    constants::FIELD_ELEMENTS_PER_BLOB,
    serialization::{
        deserialize_blob_to_scalars, deserialize_bytes_to_scalar, deserialize_compressed_g1,
        serialize_g1_compressed,
    },
    trusted_setup::{deserialize_g1_points, SubgroupCheck},
    verifier::{bitreverse, bitreverse_slice, compute_fiat_shamir_challenge},
    BlobRef, Context, Error, KZGCommitment, KZGOpeningEvaluation, KZGOpeningPoint, KZGProof,
    TrustedSetup,
};

/// The key that is used to commit to polynomials in lagrange form (bit-reversed
/// order)
pub struct CommitKey {
    g1_lagrange: Vec<G1Point>,
}

impl From<&TrustedSetup> for CommitKey {
    fn from(setup: &TrustedSetup) -> Self {
        let g1_lagrange = deserialize_g1_points(&setup.g1_lagrange, SubgroupCheck::NoCheck);
        Self { g1_lagrange }
    }
}

pub struct Prover {
    /// Domain used to create the opening proofs.
    domain: Domain,
    /// Commitment key used for committing to the polynomial
    /// in lagrange form
    commit_key: CommitKey,
}

impl Prover {
    pub fn new(trusted_setup: &TrustedSetup) -> Self {
        Self {
            domain: Domain::new(FIELD_ELEMENTS_PER_BLOB),
            commit_key: CommitKey::from(trusted_setup),
        }
    }
}

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

/// Compute evaluation and quotient of the given polynomial at the given point.
fn compute_evaluation_and_quotient(
    domain: &Domain,
    polynomial: &[Scalar],
    z: Scalar,
) -> (Scalar, Vec<Scalar>) {
    // Find the index of point if it's in the domain.
    let point_idx = domain.roots.iter().position(|root| *root == z);

    // Compute evaluation and quotient.
    let (z, quotient) = point_idx.map_or_else(
        || compute_evaluation_and_quotient_out_of_domain(domain, polynomial, z),
        |point_idx| compute_evaluation_and_quotient_within_domain(domain, polynomial, point_idx),
    );

    (z, quotient)
}

/// Compute evaluation and quotient of the given polynomial at the given point.
/// The point is guaranteed to be out-of-domain.
fn compute_evaluation_and_quotient_out_of_domain(
    domain: &Domain,
    polynomial: &[Scalar],
    z: Scalar,
) -> (Scalar, Vec<Scalar>) {
    let mut roots_brp = domain.roots.clone();
    bitreverse_slice(&mut roots_brp);

    // 1 / (z - ω^i)
    let mut denoms = roots_brp.iter().map(|root| z - *root).collect::<Vec<_>>();
    batch_inverse(&mut denoms);

    // \sum (ω^i * f(ω^i) / (z - ω^i)) * ((z^n - 1) / n)
    let y = roots_brp
        .maybe_into_par_iter()
        .zip(polynomial)
        .zip(&denoms)
        .map(|((root, f_root), denom)| root * *f_root * denom)
        .sum::<Scalar>()
        * (z.pow_vartime([FIELD_ELEMENTS_PER_BLOB as u64]) - Scalar::ONE)
        * domain.domain_size_inv;

    // (y - f(ω^i)) / (z - ω^i)
    let quotient = denoms
        .maybe_into_par_iter()
        .zip(polynomial)
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
fn compute_evaluation_and_quotient_within_domain(
    domain: &Domain,
    polynomial: &[Scalar],
    point_idx: usize,
) -> (Scalar, Vec<Scalar>) {
    // ω^m
    let z = domain.roots[point_idx];
    // ω^(n-m)
    let z_inv = domain.roots[(FIELD_ELEMENTS_PER_BLOB - point_idx) % FIELD_ELEMENTS_PER_BLOB];

    // Because polynomial is in bit-reversed order, and we need to compute
    // quotient also in bit-reversed order, so we work with bit-reversed point
    // index from now on.
    let point_idx_brp = bitreverse(point_idx as u32, FIELD_ELEMENTS_PER_BLOB.ilog2()) as usize;

    // Root in bit-reversed order.
    let mut roots_brp = domain.roots.clone();
    bitreverse_slice(&mut roots_brp);

    // f(ω^m)
    let y = polynomial[point_idx_brp];

    // 1 / (ω^m - ω^j)
    // Note that we set (ω^m - ω^m) to be one to make the later `batch_inverse` work.
    let mut denoms = (&roots_brp)
        .maybe_into_par_iter()
        .enumerate()
        .map(|(idx, root)| {
            (idx == point_idx_brp)
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

    // Compute q(ω^m) = \sum q(ω^j) * (A'(ω_m) / A'(ω_j)) = \sum q(ω^j) * (ω_j / ω_m)
    quotient[point_idx_brp] = Scalar::ZERO;
    quotient[point_idx_brp] = -roots_brp
        .maybe_into_par_iter()
        .zip(&quotient)
        .map(|(root, quotient)| *quotient * root * z_inv)
        .sum::<Scalar>();

    (y, quotient)
}
