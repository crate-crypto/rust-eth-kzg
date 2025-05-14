use std::{
    iter::successors,
    ops::{Add, Mul, Neg, Sub},
};

use bls12_381::{ff::Field, group::Group, G1Projective, Scalar};
use maybe_rayon::prelude::*;

pub(crate) trait FFTElement:
    Sized
    + Send
    + Copy
    + PartialEq
    + Eq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Scalar, Output = Self>
    + Neg<Output = Self>
{
    fn zero() -> Self;
}

impl FFTElement for Scalar {
    fn zero() -> Self {
        Self::ZERO
    }
}

impl FFTElement for G1Projective {
    fn zero() -> Self {
        Self::identity()
    }
}

// Taken and modified from https://github.com/Plonky3/Plonky3/blob/a374139/dft/src/radix_2_dit_parallel.rs#L106.
pub(crate) fn fft_inplace<T: FFTElement>(
    omegas: &[Scalar],
    twiddle_factors_bo: &[Scalar],
    values: &mut [T],
) {
    let log_n = log2_pow2(values.len()) as usize;
    let mid = log_n.div_ceil(2);

    // The first half looks like a normal DIT.
    reverse_bit_order(values);
    first_half(values, mid, omegas);

    // For the second half, we flip the DIT, working in bit-reversed order,
    // so the max block size will be at most `1 << (log_n - mid)`.
    reverse_bit_order(values);
    second_half(values, mid, twiddle_factors_bo);

    reverse_bit_order(values);
}

#[allow(clippy::needless_range_loop)]
fn first_half<T: FFTElement>(values: &mut [T], mid: usize, omegas: &[Scalar]) {
    values.maybe_par_chunks_mut(1 << mid).for_each(|chunk| {
        let mut backwards = false;
        for layer in 0..mid {
            let half_block_size = 1 << layer;
            let omega = omegas[layer];
            dit_layer(chunk, half_block_size, omega, backwards);
            backwards = !backwards;
        }
    });
}

#[inline]
fn dit_layer<T: FFTElement>(
    blocks: &mut [T],
    half_block_size: usize,
    omega: Scalar,
    backwards: bool,
) {
    let process_block = |block: &mut [T]| {
        let (a, b) = block.split_at_mut(half_block_size);
        let mut twiddle = Scalar::ONE;
        a.iter_mut().zip(b).for_each(|(a, b)| {
            dit(a, b, twiddle);
            twiddle *= omega;
        });
    };

    let blocks = blocks.chunks_mut(2 * half_block_size);
    if backwards {
        blocks.rev().for_each(process_block);
    } else {
        blocks.for_each(process_block);
    }
}

fn second_half<T: FFTElement>(values: &mut [T], mid: usize, twiddles_bo: &[Scalar]) {
    let log_n = log2_pow2(values.len()) as usize;
    values
        .maybe_par_chunks_mut(1 << (log_n - mid))
        .enumerate()
        .for_each(|(chunk_idx, chunk)| {
            let mut backwards = false;
            for layer in mid..log_n {
                let half_block_size = 1 << (log_n - 1 - layer);
                let twiddles_bo = &twiddles_bo[chunk_idx << (layer - mid)..];
                dit_layer_bo(chunk, half_block_size, twiddles_bo, backwards);
                backwards = !backwards;
            }
        });
}

#[inline]
fn dit_layer_bo<T: FFTElement>(
    blocks: &mut [T],
    half_block_size: usize,
    twiddles_bo: &[Scalar],
    backwards: bool,
) {
    let process_block = |block: &mut [T], twiddle| {
        let (a, b) = block.split_at_mut(half_block_size);
        a.iter_mut().zip(b).for_each(|(a, b)| dit(a, b, twiddle));
    };

    let blocks_and_twiddles = blocks.chunks_mut(2 * half_block_size).zip(twiddles_bo);
    if backwards {
        blocks_and_twiddles
            .rev()
            .for_each(|(block, twiddle)| process_block(block, *twiddle));
    } else {
        blocks_and_twiddles.for_each(|(block, twiddle)| process_block(block, *twiddle));
    }
}

#[inline]
fn dit<T: FFTElement>(a: &mut T, b: &mut T, twiddle: Scalar) {
    let t = if twiddle == Scalar::ONE {
        *b
    } else if twiddle == -Scalar::ONE {
        -*b
    } else if *b == FFTElement::zero() {
        FFTElement::zero()
    } else {
        *b * twiddle
    };
    *b = *a;
    *a = *a + t;
    *b = *b - t;
}

/// Reverses the least significant `bits` of the given number `n`.
///
/// `n` - The input number whose bits are to be reversed.
/// `bits` - The number of least significant bits to reverse.
///
/// Returns a new `usize` with the specified number of bits reversed.
///
/// Taken and modified from: https://github.com/Plonky3/Plonky3/blob/a374139abead1008f84a439e95bb495e81ea4be5/util/src/lib.rs#L67-L76
pub(crate) const fn reverse_bits(n: usize, bits: u32) -> usize {
    // NB: The only reason we need overflowing_shr() here as opposed
    // to plain '>>' is to accommodate the case n == num_bits == 0,
    // which would become `0 >> 64`. Rust thinks that any shift of 64
    // bits causes overflow, even when the argument is zero.
    n.reverse_bits().overflowing_shr(usize::BITS - bits).0
}

/// In-place bit-reversal permutation of a slice.
///
/// Reorders the elements of the slice `a` in-place by reversing the binary representation of their indices.
///
/// For example, if `a.len() == 8` (i.e., `log2(n) = 3`), the index permutation would be:
///
/// ```text
/// Index  Binary   Reversed   Swapped With
/// -----  -------  ---------  -------------
///   0     000       000           -
///   1     001       100           4
///   2     010       010           -
///   3     011       110           6
///   4     100       001           1
///   5     101       101           -
///   6     110       011           3
///   7     111       111           -
/// ```
///
/// This transformation is its own inverse, so applying it twice restores the original order.
///
/// # Panics
/// Panics if the slice length is not a power of two.
///
/// # Arguments
/// * `a` — A mutable slice of data to reorder in-place.
///
/// Taken and modified from: https://github.com/filecoin-project/ec-gpu/blob/bdde768d0613ae546524c5612e2ad576a646e036/ec-gpu-gen/src/fft_cpu.rs#L10C8-L10C18
pub fn reverse_bit_order<T>(a: &mut [T]) {
    // If we are looking for optimizations, it would be nice to have a look at the following:
    // https://github.com/Plonky3/Plonky3/blob/a374139abead1008f84a439e95bb495e81ea4be5/matrix/src/util.rs#L36-L57

    // If the slice is empty, there is nothing to do
    //
    // WARNING: We should not go further if the slice is empty because it will panic:
    // The len is not a power of two.
    if a.is_empty() {
        return;
    }

    let n = a.len();

    // Ensure the length is a power of two for valid bit-reversal indexing
    assert!(n.is_power_of_two(), "n must be a power of two");

    // Compute the number of bits needed to index `n` elements (i.e., log2(n))
    let log_n = log2_pow2(n);

    // Iterate through each index and swap with its bit-reversed counterpart
    for k in 0..n {
        // Compute bit-reversed index of k using only log_n bits
        let rk = reverse_bits(k, log_n);

        // Swap only if k < rk to avoid double-swapping
        if k < rk {
            a.swap(rk as usize, k);
        }
    }
}

/// We assume that `n` is a power of 2.
const fn log2_pow2(n: usize) -> u32 {
    n.trailing_zeros()
}

/// Returns `[ω_{2}, ω_{4}, ..., ω_{n}]` given input `omega` = `ω_{n}`.
pub(crate) fn precompute_omegas<F: Field>(omega: &F, n: usize) -> Vec<F> {
    let log_n = log2_pow2(n);
    (0..log_n)
        .map(|s| omega.pow([(n / (1 << (s + 1))) as u64]))
        .collect()
}

/// Returns `[ω^0, ω^1, ..., ω^{n/2-1}]` in bit-reversed order.
pub(crate) fn precompute_twiddle_factors_bo<F: Field>(omega: &F, n: usize) -> Vec<F> {
    let mut twiddle_factors = successors(Some(F::ONE), |twiddle| Some(*twiddle * omega))
        .take(n / 2)
        .collect::<Vec<_>>();
    reverse_bit_order(&mut twiddle_factors);
    twiddle_factors
}

#[cfg(test)]
mod tests {
    use rand::{prelude::SliceRandom, thread_rng};

    use super::*;

    #[test]
    fn bit_reverse_fuzz() {
        fn naive_bit_reverse(n: u32, l: u32) -> u32 {
            assert!(l.is_power_of_two());
            let num_bits = l.trailing_zeros();
            n.reverse_bits() >> (32 - num_bits)
        }

        for i in 0..10 {
            for k in (1..31).map(|exponent| 2u32.pow(exponent)) {
                let expected = naive_bit_reverse(i, k);
                let got = reverse_bits(i as usize, log2_pow2(k as usize)) as u32;
                assert_eq!(expected, got);
            }
        }
    }

    #[test]
    fn test_reverse_bits_small() {
        assert_eq!(reverse_bits(0b000, 3), 0b000);
        assert_eq!(reverse_bits(0b001, 3), 0b100);
        assert_eq!(reverse_bits(0b010, 3), 0b010);
        assert_eq!(reverse_bits(0b011, 3), 0b110);
        assert_eq!(reverse_bits(0b100, 3), 0b001);
        assert_eq!(reverse_bits(0b101, 3), 0b101);
        assert_eq!(reverse_bits(0b110, 3), 0b011);
        assert_eq!(reverse_bits(0b111, 3), 0b111);
    }

    #[test]
    fn test_reverse_bits_varied_width() {
        // 4-bit reversal
        assert_eq!(reverse_bits(0b0001, 4), 0b1000);
        assert_eq!(reverse_bits(0b1010, 4), 0b0101);
        // 5-bit reversal
        assert_eq!(reverse_bits(0b10000, 5), 0b00001);
        assert_eq!(reverse_bits(0b11001, 5), 0b10011);
    }

    #[test]
    fn test_reverse_bits_zero_zero() {
        // This simulates the edge case: n == 0, bits == 0
        // A naive implementation using .reverse_bits() >> (usize::BITS - bits)
        // would panic here due to an invalid shift (e.g., 0 >> 64).
        //
        // Our implementation should safely return 0.
        let result = reverse_bits(0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_reverse_bits_partial_width() {
        // Only reverse the least significant 4 bits of the number 0b0110_1011
        let input = 0b0110_1011usize;
        let bits = 4;

        // Expected: reverse of 0b1001 (LSB 4 bits) => 0b1011 -> 0b1101
        let expected = 0b1101;

        let result = reverse_bits(input, bits);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_reverse_bit_order_len_1() {
        let mut arr = [42];
        reverse_bit_order(&mut arr);
        assert_eq!(arr, [42]); // Nothing changes
    }

    #[test]
    fn test_reverse_bit_order_len_2() {
        let mut arr = [1, 2];
        reverse_bit_order(&mut arr);
        assert_eq!(arr, [1, 2]);
    }

    #[test]
    fn test_reverse_bit_order_len_4() {
        let mut arr = [10, 20, 30, 40];
        reverse_bit_order(&mut arr);
        // Indices 0..4 → 2-bit reversal:
        // 00 → 00 (0)
        // 01 → 10 (2)
        // 10 → 01 (1)
        // 11 → 11 (3)
        assert_eq!(arr, [10, 30, 20, 40]); // only 1↔2 swapped
    }

    #[test]
    fn test_reverse_bit_order_roundtrip() {
        for log_n in 1..=10 {
            let n = 1 << log_n;
            let mut rng = thread_rng();

            // Generate shuffled input of known size
            let mut original: Vec<u32> = (0..n).collect();
            original.shuffle(&mut rng);

            // Clone and apply reverse_bit_order twice
            let mut reversed = original.clone();
            reverse_bit_order(&mut reversed);
            reverse_bit_order(&mut reversed);

            // Check we returned to original
            assert_eq!(
                reversed, original,
                "Mismatch after double reversal for len={n}"
            );
        }
    }

    #[test]
    fn test_reverse_bit_order_empty_slice() {
        let mut arr: [u32; 0] = [];
        reverse_bit_order(&mut arr);
        assert_eq!(arr, []); // Should remain unchanged and not panic
    }
}
