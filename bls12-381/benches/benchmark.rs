use bls12_381::ff::Field;
use bls12_381::precomp_table::PrecomputedTable;
use bls12_381::{batch_inversion, G1Projective, Scalar};
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

pub fn bench_precomputed_scalar_mul(c: &mut Criterion) {
    use bls12_381::group::Group;

    let base = 9;
    let gen = G1Projective::generator();
    let precomp = PrecomputedTable::new(gen, base);
    let scalar = Scalar::random(&mut rand::thread_rng());
    c.bench_function(
        &format!("bls12-381 precomputed scalar_mul -  base {}", base),
        |b| b.iter(|| precomp.scalar_mul(scalar)),
    );
    c.bench_function(
        &format!("bls12-381 precomputed scalar_mul(batch add) - base: {}", base),
        |b| b.iter(|| precomp.scalar_mul_batch_addition(scalar)),
    );
}

criterion_group!(
    benches,
    bench_precomputed_scalar_mul,
    batch_inversion
);
criterion_main!(benches);
