use bls12_381::{ff::Field, group::Group, G1Projective};
use bls12_381::{G2Projective, Scalar};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kzg_multi_open::lincomb::{g1_lincomb, g1_lincomb_unsafe, g2_lincomb, g2_lincomb_unsafe};

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

criterion_group!(benches, bench_msm);
criterion_main!(benches);
