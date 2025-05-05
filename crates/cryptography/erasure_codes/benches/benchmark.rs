use std::ops::Range;

use bls12_381::Scalar;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ekzg_erasure_codes::{BlockErasureIndices, ReedSolomon};

pub fn bench_erasure_code_decoding_4096_8192(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;

    let block_size = 128;
    let expansion_factor = 2;
    let rs = ReedSolomon::new(POLYNOMIAL_LEN, expansion_factor, block_size);
    let extended_poly_len = rs.codeword_length();

    let mut encoded_polynomial = Vec::with_capacity(extended_poly_len);
    for i in 0..extended_poly_len {
        encoded_polynomial.push(black_box(-Scalar::from(i as u64 + 1)));
    }

    let num_blocks = extended_poly_len / block_size;

    let missing_blocks = generate_unique_random_numbers(0..block_size, num_blocks / 2);
    c.bench_function(
        &format!(
            "computing decoding: EXT_SIZE {}, MISSING_CELLS {}",
            extended_poly_len,
            num_blocks / 2
        ),
        |b| {
            b.iter(|| {
                rs.recover_polynomial_coefficient(
                    encoded_polynomial.clone(),
                    BlockErasureIndices(missing_blocks.clone()),
                )
            });
        },
    );
}

fn generate_unique_random_numbers(range: Range<usize>, n: usize) -> Vec<usize> {
    use rand::prelude::SliceRandom;
    let mut numbers: Vec<_> = range.into_iter().collect();
    numbers.shuffle(&mut rand::thread_rng());
    numbers.into_iter().take(n).collect()
}

criterion_group!(benches, bench_erasure_code_decoding_4096_8192);
criterion_main!(benches);
