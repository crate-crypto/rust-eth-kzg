use commit_key::CommitKey;
use opening_key::OpeningKey;

pub mod commit_key;
pub mod lincomb;
pub mod opening_key;
pub mod proof;

// TODO: Remove this once we have imported the consensus specs test vectors
mod consensus_specs_fixed_test_vector;

// TODO: We can replace this with a file being embedded in the future.
// This is simply the trusted setup file from the ethereum ceremony
pub mod eth_trusted_setup;

/// This is a placeholder method for creating the commit and opening keys for the ethereum
/// ceremony. This will be replaced with a method that reads the trusted setup file at a higher
/// level.
pub fn create_eth_commit_opening_keys() -> (CommitKey, OpeningKey) {
    let (g1s, g2s) = eth_trusted_setup::deserialize();
    let generator = g1s[0];

    let ck = CommitKey::new(g1s);
    let vk = OpeningKey::new(generator, g2s);
    (ck, vk)
}

#[cfg(test)]
mod tests {
    use polynomial::domain::Domain;

    use super::CommitKey;
    use crate::{
        consensus_specs_fixed_test_vector::{eth_commitment, eth_polynomial},
        eth_trusted_setup,
        opening_key::OpeningKey,
        reverse_bit_order,
    };

    #[test]
    fn eth_trusted_setup_deserializes() {
        // Just test that the trusted setup can be loaded/deserialized
        let (g1s, g2s) = eth_trusted_setup::deserialize();
        let generator = g1s[0];

        let _ck = CommitKey::new(g1s);
        let _vk = OpeningKey::new(generator, g2s);
    }

    #[test]
    fn test_polynomial_commitment_matches() {
        // Setup
        let (ck, _) = super::create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        let domain = Domain::new(POLYNOMIAL_LEN);
        let mut ck_lagrange = ck.into_lagrange(&domain);
        // We need to apply the reverse bit order permutation to the g1s
        // in order for it match the specs.
        // TODO: Apply it to the polynomial instead (time to do it is about 26 microseconds)
        reverse_bit_order(&mut ck_lagrange.g1s);

        let polynomial = eth_polynomial();
        let expected_commitment = eth_commitment();
        let got_commitment = ck_lagrange.commit_g1(&polynomial);

        assert_eq!(got_commitment, expected_commitment);
    }
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
