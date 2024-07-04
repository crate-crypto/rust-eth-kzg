use commit_key::CommitKey;
use opening_key::OpeningKey;

pub mod commit_key;
pub mod fk20;
pub mod opening_key;
pub mod proof;

// Re-export the polynomial crate
pub use polynomial;

// This is simply the trusted setup file from the ethereum ceremony
#[deprecated(
    note = "FK20 tests can use any trusted setup, not just the Ethereum one. This will be removed in the future for a random insecure setup."
)]
pub mod eth_trusted_setup;

/// This is a placeholder method for creating the commit and opening keys for the ethereum
/// ceremony. This will be replaced with a method that reads the trusted setup file at a higher
/// level.
#[allow(deprecated)]
pub fn create_eth_commit_opening_keys() -> (CommitKey, OpeningKey) {
    let (g1s, g2s) = eth_trusted_setup::deserialize();
    // The setup needs 65 g1 elements for the opening key, in order
    // to commit to the remainder polynomial.
    let g1s_65 = g1s[0..65].to_vec();

    let ck = CommitKey::new(g1s);

    // A single proof will attest to the opening of 64 points.
    let multi_opening_size = 64;

    // We are making claims about a polynomial which has 4096 coefficients;
    let num_coefficients_in_polynomial = 4096;

    let vk = OpeningKey::new(g1s_65, g2s, multi_opening_size, num_coefficients_in_polynomial);
    (ck, vk)
}

// Taken and modified from: https://github.com/filecoin-project/ec-gpu/blob/bdde768d0613ae546524c5612e2ad576a646e036/ec-gpu-gen/src/fft_cpu.rs#L10C8-L10C18
// TODO: This could also be moved higher up in the stack. We only require cosets and to know their coset generator. How
// TODO that is generated, can be abstracted away.
pub fn reverse_bit_order<T>(a: &mut [T]) {
    fn bitreverse(mut n: u32, l: u32) -> u32 {
        let mut r = 0;
        for _ in 0..l {
            r = (r << 1) | (n & 1);
            n >>= 1;
        }
        r
    }

    fn log2(x: u32) -> u32 {
        assert!(x > 0 && x.is_power_of_two(), "x must be a power of two.");
        x.trailing_zeros()
    }

    let n = a.len() as u32;
    assert!(n.is_power_of_two(), "n must be a power of two");
    let log_n = log2(n);

    for k in 0..n {
        let rk = bitreverse(k, log_n);
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }
}
