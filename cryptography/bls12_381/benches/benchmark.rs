use blstrs::G1Affine;
use crate_crypto_internal_eth_kzg_bls12_381::{
    batch_inversion, ff::Field, fixed_base_msm::FixedBaseMSM, group::Group, lincomb::g1_lincomb,
    G1Projective, Scalar,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::thread_rng;

pub fn batch_inversion(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;

    c.bench_function(
        &format!("bls12_381 batch_inversion size: {}", NUM_ELEMENTS),
        |b| {
            b.iter(|| {
                let mut elements =
                    vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_ELEMENTS];
                batch_inversion::batch_inverse(&mut elements);
            })
        },
    );
}
pub fn fixed_base_msm(c: &mut Criterion) {
    let length = 64;
    let generators: Vec<_> = (0..length)
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect();
    let fbm = FixedBaseMSM::new(generators, 8);
    let scalars: Vec<_> = (0..length)
        .map(|_| Scalar::random(&mut thread_rng()))
        .collect();

    c.bench_function("bls12_381 fixed_base_msm length=64 width=8", |b| {
        b.iter(|| fbm.msm(scalars.clone()))
    });
}
pub fn g1_lincomb_safe(c: &mut Criterion) {
    let length = 64;
    let generators: Vec<G1Affine> = (0..length)
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect();
    let scalars: Vec<_> = (0..length)
        .map(|_| Scalar::random(&mut thread_rng()))
        .collect();

    c.bench_function("g1_lincomb length=64", |b| {
        b.iter(|| g1_lincomb(&generators, &scalars))
    });
}

criterion_group!(benches, batch_inversion, fixed_base_msm, g1_lincomb_safe);
criterion_main!(benches);
