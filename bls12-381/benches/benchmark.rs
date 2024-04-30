use bls12_381::ff::Field;
use bls12_381::{batch_inversion, Scalar};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn batch_inversion(c: &mut Criterion) {
    const NUM_ELEMENTS: usize = 8192;

    c.bench_function(
        &format!("bls12-381 batch_inversion size: {}", NUM_ELEMENTS),
        |b| {
            b.iter(|| {
                let mut elements =
                    vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_ELEMENTS];
                batch_inversion::batch_inverse(&mut elements);
            })
        },
    );

    c.bench_function(
        &format!(
            "bls12-381 batch_inversion_zero_check size: {}",
            NUM_ELEMENTS
        ),
        |b| {
            b.iter(|| {
                let mut elements =
                    vec![black_box(Scalar::random(&mut rand::thread_rng())); NUM_ELEMENTS];
                batch_inversion::batch_inverse_check_for_zero(&mut elements);
            })
        },
    );
}

criterion_group!(benches, batch_inversion);
criterion_main!(benches);
