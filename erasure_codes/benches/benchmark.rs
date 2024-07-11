use std::ops::Range;

use bls12_381::Scalar;
use crate_crypto_internal_peerdas_erasure_codes::{BlockErasures, ReedSolomon};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn bench_erasure_code_decoding_4096_8192(c: &mut Criterion) {
    const POLYNOMIAL_LEN: usize = 4096;

    let rs = ReedSolomon::new(POLYNOMIAL_LEN, 2);
    let extended_poly_len = rs.extended_polynomial_length();

    let mut encoded_polynomial = Vec::with_capacity(extended_poly_len);
    for i in 0..extended_poly_len {
        encoded_polynomial.push(black_box(-Scalar::from(i as u64 + 1)));
    }

    fn generate_unique_random_numbers(range: Range<usize>, n: usize) -> Vec<usize> {
        use rand::prelude::SliceRandom;
        let mut numbers: Vec<_> = range.into_iter().collect();
        numbers.shuffle(&mut rand::thread_rng());
        numbers.into_iter().take(n).collect()
    }

    let cell_size = 64;
    let num_cells = extended_poly_len / cell_size;

    let missing_cells = generate_unique_random_numbers(0..cell_size, num_cells / 2);

    // Zero out the values in the polynomial that correspond to the cell_id
    for cell in &missing_cells {
        for i in 0..cell_size {
            encoded_polynomial[*cell as usize * cell_size + i] = Scalar::from(0);
        }
    }

    c.bench_function(
        &format!(
            "computing decoding: EXT_SIZE {}, MISSING_CELLS {}",
            extended_poly_len,
            num_cells / 2
        ),
        |b| {
            b.iter(|| {
                rs.recover_polynomial_coefficient(
                    encoded_polynomial.clone(),
                    BlockErasures {
                        coset_size: cell_size,
                        cosets: missing_cells.clone(),
                    },
                )
            })
        },
    );
}

criterion_group!(benches, bench_erasure_code_decoding_4096_8192);
criterion_main!(benches);
