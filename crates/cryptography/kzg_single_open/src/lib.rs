mod errors;
pub use errors::VerifierError;

pub mod prover;
pub mod verifier;

fn bitreverse(mut n: u32, l: u32) -> u32 {
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
