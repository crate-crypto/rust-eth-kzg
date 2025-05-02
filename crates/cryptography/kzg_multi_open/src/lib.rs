pub mod commit_key;
mod fk20;
pub mod verification_key;

pub use fk20::{
    recover_evaluations_in_domain_order, CommitmentIndex, CosetIndex, Prover, ProverInput,
    Verifier, VerifierError,
};

#[cfg(test)]
mod naive;

#[cfg(test)]
pub(crate) fn create_insecure_commit_verification_keys(
) -> (commit_key::CommitKey, verification_key::VerificationKey) {
    use bls12_381::ff::Field;
    use bls12_381::group::Group;
    use bls12_381::{g1_batch_normalize, g2_batch_normalize, G1Projective, G2Projective, Scalar};
    use commit_key::CommitKey;
    use verification_key::VerificationKey;

    // A single proof will attest to the opening of 64 points.
    let multi_opening_size = 64;

    // We are making claims about a polynomial which has 4096 coefficients;
    let num_coefficients_in_polynomial = 4096;

    let g1_gen = G1Projective::generator();

    let mut g1_points = Vec::new();
    let secret = -Scalar::ONE;
    let mut current_secret_pow = Scalar::ONE;
    for _ in 0..num_coefficients_in_polynomial {
        g1_points.push(g1_gen * current_secret_pow);
        current_secret_pow *= secret;
    }
    let g1_points = g1_batch_normalize(&g1_points);

    let ck = CommitKey::new(g1_points.clone());

    let mut g2_points = Vec::new();
    let secret = -Scalar::ONE;
    let mut current_secret_pow = Scalar::ONE;
    let g2_gen = G2Projective::generator();
    // The setup needs 65 g1 elements for the verification key, in order
    // to commit to the remainder polynomial.
    for _ in 0..=multi_opening_size {
        g2_points.push(g2_gen * current_secret_pow);
        current_secret_pow *= secret;
    }
    let g2_points = g2_batch_normalize(&g2_points);

    let vk = VerificationKey::new(
        g1_points[0..=multi_opening_size].to_vec(),
        g2_points,
        multi_opening_size,
        num_coefficients_in_polynomial,
    );

    (ck, vk)
}
