use bls12_381::lincomb::{g1_lincomb, g1_lincomb_unsafe, g2_lincomb, g2_lincomb_unsafe};
use bls12_381::{ff::Field, group::Group, G1Projective};
use bls12_381::{G2Projective, Scalar};
use crate_crypto_kzg_multi_open_fk20::create_eth_commit_opening_keys;
use crate_crypto_kzg_multi_open_fk20::fk20::{reverse_bit_order, FK20};
use crate_crypto_kzg_multi_open_fk20::naive::compute_multi_opening;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polynomial::domain::Domain;

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
    let (ck, _) = create_eth_commit_opening_keys();
    const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

    const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
    let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
    let mut domain_extended_roots = domain_extended.roots.clone();
    reverse_bit_order(&mut domain_extended_roots);

    let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
        .chunks(NUMBER_OF_POINTS_PER_PROOF)
        .collect();

    // The results for the naive version are linear, so you can multiply the time taken
    // to compute 1 proof by the number of proofs, you are interested in.
    let num_proofs = 1;
    c.bench_function(
        &format!(
            "computing proofs w/ fk20. POLY_SIZE {}, NUM_INPUT_POINTS {}, NUM_PROOFS {}",
            POLYNOMIAL_LEN, NUMBER_OF_POINTS_PER_PROOF, num_proofs
        ),
        |b| {
            b.iter(|| {
                for input_points in &chunked_bit_reversed_roots[0..num_proofs] {
                    compute_multi_opening(&ck, &polynomial_4096, input_points);
                }
            })
        },
    );

    let fk20 = FK20::new(
        &ck,
        POLYNOMIAL_LEN,
        NUMBER_OF_POINTS_PER_PROOF,
        NUMBER_OF_POINTS_TO_EVALUATE,
    );
    c.bench_function(
        &format!(
            "computing proofs with fk20. POLY_SIZE {}, NUM_INPUT_POINTS {}, NUM_PROOFS {}",
            POLYNOMIAL_LEN,
            NUMBER_OF_POINTS_PER_PROOF,
            chunked_bit_reversed_roots.len()
        ),
        |b| b.iter(|| fk20.compute_multi_opening_proofs(polynomial_4096.clone())),
    );
}

criterion_group!(benches, bench_msm, bench_compute_proof,);
criterion_main!(benches);
