use bls12_381::lincomb::{g1_lincomb, g1_lincomb_unsafe, g2_lincomb, g2_lincomb_unsafe};
use bls12_381::{ff::Field, group::Group, G1Projective};
use bls12_381::{G2Projective, Scalar};
use crate_crypto_kzg_multi_open_fk20::commit_key::CommitKey;
use crate_crypto_kzg_multi_open_fk20::fk20::{FK20Prover, ProverInput};
use crate_crypto_kzg_multi_open_fk20::opening_key::OpeningKey;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn bench_msm(c: &mut Criterion) {
    const NUM_G1_ELEMENTS: usize = 4096;

    let polynomial_4096 = vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_G1_ELEMENTS];
    let g1_elements =
        vec![black_box(G1Projective::random(&mut rand::thread_rng())); NUM_G1_ELEMENTS];

    c.bench_function(&format!("g1 msm of size {}", NUM_G1_ELEMENTS), |b| {
        b.iter(|| g1_lincomb_unsafe(&g1_elements, &polynomial_4096))
    });
    c.bench_function(&format!("g1 (safe) msm of size {}", NUM_G1_ELEMENTS), |b| {
        b.iter(|| g1_lincomb(&g1_elements, &polynomial_4096))
    });

    const NUM_G2_ELEMENTS: usize = 65;

    let polynomial_65 = vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_G2_ELEMENTS];
    let g2_elements =
        vec![black_box(G2Projective::random(&mut rand::thread_rng())); NUM_G2_ELEMENTS];

    c.bench_function(&format!("g2 msm of size {}", NUM_G2_ELEMENTS), |b| {
        b.iter(|| g2_lincomb_unsafe(&g2_elements, &polynomial_65))
    });
    c.bench_function(&format!("g2 (safe) msm of size {}", NUM_G2_ELEMENTS), |b| {
        b.iter(|| g2_lincomb(&g2_elements, &polynomial_65))
    });
}

// Note: This is just here for reference. We can remove this once, we have finished
// implementing the optimized version.
// For prosperity: On my laptop, 128 proofs take about 3.2 seconds, 1 proof takes about 25 milliseconds.
// This is on a single thread.
pub fn bench_compute_proof(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;
    let polynomial_4096 = vec![black_box(Scalar::random(&mut rand::thread_rng())); POLYNOMIAL_LEN];
    let (ck, _) = create_insecure_commit_opening_keys();
    const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

    const NUMBER_OF_POINTS_PER_PROOF: usize = 64;

    let fk20 = FK20Prover::new(
        ck,
        POLYNOMIAL_LEN,
        NUMBER_OF_POINTS_PER_PROOF,
        NUMBER_OF_POINTS_TO_EVALUATE,
    );
    let num_proofs = fk20.num_proofs();
    c.bench_function(
        &format!(
            "computing proofs with fk20. POLY_SIZE {}, NUM_INPUT_POINTS {}, NUM_PROOFS {}",
            POLYNOMIAL_LEN, NUMBER_OF_POINTS_PER_PROOF, num_proofs
        ),
        |b| {
            b.iter(|| {
                fk20.compute_multi_opening_proofs(ProverInput::PolyCoeff(polynomial_4096.clone()))
            })
        },
    );
}

// We duplicate this to ensure that the version in the src code is only ever compiled with the test feature.
//
// This code should never be used outside of benchmarks and tests.
pub fn create_insecure_commit_opening_keys() -> (CommitKey, OpeningKey) {
    // A single proof will attest to the opening of 64 points.
    let multi_opening_size = 64;

    // We are making claims about a polynomial which has 4096 coefficients;
    let num_coefficients_in_polynomial = 4096;
    use bls12_381::ff::Field;
    use bls12_381::group::Group;

    let g1_gen = G1Projective::generator();

    let mut g1_points = Vec::new();
    let secret = -Scalar::from(1 as u64);
    let mut current_secret_pow = Scalar::ONE;
    for _ in 0..num_coefficients_in_polynomial {
        g1_points.push(g1_gen * current_secret_pow);
        current_secret_pow *= secret;
    }
    let ck = CommitKey::new(g1_points.clone());

    let mut g2_points = Vec::new();
    let secret = -Scalar::from(1 as u64);
    let mut current_secret_pow = Scalar::ONE;
    let g2_gen = G2Projective::generator();
    // The setup needs 65 g1 elements for the opening key, in order
    // to commit to the remainder polynomial.
    for _ in 0..multi_opening_size + 1 {
        g2_points.push(g2_gen * current_secret_pow);
        current_secret_pow *= secret;
    }
    let vk = OpeningKey::new(
        g1_points[0..multi_opening_size + 1].to_vec(),
        g2_points,
        multi_opening_size,
        num_coefficients_in_polynomial,
    );

    (ck, vk)
}

criterion_group!(benches, bench_msm, bench_compute_proof,);
criterion_main!(benches);
