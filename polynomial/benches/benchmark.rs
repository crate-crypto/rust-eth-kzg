use bls12_381::Scalar;
use bls12_381::{ff::Field, group::Group, G1Projective};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polynomial::monomial::lagrange_interpolate;
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

pub fn bench_lagrange_interpolation(c: &mut Criterion) {
    // Note: We have commented out 4096 as it takes too much time in the benchmarks
    // and this usecase is not needed.
    for size in [65 /*4096*/] {
        let domain = Domain::new(size);
        let polynomial = vec![black_box(Scalar::random(&mut rand::thread_rng())); size];
        let roots = domain.roots.clone();
        let points = roots
            .into_iter()
            .zip(polynomial.clone().into_iter())
            .collect::<Vec<_>>();

        c.bench_function(&format!("lagrange_interpolate of size {}", size), |b| {
            b.iter(|| lagrange_interpolate(&points))
        });
    }
}

criterion_group!(
    benches,
    bench_poly_eval,
    bench_fft,
    bench_lagrange_interpolation
);
criterion_main!(benches);
