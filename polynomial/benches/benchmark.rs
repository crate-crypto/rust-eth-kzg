use bls12_381::ff::Field;
use bls12_381::Scalar;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polynomial::monomial::poly_eval;

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

criterion_group!(benches, bench_poly_eval);
criterion_main!(benches);
