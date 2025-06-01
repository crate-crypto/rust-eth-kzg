mod errors;
use bls12_381::Scalar;
pub use errors::VerifierError;

pub mod prover;
pub mod verifier;

pub fn bitreverse(mut n: u32, l: u32) -> u32 {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

pub fn bitreverse_slice<T>(a: &mut [T]) {
    if a.is_empty() {
        return;
    }

    let n = a.len();
    let log_n = n.ilog2();
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k as u32, log_n) as usize;
        if k < rk {
            a.swap(rk, k);
        }
    }
}

/// Divides poly by X-Z using ruffini's rule, and returns quotient and reminder.
pub fn divide_by_linear(poly: &[Scalar], z: Scalar) -> (Vec<Scalar>, Scalar) {
    let mut quotient: Vec<Scalar> = Vec::with_capacity(poly.len());
    let mut k = Scalar::from(0u64);

    for coeff in poly.iter().rev() {
        let t = *coeff + k;
        quotient.push(t);
        k = z * t;
    }

    // Pop off the remainder term
    let remainder = quotient.pop().expect("!quotient.is_empty()");

    // Reverse the results as monomial form stores coefficients starting with lowest degree
    quotient.reverse();

    (quotient, remainder)
}
