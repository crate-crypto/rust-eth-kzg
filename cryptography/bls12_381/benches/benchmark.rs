use blstrs::Scalar;
use crate_crypto_internal_eth_kzg_bls12_381::{
    batch_inversion,
    ff::Field,
    fixed_base_msm::{FixedBaseMSM, UsePrecomp},
    fixed_base_msm_pippenger::FixedBaseMSMPippenger,
    g1_batch_normalize, g2_batch_normalize,
    group::Group,
    lincomb::{g1_lincomb, g1_lincomb_unsafe, g2_lincomb, g2_lincomb_unsafe},
    G1Projective, G2Projective,
};
use criterion::{criterion_group, criterion_main, Criterion};

pub fn batch_inversion(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;

    c.bench_function(
        &format!("bls12_381 batch_inversion size: {}", NUM_ELEMENTS),
        |b| {
            b.iter(|| {
                let mut elements = random_scalars(NUM_ELEMENTS);
                batch_inversion::batch_inverse(&mut elements);
            })
        },
    );
}
pub fn fixed_base_msm(c: &mut Criterion) {
    let length = 64;
    let generators: Vec<_> = random_g1_points(length)
        .into_iter()
        .map(|p| p.into())
        .collect();
    let fbm = FixedBaseMSM::new(generators.clone(), UsePrecomp::Yes { width: 8 });
    let scalars: Vec<_> = random_scalars(length);

    c.bench_function("bls12_381 fixed_base_msm length=64 width=8", |b| {
        b.iter(|| fbm.msm(scalars.clone()))
    });

    let fixed_base_pip = FixedBaseMSMPippenger::new(&generators);

    c.bench_function("bls12_381 fixed based pippenger algorithm", |b| {
        b.iter(|| fixed_base_pip.msm(&scalars))
    });
}

pub fn bench_msm(c: &mut Criterion) {
    const NUM_G1_ELEMENTS: usize = 64;

    let polynomial_4096 = random_scalars(NUM_G1_ELEMENTS);
    let g1_elements_proj = random_g1_points(NUM_G1_ELEMENTS);
    let g1_elements = g1_batch_normalize(&g1_elements_proj);

    c.bench_function(&format!("g1 msm of size {}", NUM_G1_ELEMENTS), |b| {
        b.iter(|| g1_lincomb_unsafe(&g1_elements, &polynomial_4096))
    });

    c.bench_function(&format!("g1 (safe) msm of size {}", NUM_G1_ELEMENTS), |b| {
        b.iter(|| g1_lincomb(&g1_elements, &polynomial_4096))
    });

    const NUM_G2_ELEMENTS: usize = 65;

    let polynomial_65 = random_scalars(NUM_G2_ELEMENTS);
    let g2_elements = random_g2_points(NUM_G2_ELEMENTS);
    let g2_elements = g2_batch_normalize(&g2_elements);

    c.bench_function(&format!("g2 msm of size {}", NUM_G2_ELEMENTS), |b| {
        b.iter(|| g2_lincomb_unsafe(&g2_elements, &polynomial_65))
    });
    c.bench_function(&format!("g2 (safe) msm of size {}", NUM_G2_ELEMENTS), |b| {
        b.iter(|| g2_lincomb(&g2_elements, &polynomial_65))
    });
}

fn random_scalars(size: usize) -> Vec<Scalar> {
    let mut scalars = Vec::with_capacity(size);
    for _ in 0..size {
        scalars.push(Scalar::random(&mut rand::thread_rng()))
    }
    scalars
}
fn random_g1_points(size: usize) -> Vec<G1Projective> {
    let mut points = Vec::with_capacity(size);
    for _ in 0..size {
        points.push(G1Projective::random(&mut rand::thread_rng()))
    }
    points
}
fn random_g2_points(size: usize) -> Vec<G2Projective> {
    let mut points = Vec::with_capacity(size);
    for _ in 0..size {
        points.push(G2Projective::random(&mut rand::thread_rng()))
    }
    points
}

criterion_group!(
    benches,
    batch_inversion,
    fixed_base_msm,
    bench_msm,
    fixed_base_msm
);
criterion_main!(benches);
