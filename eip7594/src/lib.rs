use constants::{BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT};
pub use prover::ProverContext;
pub use trusted_setup::TrustedSetup;
pub use verifier::VerifierContext;

pub mod constants;
pub mod prover;
mod serialization;
pub mod trusted_setup;
pub mod verifier;

pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];
pub type Bytes48Ref<'a> = &'a [u8; 48];

// TODO: We require a bit of feedback re usage to know whether we should make
// TODO: Cell type just be Vec<u8> -- This would avoid accidental stack overflows.
pub type Cell = Box<[u8; BYTES_PER_CELL]>;
pub type CellRef<'a> = &'a [u8; BYTES_PER_CELL];

pub type KZGProof = [u8; BYTES_PER_COMMITMENT];
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];
pub type CellID = u64;
pub type RowIndex = u64;
pub type ColumnIndex = u64;

mod errors;

/// The context that will be used to create and verify proofs.
#[derive(Debug)]
pub struct PeerDASContext {
    pub prover_ctx: ProverContext,
    pub verifier_ctx: VerifierContext,
}

impl Default for PeerDASContext {
    fn default() -> Self {
        let trusted_setup = TrustedSetup::default();
        PeerDASContext {
            prover_ctx: ProverContext::new(&trusted_setup),
            verifier_ctx: VerifierContext::new(&trusted_setup),
        }
    }
}

impl PeerDASContext {
    pub fn with_threads(trusted_setup: &TrustedSetup, num_threads: usize) -> Self {
        let thread_pool = std::sync::Arc::new(rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap());
        PeerDASContext {
            prover_ctx: ProverContext::from_threads_pool(trusted_setup, thread_pool.clone()),
            verifier_ctx: VerifierContext::from_thread_pool(trusted_setup, thread_pool),
        }
    }

    pub fn prover_ctx(&self) -> &ProverContext {
        &self.prover_ctx
    }

    pub fn verifier_ctx(&self) -> &VerifierContext {
        &self.verifier_ctx
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::Scalar;
    use kzg_multi_open::polynomial::domain::Domain;
    use kzg_multi_open::{
        create_eth_commit_opening_keys, fk20::naive, proof::compute_multi_opening_naive,
        reverse_bit_order,
    };

    // This test becomes redundant once we have consensus-specs fixed test vectors
    // added. Although, it may be beneficial to test consensus-specs fixed test vectors against
    // the naive implementation, if it doesn't add too much overhead.
    #[test]
    fn test_consistency_between_naive_kzg_naive_fk20() {
        // Setup
        let (ck, _) = create_eth_commit_opening_keys();
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
        let mut polynomial: Vec<_> = (0..POLYNOMIAL_LEN)
            .map(|i| -Scalar::from(i as u64))
            .collect();
        // Polynomial really corresponds to the evaluation form, so we need
        // to apply bit reverse order and then IFFT to get the coefficients
        reverse_bit_order(&mut polynomial);
        let poly_coeff = domain.ifft_scalars(polynomial);

        let (got_proofs, got_set_of_output_points) = naive::fk20_open_multi_point(
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
