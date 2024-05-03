use commit_key::CommitKey;
use opening_key::OpeningKey;

pub mod commit_key;
pub mod fk20;
pub mod lincomb;
pub mod opening_key;
pub mod proof;

// TODO: Remove this once we have imported the consensus specs test vectors
pub mod consensus_specs_fixed_test_vector;

// TODO: We can replace this with a file being embedded in the future.
// This is simply the trusted setup file from the ethereum ceremony
pub mod eth_trusted_setup;

/// This is a placeholder method for creating the commit and opening keys for the ethereum
/// ceremony. This will be replaced with a method that reads the trusted setup file at a higher
/// level.
pub fn create_eth_commit_opening_keys() -> (CommitKey, OpeningKey) {
    let (g1s, g2s) = eth_trusted_setup::deserialize();
    // The setup needs 65 g1 elements for the opening key, in order
    // to commit to the remainder polynomial.
    let g1s_65 = g1s[0..65].to_vec();

    let ck = CommitKey::new(g1s);

    let vk = OpeningKey::new(g1s_65, g2s);
    (ck, vk)
}

#[cfg(test)]
mod eth_tests {
    use bls12_381::G1Point;
    use polynomial::domain::Domain;

    use super::CommitKey;
    use crate::{
        consensus_specs_fixed_test_vector::{
            eth_cells, eth_commitment, eth_polynomial, eth_proofs,
        },
        eth_trusted_setup,
        opening_key::OpeningKey,
        proof::{compute_multi_opening_naive, verify_multi_opening_naive},
        reverse_bit_order,
    };

    #[test]
    fn eth_trusted_setup_deserializes() {
        // Just test that the trusted setup can be loaded/deserialized
        let (g1s, g2s) = eth_trusted_setup::deserialize();
        let g1s_65 = g1s[0..65].to_vec();
        let _ck = CommitKey::new(g1s);
        let _vk = OpeningKey::new(g1s_65, g2s);
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

    #[test]
    fn test_proofs_verify() {
        // Setup
        let (_, vk) = super::create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;

        const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
            .chunks(NUMBER_OF_POINTS_PER_PROOF)
            .collect();

        let commitment: G1Point = eth_commitment().into();
        let proofs = eth_proofs();
        let cells = eth_cells();

        for k in 0..proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            let proof: G1Point = proofs[k].into();
            let coset_eval = &cells[k];

            assert!(verify_multi_opening_naive(
                &vk,
                commitment,
                proof,
                &input_points,
                coset_eval
            ));
        }
    }

    #[test]
    fn test_computing_proofs() {
        // Setup
        let (ck, _) = super::create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;
        let domain = Domain::new(POLYNOMIAL_LEN);

        const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
            .chunks(NUMBER_OF_POINTS_PER_PROOF)
            .collect();

        let mut polynomial = eth_polynomial();
        // Polynomial really corresponds to the evaluation form, so we need
        // to apply bit reverse order and then IFFT to get the coefficients
        reverse_bit_order(&mut polynomial);
        let poly_coeff = domain.ifft_scalars(polynomial);

        let proofs = eth_proofs();
        let cells = eth_cells();
        for k in 0..proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            let proof: G1Point = proofs[k].clone().into();
            let (quotient_comm, output_points) =
                compute_multi_opening_naive(&ck, &poly_coeff, input_points);

            assert_eq!(cells[k], output_points);
            assert_eq!(proof, quotient_comm);
        }
    }

    // This test does not need to be moved to a higher level.
    // It is here as its easier to test against the naive implementation here.
    #[test]
    fn test_consistency_between_naive_kzg_naive_fk20() {
        // Setup
        let (ck, _) = super::create_eth_commit_opening_keys();
        const POLYNOMIAL_LEN: usize = 4096;
        const NUMBER_OF_POINTS_TO_EVALUATE: usize = 2 * POLYNOMIAL_LEN;
        let domain = Domain::new(POLYNOMIAL_LEN);

        const NUMBER_OF_POINTS_PER_PROOF: usize = 64;
        let domain_extended = Domain::new(NUMBER_OF_POINTS_TO_EVALUATE);
        let mut domain_extended_roots = domain_extended.roots.clone();
        reverse_bit_order(&mut domain_extended_roots);

        let chunked_bit_reversed_roots: Vec<_> = domain_extended_roots
            .chunks(NUMBER_OF_POINTS_PER_PROOF)
            .collect();

        const NUMBER_OF_PROOFS: usize = NUMBER_OF_POINTS_TO_EVALUATE / NUMBER_OF_POINTS_PER_PROOF;
        let proof_domain = Domain::new(NUMBER_OF_PROOFS);
        let mut polynomial = eth_polynomial();
        // Polynomial really corresponds to the evaluation form, so we need
        // to apply bit reverse order and then IFFT to get the coefficients
        reverse_bit_order(&mut polynomial);
        let poly_coeff = domain.ifft_scalars(polynomial);

        let (got_proofs, got_set_of_output_points) = crate::fk20::naive::fk20_open_multi_point(
            &ck,
            &proof_domain,
            &domain_extended,
            &poly_coeff,
            NUMBER_OF_POINTS_PER_PROOF,
        );

        for k in 0..got_proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            let (expected_quotient_comm, expected_output_points) =
                compute_multi_opening_naive(&ck, &poly_coeff, input_points);
            assert_eq!(expected_output_points, got_set_of_output_points[k]);
            assert_eq!(expected_quotient_comm, got_proofs[k]);
        }
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
