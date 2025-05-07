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

pub mod verifier {
    use bls12_381::{
        batch_inversion::batch_inverse, ff::Field, group::Curve, lincomb::g1_lincomb,
        multi_pairings, G1Point, G2Point, G2Prepared, Scalar,
    };
    use itertools::{chain, cloned, izip, Itertools};
    use polynomial::domain::Domain;

    use crate::{kzg_open::bitreverse_slice, trusted_setup::TrustedSetup, VerifierError};

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
        let mut denoms = domain.roots.iter().map(|root| z - root).collect_vec();
        batch_inverse(&mut denoms);

        // \sum (ω^i * f(ω^i) / (z - ω^i)) * ((z^n - 1) / n)
        izip!(&domain.roots, &polynomial, &denoms)
            .map(|(root, f_root, denom)| root * f_root * denom)
            .sum::<Scalar>()
            * (z.pow_vartime([domain_size as u64]) - Scalar::ONE)
            * domain.domain_size_inv
    }
}

pub mod prover {
    use bls12_381::{batch_inversion::batch_inverse, ff::Field, G1Point, Scalar};
    use maybe_rayon::prelude::*;
    use polynomial::domain::Domain;

    use crate::{kzg_open::bitreverse_slice, TrustedSetup};

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
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
            .map(|root| z - root)
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
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
                if idx == point_idx {
                    Scalar::ONE
                } else {
                    z - root
                }
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
