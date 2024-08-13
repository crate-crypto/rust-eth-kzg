use bls12_381::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};
use rust_eth_kzg::{
    constants::{BYTES_PER_BLOB, CELLS_PER_EXT_BLOB},
    Bytes48Ref, Cell, CellIndex, CellRef, DASContext, KZGCommitment, KZGProof, RowIndex,
    TrustedSetup,
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
    let ctx = DASContext::default();
    let blob = dummy_blob();

    let commitment = ctx.blob_to_kzg_commitment(&blob).unwrap();
    (commitment, ctx.compute_cells_and_kzg_proofs(&blob).unwrap())
}

const THREAD_COUNTS: [usize; 5] = [1, 4, 8, 16, 32];

pub fn bench_compute_cells_and_kzg_proofs(c: &mut Criterion) {
    let trusted_setup = TrustedSetup::default();

    let blob = dummy_blob();

    for num_threads in THREAD_COUNTS {
        let ctx = DASContext::with_threads(&trusted_setup, num_threads);
        c.bench_function(
            &format!(
                "computing cells_and_kzg_proofs - NUM_THREADS: {}",
                num_threads
            ),
            |b| b.iter(|| ctx.compute_cells_and_kzg_proofs(&blob)),
        );
    }
}

pub fn bench_recover_cells_and_compute_kzg_proofs(c: &mut Criterion) {
    let trusted_setup = TrustedSetup::default();

    let (_, (cells, _)) = dummy_commitment_cells_and_proofs();
    let cell_indices: Vec<u64> = (0..cells.len()).map(|x| x as u64).collect();

    // Worse case is when half of the cells are missing
    let half_cell_indices = &cell_indices[..CELLS_PER_EXT_BLOB / 2];
    let half_cells = &cells[..CELLS_PER_EXT_BLOB / 2];
    let half_cells = half_cells
        .into_iter()
        .map(|cell| cell.as_ref())
        .collect::<Vec<_>>();

    for num_threads in THREAD_COUNTS {
        let ctx = DASContext::with_threads(&trusted_setup, num_threads);
        c.bench_function(
            &format!(
                "worse-case recover_cells_and_compute_proofs - NUM_THREADS: {}",
                num_threads
            ),
            |b| {
                b.iter(|| {
                    ctx.recover_cells_and_proofs(half_cell_indices.to_vec(), half_cells.to_vec())
                })
            },
        );
    }
}

pub fn bench_verify_cell_kzg_proof_batch(c: &mut Criterion) {
    let trusted_setup = TrustedSetup::default();

    let (commitment, (cells, proofs)) = dummy_commitment_cells_and_proofs();

    let commitments = vec![&commitment; CELLS_PER_EXT_BLOB];
    let cell_indices: Vec<CellIndex> = (0..CELLS_PER_EXT_BLOB).map(|x| x as CellIndex).collect();
    let cell_refs: Vec<CellRef> = cells.iter().map(|cell| cell.as_ref()).collect();
    let proof_refs: Vec<Bytes48Ref> = proofs.iter().map(|proof| proof).collect();

    for num_threads in THREAD_COUNTS {
        let ctx = DASContext::with_threads(&trusted_setup, num_threads);
        c.bench_function(
            &format!("verify_cell_kzg_proof_batch - NUM_THREADS: {}", num_threads),
            |b| {
                b.iter(|| {
                    ctx.verify_cell_kzg_proof_batch(
                        commitments.clone(),
                        cell_indices.clone(),
                        cell_refs.clone(),
                        proof_refs.clone(),
                    )
                })
            },
        );
    }
}

pub fn bench_init_context(c: &mut Criterion) {
    const NUM_THREADS: usize = 1;
    c.bench_function(&format!("Initialize context"), |b| {
        b.iter(|| {
            let trusted_setup = TrustedSetup::default();
            DASContext::with_threads(&trusted_setup, NUM_THREADS)
        })
    });
}

criterion_group!(
    benches,
    bench_init_context,
    bench_compute_cells_and_kzg_proofs,
    bench_recover_cells_and_compute_kzg_proofs,
    bench_verify_cell_kzg_proof_batch
);
criterion_main!(benches);
