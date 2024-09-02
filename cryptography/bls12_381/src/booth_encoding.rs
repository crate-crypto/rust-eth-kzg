use std::ops::Neg;

use blstrs::{G1Projective, Scalar};
use ff::PrimeField;

use crate::G1Point;
// TODO: Link to halo2 file + docs + comments
pub fn get_booth_index(window_index: usize, window_size: usize, el: &[u8]) -> i32 {
    // Booth encoding:
    // * step by `window` size
    // * slice by size of `window + 1``
    // * each window overlap by 1 bit
    // * append a zero bit to the least significant end
    // Indexing rule for example window size 3 where we slice by 4 bits:
    // `[0, +1, +1, +2, +2, +3, +3, +4, -4, -3, -3 -2, -2, -1, -1, 0]``
    // So we can reduce the bucket size without preprocessing scalars
    // and remembering them as in classic signed digit encoding

    let skip_bits = (window_index * window_size).saturating_sub(1);
    let skip_bytes = skip_bits / 8;

    // fill into a u32
    let mut v: [u8; 4] = [0; 4];
    for (dst, src) in v.iter_mut().zip(el.iter().skip(skip_bytes)) {
        *dst = *src
    }
    let mut tmp = u32::from_le_bytes(v);

    // pad with one 0 if slicing the least significant window
    if window_index == 0 {
        tmp <<= 1;
    }

    // remove further bits
    tmp >>= skip_bits - (skip_bytes * 8);
    // apply the booth window
    tmp &= (1 << (window_size + 1)) - 1;

    let sign = tmp & (1 << window_size) == 0;

    // div ceil by 2
    tmp = (tmp + 1) >> 1;

    // find the booth action index
    if sign {
        tmp as i32
    } else {
        ((!(tmp - 1) & ((1 << window_size) - 1)) as i32).neg()
    }
}

#[test]
fn smoke_scalar_mul() {
    use group::prime::PrimeCurveAffine;
    let gen = G1Point::generator();
    let s = -Scalar::ONE;

    let res = gen * s;

    let got = mul(&s, &gen, 4);

    assert_eq!(G1Point::from(res), got)
}

fn mul(scalar: &Scalar, point: &G1Point, window: usize) -> G1Point {
    let u = scalar.to_bytes_le();
    let n = Scalar::NUM_BITS as usize / window + 1;

    let table = (0..=1 << (window - 1))
        .map(|i| point * Scalar::from(i as u64))
        .collect::<Vec<_>>();

    let mut acc: G1Projective = G1Point::default().into();
    for i in (0..n).rev() {
        for _ in 0..window {
            acc = acc + acc;
        }

        let idx = get_booth_index(i as usize, window, u.as_ref());

        if idx.is_negative() {
            acc += table[idx.unsigned_abs() as usize].neg();
        }
        if idx.is_positive() {
            acc += table[idx.unsigned_abs() as usize];
        }
    }

    acc.into()
}
