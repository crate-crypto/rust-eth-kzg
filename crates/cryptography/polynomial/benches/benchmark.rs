use bls12_381::Scalar;
use bls12_381::{ff::Field, group::Group, G1Projective};
use crate_crypto_internal_eth_kzg_polynomial::{domain::Domain, poly_coeff::poly_eval};
use criterion::{criterion_group, criterion_main, Criterion};

pub fn bench_polynomial_evaluation(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;
    let polynomial = random_scalars(NUM_ELEMENTS);
    let value = Scalar::random(&mut rand::thread_rng());

    c.bench_function("poly_eval", |b| {
        b.iter(|| {
            poly_eval(&polynomial, &value);
        });
    });
}

pub fn bench_fft(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;
    let polynomial = random_scalars(NUM_ELEMENTS);
    let domain = Domain::new(NUM_ELEMENTS);

    c.bench_function(&format!("fft_scalars of size {NUM_ELEMENTS}"), |b| {
        b.iter(|| {
            domain.fft_scalars(polynomial.clone());
        });
    });

    let points = random_g1_points(NUM_ELEMENTS);
    c.bench_function(&format!("fft_group_elements of size {NUM_ELEMENTS}"), |b| {
        b.iter(|| {
            domain.fft_g1(points.clone());
        });
    });
}

fn random_scalars(size: usize) -> Vec<Scalar> {
    let mut scalars = Vec::with_capacity(size);
    for _ in 0..size {
        scalars.push(Scalar::random(&mut rand::thread_rng()));
    }
    scalars
}
fn random_g1_points(size: usize) -> Vec<G1Projective> {
    let mut points = Vec::with_capacity(size);
    for _ in 0..size {
        points.push(G1Projective::random(&mut rand::thread_rng()));
    }
    points
}

criterion_group!(benches, bench_polynomial_evaluation, bench_fft,);
criterion_main!(benches);
