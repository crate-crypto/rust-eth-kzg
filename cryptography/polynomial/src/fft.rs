use bls12_381::{ff::Field, G1Projective, Scalar};
use std::ops::{Add, Mul, Neg, Sub};

trait FFTElement:
    Sized
    + Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Scalar, Output = Self>
    + Neg<Output = Self>
{
}

impl FFTElement for Scalar {}

impl FFTElement for G1Projective {}

fn fft_inplace<T: FFTElement>(twiddle_factors: &[Scalar], a: &mut [T]) {
    let n = a.len();
    let log_n = log2_pow2(n);
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k as u32, log_n) as usize;
        if k < rk {
            a.swap(rk, k);
        }
    }

    let mut m = 1;
    for s in 0..log_n {
        let w_m = twiddle_factors[s as usize];
        for k in (0..n).step_by(2 * m) {
            let mut w = Scalar::ONE;
            for j in 0..m {
                let t = if w == Scalar::ONE {
                    a[k + j + m]
                } else if w == -Scalar::ONE {
                    -a[k + j + m]
                } else {
                    a[k + j + m] * w
                };
                let u = a[k + j];
                a[k + j] = u + t;
                a[k + j + m] = u - t;
                w *= w_m;
            }
        }
        m *= 2;
    }
}

pub(crate) fn fft_scalar_inplace(twiddle_factors: &[Scalar], a: &mut [Scalar]) {
    fft_inplace(twiddle_factors, a);
}

pub(crate) fn fft_g1_inplace(twiddle_factors: &[Scalar], a: &mut [G1Projective]) {
    fft_inplace(twiddle_factors, a);
}

fn bitreverse(mut n: u32, l: u32) -> u32 {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}
fn log2_pow2(n: usize) -> u32 {
    n.trailing_zeros()
}
pub(crate) fn precompute_twiddle_factors<F: Field>(omega: &F, n: usize) -> Vec<F> {
    let log_n = log2_pow2(n);
    (0..log_n)
        .map(|s| omega.pow(&[(n / (1 << (s + 1))) as u64]))
        .collect()
}
