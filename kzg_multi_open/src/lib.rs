use commit_key::CommitKey;
use opening_key::OpeningKey;

pub mod commit_key;
pub mod fk20;
pub mod opening_key;

#[cfg(test)]
pub(crate) mod naive;

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

#[cfg(test)]
mod tests {
    use crate::fk20::cosets::reverse_bit_order;
    use crate::polynomial::domain::Domain;
    use crate::{create_eth_commit_opening_keys, fk20::naive as fk20naive, naive as kzgnaive};
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
        let (got_proofs, got_set_of_output_points) = fk20naive::fk20_open_multi_point(
            &ck,
            &proof_domain,
            &poly_coeff,
            COSET_SIZE,
            &domain_extended,
        );
        for k in 0..got_proofs.len() {
            let input_points = chunked_bit_reversed_roots[k];
            // Compute the opening proofs the naive way (without fk20)
            let (expected_quotient_comm, expected_output_points) =
                kzgnaive::compute_multi_opening(&ck, &poly_coeff, input_points);

            assert_eq!(expected_output_points, got_set_of_output_points[k]);
            assert_eq!(expected_quotient_comm, got_proofs[k]);
        }
    }
}
