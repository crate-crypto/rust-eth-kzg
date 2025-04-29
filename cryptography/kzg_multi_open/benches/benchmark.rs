use bls12_381::fixed_base_msm::UsePrecomp;
use bls12_381::{ff::Field, G1Projective};
use bls12_381::{g1_batch_normalize, g2_batch_normalize, G2Projective, Scalar};
use crate_crypto_kzg_multi_open_fk20::Verifier;
use crate_crypto_kzg_multi_open_fk20::{
    commit_key::CommitKey, verification_key::VerificationKey, Prover, ProverInput,
};
use criterion::{criterion_group, criterion_main, Criterion};

pub fn bench_compute_proof_fk20(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;
    let polynomial_4096 = random_scalars(POLYNOMIAL_LEN);
    let (ck, _) = create_insecure_commit_verification_keys();
    const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

    const NUMBER_OF_POINTS_PER_PROOF: usize = 64;

    let prover = Prover::new(
        ck,
        POLYNOMIAL_LEN,
        NUMBER_OF_POINTS_PER_PROOF,
        NUMBER_OF_POINTS_TO_EVALUATE,
        UsePrecomp::Yes { width: 8 },
    );

    let num_proofs = prover.num_proofs();
    c.bench_function(
        &format!(
            "computing proofs with fk20. POLY_SIZE {}, NUM_INPUT_POINTS {}, NUM_PROOFS {}",
            POLYNOMIAL_LEN, NUMBER_OF_POINTS_PER_PROOF, num_proofs
        ),
        |b| {
            b.iter(|| {
                prover.compute_multi_opening_proofs(ProverInput::PolyCoeff(polynomial_4096.clone()))
            })
        },
    );
}

pub fn bench_verify_proof_fk20(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;
    let polynomial_4096 = random_scalars(POLYNOMIAL_LEN);
    let (ck, vk) = create_insecure_commit_verification_keys();
    const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

    const NUMBER_OF_POINTS_PER_PROOF: usize = 64;

    let prover = Prover::new(
        ck,
        POLYNOMIAL_LEN,
        NUMBER_OF_POINTS_PER_PROOF,
        NUMBER_OF_POINTS_TO_EVALUATE,
        UsePrecomp::Yes { width: 8 },
    );
    let num_proofs = prover.num_proofs();
    let commitment = prover.commit(ProverInput::PolyCoeff(polynomial_4096.clone()));
    let verifier = Verifier::new(vk, NUMBER_OF_POINTS_TO_EVALUATE, prover.num_proofs());

    let (proofs, coset_evals) =
        prover.compute_multi_opening_proofs(ProverInput::PolyCoeff(polynomial_4096));

    c.bench_function(
        &format!(
            "verifying proofs. POLY_SIZE {}, NUM_INPUT_POINTS {}, NUM_PROOFS {}",
            POLYNOMIAL_LEN, NUMBER_OF_POINTS_PER_PROOF, num_proofs
        ),
        |b| {
            b.iter(|| {
                verifier.verify_multi_opening(
                    &[commitment],
                    &vec![0; 128],
                    &(0..128).collect::<Vec<_>>(),
                    &coset_evals,
                    &proofs,
                )
            })
        },
    );
}

fn random_scalars(size: usize) -> Vec<Scalar> {
    let mut scalars = Vec::with_capacity(size);
    for _ in 0..size {
        scalars.push(Scalar::random(&mut rand::thread_rng()))
    }
    scalars
}

// We duplicate this to ensure that the version in the src code is only ever compiled with the test feature.
//
// This code should never be used outside of benchmarks and tests.
pub fn create_insecure_commit_verification_keys() -> (CommitKey, VerificationKey) {
    // A single proof will attest to the opening of 64 points.
    let multi_opening_size = 64;

    // We are making claims about a polynomial which has 4096 coefficients;
    let num_coefficients_in_polynomial = 4096;
    use bls12_381::ff::Field;
    use bls12_381::group::Group;

    let g1_gen = G1Projective::generator();

    let secret = Scalar::random(&mut rand::thread_rng());

    let mut g1_points = Vec::new();
    let mut current_secret_pow = secret;
    for _ in 0..num_coefficients_in_polynomial {
        g1_points.push(g1_gen * current_secret_pow);
        current_secret_pow *= secret;
    }
    let g1_points = g1_batch_normalize(&g1_points);

    let ck = CommitKey::new(g1_points.clone());

    let mut g2_points = Vec::new();
    let mut current_secret_pow = secret;
    let g2_gen = G2Projective::generator();
    // The setup needs 65 g1 elements for the verification key, in order
    // to commit to the remainder polynomial.
    for _ in 0..multi_opening_size + 1 {
        g2_points.push(g2_gen * current_secret_pow);
        current_secret_pow *= secret;
    }
    let g2_points = g2_batch_normalize(&g2_points);

    let vk = VerificationKey::new(
        g1_points[0..multi_opening_size + 1].to_vec(),
        g2_points,
        multi_opening_size,
        num_coefficients_in_polynomial,
    );

    (ck, vk)
}

criterion_group!(benches, bench_compute_proof_fk20, bench_verify_proof_fk20);
criterion_main!(benches);
