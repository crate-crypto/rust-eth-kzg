use std::{
    iter::successors,
    ops::{Add, Mul, Neg, Sub},
};

use bls12_381::{ff::Field, group::Group, G1Projective, Scalar};
use maybe_rayon::prelude::*;

trait FFTElement:
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
fn fft_inplace<T: FFTElement>(omegas: &[Scalar], twiddle_factors_bo: &[Scalar], values: &mut [T]) {
    let log_n = log2_pow2(values.len()) as usize;
    let mid = log_n.div_ceil(2);

    // The first half looks like a normal DIT.
    bitreverse_slice(values);
    first_half(values, mid, omegas);

    // For the second half, we flip the DIT, working in bit-reversed order,
    // so the max block size will be at most `1 << (log_n - mid)`.
    bitreverse_slice(values);
    second_half(values, mid, twiddle_factors_bo);

    bitreverse_slice(values);
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

pub(crate) fn fft_scalar_inplace(
    twiddle_factors: &[Scalar],
    twiddle_factors_bo: &[Scalar],
    a: &mut [Scalar],
) {
    fft_inplace(twiddle_factors, twiddle_factors_bo, a);
}

pub(crate) fn fft_g1_inplace(
    twiddle_factors: &[Scalar],
    twiddle_factors_bo: &[Scalar],
    a: &mut [G1Projective],
) {
    fft_inplace(twiddle_factors, twiddle_factors_bo, a);
}

fn bitreverse(mut n: u32, l: u32) -> u32 {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

fn bitreverse_slice<T>(a: &mut [T]) {
    if a.is_empty() {
        return;
    }

    let n = a.len();
    let log_n = log2_pow2(n);
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k as u32, log_n) as usize;
        if k < rk {
            a.swap(rk, k);
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
    bitreverse_slice(&mut twiddle_factors);
    twiddle_factors
}
