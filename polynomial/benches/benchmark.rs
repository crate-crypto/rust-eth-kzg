use bls12_381::Scalar;
use bls12_381::ff::Field;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polynomial::monomial::{horners_eval, poly_eval};

pub fn bench_poly_eval(c: &mut Criterion) {
    
    const NUM_ELEMENTS : usize = 2usize.pow(16);
    let polynomial = vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_ELEMENTS];
    let value = Scalar::random(&mut rand::thread_rng());

    c.bench_function("naive poly_eval", |b| {
        b.iter(|| {
            poly_eval(&polynomial, &value);
        })
    });
    c.bench_function("horner eval", |b| {
        b.iter(|| {
            horners_eval(&polynomial, &value);
        })
    });
}

criterion_group!(benches, bench_poly_eval);
criterion_main!(benches);
