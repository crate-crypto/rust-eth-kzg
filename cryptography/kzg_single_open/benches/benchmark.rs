use bls12_381::Scalar;
use bls12_381::{ff::Field, group::Group, G1Projective};
use crate_crypto_kzg_single_open::compute_proof;
use criterion::{criterion_group, criterion_main, Criterion};
use kzg_multi_open::commit_key::CommitKey;
use polynomial::domain::Domain;

pub fn bench_single_opening_proof(c: &mut Criterion) {
    const NUM_G1_ELEMENTS: usize = 4096;

    let polynomial_4096: Vec<_> = (0..4096)
        .into_iter()
        .map(|i| -Scalar::from(i as u64))
        .collect();
    let G1s: Vec<_> = (0..NUM_G1_ELEMENTS)
        .into_iter()
        .map(|i| (G1Projective::generator() * (Scalar::from((i + 123456789) as u64))).into())
        .collect();
    let ck = CommitKey::new(G1s);
    let rand_point = Scalar::random(&mut rand::thread_rng());
    let domain = Domain::new(4096);
    c.bench_function("compute single proof", |b| {
        b.iter(|| compute_proof(&ck, &domain, &polynomial_4096, rand_point))
    });
}

criterion_group!(benches, bench_single_opening_proof);
criterion_main!(benches);
