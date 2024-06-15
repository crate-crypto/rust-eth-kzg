use bls12_381::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};
use eip7594::prover::ProverContext;

/// This is here for reference, same as the above `bench_compute_proof_without_fk20`.
pub fn bench_compute_cells_and_kzg_proofs(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;

    let prover_context = ProverContext::with_num_threads(100);

    let polynomial_4096: Vec<_> = (0..POLYNOMIAL_LEN)
        .map(|i| -Scalar::from(i as u64))
        .collect();

    let blob: Vec<_> = polynomial_4096
        .into_iter()
        .flat_map(|scalar| scalar.to_bytes_be())
        .collect();
    let blob = &blob.try_into().unwrap();

    c.bench_function("computing cells_and_kzg_proofs", |b| {
        b.iter(|| prover_context.compute_cells_and_kzg_proofs(blob))
    });
}

criterion_group!(benches, bench_compute_cells_and_kzg_proofs);
criterion_main!(benches);
