use bls12_381::Scalar;
use bls12_381::{ff::Field, group::Group, G1Projective};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polynomial::{domain::Domain, monomial::poly_eval};

pub fn bench_poly_eval(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;
    let polynomial = vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_ELEMENTS];
    let value = Scalar::random(&mut rand::thread_rng());

    c.bench_function("poly_eval", |b| {
        b.iter(|| {
            poly_eval(&polynomial, &value);
        })
    });
}
pub fn bench_fft(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;
    let polynomial = vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_ELEMENTS];
    let domain = Domain::new(NUM_ELEMENTS);

    c.bench_function(&format!("fft_scalars of size {}", NUM_ELEMENTS), |b| {
        b.iter(|| {
            domain.fft_scalars(polynomial.clone());
        })
    });

    let points = vec![black_box(G1Projective::random(&mut rand::thread_rng())); NUM_ELEMENTS];
    c.bench_function(
        &format!("fft_group_elements of size {}", NUM_ELEMENTS),
        |b| {
            b.iter(|| {
                domain.fft_g1(points.clone());
            })
        },
    );
}

criterion_group!(benches, bench_poly_eval, bench_fft);
criterion_main!(benches);
