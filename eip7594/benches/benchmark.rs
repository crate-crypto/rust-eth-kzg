use bls12_381::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};
use kzg_multi_open::{
    create_eth_commit_opening_keys, fk20::naive, polynomial::domain::Domain, reverse_bit_order,
};

/// This is here for reference, same as the above `bench_compute_proof_without_fk20`.
pub fn bench_compute_proof_with_naive_fk20(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;

    let mut polynomial_4096: Vec<_> = (0..POLYNOMIAL_LEN)
        .map(|i| -Scalar::from(i as u64))
        .collect();
    reverse_bit_order(&mut polynomial_4096);
    let domain = Domain::new(POLYNOMIAL_LEN);
    let polynomial_4096 = domain.ifft_scalars(polynomial_4096);

    let (ck, _) = create_eth_commit_opening_keys();
    const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

    const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
    let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
    let mut domain_extended_roots = domain_extended.roots.clone();
    reverse_bit_order(&mut domain_extended_roots);

    let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
        .chunks(NUMBER_OF_POINTS_PER_PROOF)
        .collect();
    let proof_domain = Domain::new(chunked_bit_reversed_roots.len());

    c.bench_function(
        &format!(
            "computing proofs. POLY_SIZE {}, NUM_INPUT_POINTS {}, NUM_PROOFS {}",
            POLYNOMIAL_LEN,
            NUMBER_OF_POINTS_PER_PROOF,
            chunked_bit_reversed_roots.len()
        ),
        |b| {
            b.iter(|| {
                naive::fk20_open_multi_point(
                    &ck,
                    &proof_domain,
                    &domain_extended,
                    &polynomial_4096,
                    NUMBER_OF_POINTS_PER_PROOF,
                )
            })
        },
    );
}

criterion_group!(
    benches,
    // bench_msm,
    // bench_compute_proof_without_fk20,
    bench_compute_proof_with_naive_fk20
);
criterion_main!(benches);
