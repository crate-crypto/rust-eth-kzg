use commit_key::CommitKey;
use opening_key::OpeningKey;

pub mod commit_key;
pub mod fk20;
pub mod opening_key;

// TODO: put this under a feature flag, so we can use it within benchmarks
// TODO: alternatively, put it under a test flag and never benchmark the naive implementations
pub mod naive;

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

    let vk = OpeningKey::new(
        g1s_65,
        g2s,
        multi_opening_size,
        num_coefficients_in_polynomial,
    );
    (ck, vk)
}

// Taken and modified from: https://github.com/filecoin-project/ec-gpu/blob/bdde768d0613ae546524c5612e2ad576a646e036/ec-gpu-gen/src/fft_cpu.rs#L10C8-L10C18
// TODO: This could also be moved higher up in the stack. We only require cosets and to know their coset generator. How
// TODO that is generated, can be abstracted away.
// TODO: Put this into cosets module or polynomial
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

#[cfg(test)]
mod tests {
    use crate::polynomial::domain::Domain;
    use crate::{
        create_eth_commit_opening_keys, fk20::naive as fk20naive, naive as kzgnaive,
        reverse_bit_order,
    };
    use bls12_381::Scalar;

    // We can move this down into the fk20 module.
    // TODO: Currently we need a way to produce fake commit keys and opening keys
    #[test]
    fn test_consistency_between_naive_kzg_naive_fk20() {
        // Setup
        //
        let (ck, _) = create_eth_commit_opening_keys();

        const POLYNOMIAL_LEN: usize = 4096;
        let poly_domain = Domain::new(POLYNOMIAL_LEN);

        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);

        const COSET_SIZE: usize = 64;

        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);
        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots.chunks(COSET_SIZE).collect();

        const NUMBER_OF_PROOFS: usize = NUMBER_OF_POINTS_TO_EVALUATE / COSET_SIZE;
        let proof_domain = Domain::new(NUMBER_OF_PROOFS);
        let polynomial_lagrange: Vec<_> = (0..POLYNOMIAL_LEN)
            .map(|i| -Scalar::from(i as u64))
            .collect();

        let poly_coeff = poly_domain.ifft_scalars(polynomial_lagrange);

        // Compute FK20 the naive way
        let got_proofs =
            fk20naive::fk20_open_multi_point(&ck, &proof_domain, &poly_coeff, COSET_SIZE);
        let got_set_of_output_points =
            fk20naive::fk20_compute_evaluation_set(&poly_coeff, COSET_SIZE, domain_extended);

        for k in 0..got_proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            // Compute the opening proofs the naive way (without fk20)
            let expected_proof = kzgnaive::compute_multi_opening(&ck, &poly_coeff, input_points);
            let expected_quotient_comm = expected_proof.quotient_commitment;
            let expected_output_points = expected_proof.output_points;

            assert_eq!(expected_output_points, got_set_of_output_points[k]);
            assert_eq!(expected_quotient_comm, got_proofs[k]);
        }
    }
}
