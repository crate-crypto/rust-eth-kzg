use bls12_381::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};
use eip7594::{
    constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB},
    prover::ProverContext,
    trusted_setup, Cell, KZGCommitment, KZGProof, VerifierContext,
};

const POLYNOMIAL_LEN: usize = 4096;

fn dummy_blob() -> [u8; BYTES_PER_BLOB] {
    let polynomial: Vec<_> = (0..POLYNOMIAL_LEN)
        .map(|i| -Scalar::from(i as u64))
        .collect();
    let blob: Vec<_> = polynomial
        .into_iter()
        .flat_map(|scalar| scalar.to_bytes_be())
        .collect();
    blob.try_into().unwrap()
}

fn dummy_commitment_cells_and_proofs() -> (
    KZGCommitment,
    ([Cell; CELLS_PER_EXT_BLOB], [KZGProof; CELLS_PER_EXT_BLOB]),
) {
    let ctx = ProverContext::default();
    let blob = dummy_blob();

    let commitment = ctx.blob_to_kzg_commitment(&blob).unwrap();
    (commitment, ctx.compute_cells_and_kzg_proofs(&blob).unwrap())
}

const THREAD_COUNTS: [usize; 5] = [1, 4, 8, 16, 32];

pub fn bench_compute_cells_and_kzg_proofs(c: &mut Criterion) {
    let trusted_setup = trusted_setup::TrustedSetup::default();

    let blob = dummy_blob();

    for num_threads in THREAD_COUNTS {
        let prover_context = ProverContext::with_num_threads(&trusted_setup, num_threads);
        c.bench_function(
            &format!(
                "computing cells_and_kzg_proofs - NUM_THREADS: {}",
                num_threads
            ),
            |b| b.iter(|| prover_context.compute_cells_and_kzg_proofs(&blob)),
        );
    }
}

pub fn bench_recover_cells_and_compute_kzg_proofs(c: &mut Criterion) {
    let trusted_setup = trusted_setup::TrustedSetup::default();

    let (_, (cells, proofs)) = dummy_commitment_cells_and_proofs();
    let cell_ids: Vec<u64> = (0..cells.len()).map(|x| x as u64).collect();

    // Worse case is when half of the cells are missing
    let half_cell_ids = &cell_ids[..CELLS_PER_EXT_BLOB / 2];
    let half_cells = &cells[..CELLS_PER_EXT_BLOB / 2];
    let half_cells = half_cells
        .into_iter()
        .map(|cell| cell.as_ref())
        .collect::<Vec<_>>();
    let half_proofs = &proofs[..CELLS_PER_EXT_BLOB / 2];
    let half_proofs = half_proofs.into_iter().collect::<Vec<_>>();

    for num_threads in THREAD_COUNTS {
        let prover_context = ProverContext::with_num_threads(&trusted_setup, num_threads);
        c.bench_function(
            &format!(
                "worse-case recover_cells_and_compute_proofs - NUM_THREADS: {}",
                num_threads
            ),
            |b| {
                b.iter(|| {
                    prover_context.recover_cells_and_proofs(
                        half_cell_ids.to_vec(),
                        half_cells.to_vec(),
                        half_proofs.to_vec(),
                    )
                })
            },
        );
    }
}

pub fn bench_verify_cell_kzg_proofs(c: &mut Criterion) {
    let trusted_setup = trusted_setup::TrustedSetup::default();

    let (commitment, (cells, proofs)) = dummy_commitment_cells_and_proofs();

    for num_threads in THREAD_COUNTS {
        let verifier_context = VerifierContext::with_num_threads(&trusted_setup, num_threads);
        c.bench_function(
            &format!("verify_cell_kzg_proof - NUM_THREADS: {}", num_threads),
            |b| {
                b.iter(|| {
                    verifier_context.verify_cell_kzg_proof(&commitment, 0, &cells[0], &proofs[0])
                })
            },
        );
    }
}

criterion_group!(
    benches,
    bench_compute_cells_and_kzg_proofs,
    bench_recover_cells_and_compute_kzg_proofs,
    bench_verify_cell_kzg_proofs
);
criterion_main!(benches);
