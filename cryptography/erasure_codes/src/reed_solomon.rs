use bls12_381::{
    batch_inversion::batch_inverse,
    ff::{Field, PrimeField},
    Scalar,
};

use crate::errors::RSError;
use polynomial::{domain::Domain, poly_coeff::vanishing_poly, CosetFFT};

/// ErasurePattern is an abstraction created to capture the idea
/// that erasures do not appear in completely random locations.
///
/// This is useful as it allows us to optimize the construction of
/// the vanishing polynomial. This is by far the most time consuming part
/// of unique decoding.
pub(crate) enum ErasurePattern {
    /// Given a block_size, we can group the codeword into blocks.
    /// A block erasure index now signifies
    /// an erasure in the same position of each block.
    /// Example:
    ///  - Codeword = [0,b,0,d,0,f,0,h]
    ///  - block_size = 2
    ///  - block_index = 0
    ///
    /// In the above example, we had 4 blocks and each block had an erasure at index 0.
    BlockSynchronizedErasures(BlockErasureIndices),
    /// There is no pattern to the missing erasures.
    ///
    /// This is used for tests.
    #[cfg(test)]
    Random { indices: Vec<usize> },
}

/// Given a `block_size`, BlockErasureIndex denotes
/// the index in every block that an erasure has occurred.
type BlockErasureIndex = usize;

#[derive(Debug, Clone, Default)]
pub struct BlockErasureIndices(pub Vec<BlockErasureIndex>);

#[derive(Debug)]
pub struct ReedSolomon {
    /// Denotes the factor by which the message/poly_len will be expanded.
    /// Example, if poly_len = 2 and expansion_factor = 4, Then the codeword will have length 4 * 2 = 8.
    expansion_factor: usize,
    /// The length of the polynomial that we will be encoding to a codeword.
    poly_len: usize,
    /// The domain that we will use to convert the polynomial in coefficient form (msg)
    /// to a codeword.
    ///
    /// Note: This domain will have size = poly_len * expansion_factor.
    evaluation_domain: Domain,
    /// Denotes the number of scalars that we should group together in the codeword to form a block.
    ///
    /// When the ErasurePattern is BlockSynchronized, we know that every block will
    /// have an erasure at the BlockErasureIndices given.
    ///
    /// Another way to think about this is that `block_size` corresponds to the gap between
    /// each `BlockSynchronizedErasures` in a codeword.
    block_size: usize,
    /// Given we split the codeword into `block_size` chunks, `num_blocks` denotes how many blocks
    /// we would have.
    ///
    /// Note: This also denotes the number of synchronized erasures or block propagated erasures that may occur.
    num_blocks: usize,
    /// The domain that we will use to efficiently compute the vanishing polynomial with, when the erasure pattern
    /// being used is `BlockSynchronizedErasures`.
    block_size_domain: Domain,

    fft_coset_gen: CosetFFT,
}

impl ReedSolomon {
    pub fn new(poly_len: usize, expansion_factor: usize, block_size: usize) -> Self {
        assert!(expansion_factor.is_power_of_two());
        assert!(poly_len.is_power_of_two());
        assert!(block_size.is_power_of_two());

        let evaluation_size = poly_len * expansion_factor;
        let evaluation_domain = Domain::new(evaluation_size);

        let num_blocks = evaluation_size / block_size;

        let block_size_domain = Domain::new(block_size);

        let fft_coset_gen = CosetFFT::new(Scalar::MULTIPLICATIVE_GENERATOR);

        Self {
            poly_len,
            evaluation_domain,
            expansion_factor,
            block_size,
            block_size_domain,
            num_blocks,
            fft_coset_gen,
        }
    }

    /// Returns the maximum number of known missing values that we can
    /// tolerate before are not able to recover the message.
    ///
    /// Note: we need to have at least `poly_len` evaluations
    const fn acceptable_num_random_erasures(&self) -> usize {
        let total_codeword_len = self.poly_len * self.expansion_factor;
        let min_num_evaluations_needed = self.poly_len;
        total_codeword_len - min_num_evaluations_needed
    }

    /// Returns the maximum number of block erasures indices that can be missing
    /// before we are not able to recover the message.
    ///
    /// Note: This can also be computed by doing block_size / expansion_factor
    pub const fn acceptable_num_block_erasures(&self) -> usize {
        self.acceptable_num_random_erasures() / self.num_blocks
    }

    /// The number of scalars in the reed solomon encoded polynomial
    pub const fn codeword_length(&self) -> usize {
        self.poly_len * self.expansion_factor
    }

    /// Encodes a polynomial in coefficient form by evaluating it at `poly_len * expansion_factor`
    /// points.
    pub fn encode(&self, poly_coefficient_form: Vec<Scalar>) -> Result<Vec<Scalar>, RSError> {
        if poly_coefficient_form.len() > self.poly_len {
            return Err(RSError::PolynomialHasTooManyCoefficients {
                num_coefficients: poly_coefficient_form.len(),
                max_num_coefficients: self.poly_len,
            });
        }
        Ok(self.evaluation_domain.fft_scalars(poly_coefficient_form))
    }

    /// Given a codeword and a list of its erasures,
    /// This method will return the polynomial in coefficient form
    /// that is able to generate the codeword with the erasures recovered.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#recover_polynomialcoeff
    pub fn recover_polynomial_coefficient(
        &self,
        codeword_with_erasures: Vec<Scalar>,
        erasures: BlockErasureIndices,
    ) -> Result<Vec<Scalar>, RSError> {
        self.recover_polynomial_coefficient_erasure_pattern(
            codeword_with_erasures,
            ErasurePattern::BlockSynchronizedErasures(erasures),
        )
    }

    #[cfg(test)]
    fn recover_polynomial_coefficient_random_erasure(
        &self,
        codeword_with_erasures: Vec<Scalar>,
        random_erasure: Vec<usize>,
    ) -> Result<Vec<Scalar>, RSError> {
        self.recover_polynomial_coefficient_erasure_pattern(
            codeword_with_erasures,
            ErasurePattern::Random {
                indices: random_erasure,
            },
        )
    }

    /// Constructs a polynomial that vanishes on all of the block indices in each block.
    ///
    /// This method makes the following assumptions:
    ///  - All of the blocks are not missing.
    ///  - The block indices are not repeated.
    ///  - The block indices are valid (ie each index references a block)
    ///
    /// It is the responsibility of the caller to ensure that these are valid.
    ///
    /// - We note that the algorithm below has an edge case when all of the blocks
    ///   are missing. In that particular case, the vanishing polynomial
    ///   would be Z(x) = x^{2n} - 1.
    ///   We explicitly do not handle this case because this is an internal function
    ///   and recovery would fail if all of the blocks were missing.
    ///
    /// The matching function in the specs is: https://github.com/ethereum/consensus-specs/blob/13ac373a2c284dc66b48ddd2ef0a10537e4e0de6/specs/_features/eip7594/polynomial-commitments-sampling.md#construct_vanishing_polynomial
    fn construct_vanishing_poly_from_block_erasures(
        &self,
        block_indices: BlockErasureIndices,
    ) -> Vec<Scalar> {
        assert!(block_indices.0.len() != self.block_size, "all of the blocks are missing. This should have been checked by the caller of this method");

        let evaluation_domain_size = self.evaluation_domain.roots.len();

        // Compute the polynomial that vanishes on all roots of unity corresponding
        // to the block_indices.
        //
        // We are essentially calculating the polynomial that vanishes only on the indices
        // in the first block.
        let z_x_missing_indices_roots: Vec<_> = block_indices
            .0
            .iter()
            .map(|index| self.block_size_domain.roots[*index])
            .collect();
        let vanish_poly_first_block = vanishing_poly(&z_x_missing_indices_roots);

        // Expand the vanishing polynomial, so that it vanishes on all blocks in the codeword
        // at the same indices.
        //
        // Example; consider the following polynomial f(x) = x - r
        // It vanishes/has roots at `r`.
        //
        // Now if expand it by a factor of three which is the process of shifting all coefficients
        // up three spaces, we get the polynomial g(x) = x^3 - r^3.
        // g(x) has all of the roots of f(x) and a few extra roots.
        //
        // The roots of g(x) can be characterized as {r, \omega *r, \omega^2 * r}
        // where \omega is a third root of unity.
        //
        // This process is happening below, ie we create a polynomial which has roots `r_i`
        // Then we expand it by `num_blocks` so that it has additional roots \omega^i * r_i
        // Where \omega is a `num_blocks` root of unity.
        let mut z_x = vec![Scalar::ZERO; evaluation_domain_size];
        for (i, coeff) in vanish_poly_first_block.into_iter().enumerate() {
            // Let's compute the bounds for the array access below to argue that it is safe:
            //
            //  For all array accesses to be in bound, we have:
            //  i * self.num_blocks < z_x.len()
            //  => i * self.num_blocks < poly_len * expansion_factor
            //  => i < poly_len * expansion_factor / self.num_blocks
            //  => i < block_size
            // We know that i \in [0, block_indices.len]
            // A simple example; when we have one erasure, we get a linear polynomial and i \in [0,1]
            // => block_indices.len < block_size
            //
            // If block_indices.len >= block_size, it means either two things:
            // - All of the blocks are missing
            // - There are duplicate block indices.
            // This function makes the assumption that the caller has checked these conditions.
            z_x[i * self.num_blocks] = coeff;
        }

        z_x
    }

    fn construct_vanishing_poly_from_erasure_pattern(
        &self,
        erasures: ErasurePattern,
    ) -> Result<Vec<Scalar>, RSError> {
        match erasures {
            ErasurePattern::BlockSynchronizedErasures(indices) => {
                // Check that each block index is valid
                for block_index in &indices.0 {
                    if *block_index >= self.block_size {
                        return Err(RSError::InvalidBlockIndex {
                            block_index: *block_index,
                            block_size: self.block_size,
                        });
                    }
                }
                // This method is only used for recovery.
                // Check that we do not have too many erasures, such that we cannot
                // recover.
                if indices.0.len() > self.acceptable_num_block_erasures() {
                    return Err(RSError::TooManyBlockErasures {
                        num_block_erasures: indices.0.len(),
                        max_num_block_erasures_accepted: self.acceptable_num_block_erasures(),
                    });
                }
                Ok(self.construct_vanishing_poly_from_block_erasures(indices))
            }
            #[cfg(test)]
            ErasurePattern::Random { indices } => {
                assert!(
                    indices.len() <= self.acceptable_num_random_erasures(),
                    "num random erasures = {} but tolerable erasures = {}",
                    indices.len(),
                    self.acceptable_num_random_erasures()
                );
                // Computes the polynomial in coefficient form, that vanishes
                // on all of the \omega^i roots, where `i` is taken from the indices vector
                // and \omega is a primitive root of unity used to generate the evaluation domain.
                let roots: Vec<_> = indices
                    .into_iter()
                    .map(|index| self.evaluation_domain.roots[index])
                    .collect();
                Ok(vanishing_poly(&roots))
            }
        }
    }

    /// The matching function in the spec is: https://github.com/ethereum/consensus-specs/blob/dc5f74da0e9834fa842cdcb33c64b3a1fb1ad579/specs/_features/eip7594/polynomial-commitments-sampling.md#recover_data
    fn recover_polynomial_coefficient_erasure_pattern(
        &self,
        data_eval: Vec<Scalar>,
        erasure: ErasurePattern,
    ) -> Result<Vec<Scalar>, RSError> {
        // Compute Z(X) which is the polynomial that vanishes on all
        // of the missing points
        let z_x = self.construct_vanishing_poly_from_erasure_pattern(erasure)?;

        // Compute Z(X)_eval which is the vanishing polynomial evaluated
        // at the missing points
        let z_x_eval = self.evaluation_domain.fft_scalars(z_x.clone());

        // Compute (D * Z)(X) or (E * Z)(X) (same polynomials)
        let ez_eval: Vec<_> = z_x_eval
            .iter()
            .zip(data_eval)
            .map(|(zx, d)| zx * d)
            .collect();

        let dz_poly = self.evaluation_domain.ifft_scalars(ez_eval);

        let coset_dz_eval = self
            .evaluation_domain
            .coset_fft_scalars(dz_poly, &self.fft_coset_gen);
        let mut inv_coset_z_x_eval = self
            .evaluation_domain
            .coset_fft_scalars(z_x, &self.fft_coset_gen);
        // We know that none of the values will be zero since we are evaluating z_x
        // over a coset, that we know it has no roots in.
        batch_inverse(&mut inv_coset_z_x_eval);
        let coset_quotient_eval: Vec<_> = coset_dz_eval
            .iter()
            .zip(inv_coset_z_x_eval)
            .map(|(d, zx_inv)| d * zx_inv)
            .collect();

        let coefficients = self
            .evaluation_domain
            .coset_ifft_scalars(coset_quotient_eval, &self.fft_coset_gen);

        // Check that the polynomial being returned has the correct degree
        //
        // The first poly_len terms should describe the polynomial and the
        // higher terms should have zero coefficients.
        for coefficient in coefficients.iter().skip(self.poly_len) {
            if *coefficient != Scalar::ZERO {
                return Err(RSError::PolynomialHasInvalidLength {
                    num_coefficients: coefficients.len(),
                    expected_num_coefficients: self.poly_len,
                });
            }
        }

        // Return the truncated polynomial
        Ok(coefficients[0..self.poly_len].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::{ff::Field, Scalar};

    use crate::{reed_solomon::ErasurePattern, BlockErasureIndices, ReedSolomon};

    #[test]
    #[should_panic]
    fn test_compute_vanishing_panics() {
        // Document the case where all block indices are missing.
        // The method should panic.
        const POLY_LEN: usize = 16;
        const EXPANSION_FACTOR: usize = 2;
        const BLOCK_SIZE: usize = 1;

        let rs = ReedSolomon::new(POLY_LEN, EXPANSION_FACTOR, BLOCK_SIZE);
        let block_erasure_indices: Vec<_> = (0..BLOCK_SIZE).collect();

        rs.construct_vanishing_poly_from_block_erasures(BlockErasureIndices(block_erasure_indices));
    }

    #[test]
    fn smoke_test_recovery_no_erasures() {
        const POLY_LEN: usize = 16;
        const EXPANSION_FACTOR: usize = 2;
        const BLOCK_SIZE: usize = 1;

        let rs = ReedSolomon::new(POLY_LEN, EXPANSION_FACTOR, BLOCK_SIZE);
        let poly_coeff: Vec<_> = (0..16).map(|i| -Scalar::from(i)).collect();

        let codewords = rs.encode(poly_coeff.clone()).unwrap();
        assert_eq!(codewords.len(), 32);

        let got_poly_coeff = rs
            .recover_polynomial_coefficient(codewords, BlockErasureIndices::default())
            .unwrap();

        assert_eq!(got_poly_coeff.len(), poly_coeff.len());
        assert_eq!(got_poly_coeff, poly_coeff);
    }

    #[test]
    fn test_vanishing_poly_erasure_pattern_block_synchronized() {
        let indices = vec![0, 1, 2, 3];

        const POLY_LEN: usize = 512;
        const EXPANSION_FACTOR: usize = 2;
        const BLOCK_SIZE: usize = 16;

        let rs = ReedSolomon::new(POLY_LEN, EXPANSION_FACTOR, BLOCK_SIZE);
        let z =
            rs.construct_vanishing_poly_from_block_erasures(BlockErasureIndices(indices.clone()));

        assert_eq!(z.len(), POLY_LEN * EXPANSION_FACTOR);

        // Evaluate vanishing polynomial on the evaluation domain
        let evals = rs.evaluation_domain.fft_scalars(z);

        let blocks: Vec<_> = evals.chunks(BLOCK_SIZE).collect();
        assert!(blocks.len() == rs.num_blocks);

        // For each block, we should have zeroes on the indices in that block
        for block in &blocks {
            for index in 0..BLOCK_SIZE {
                if indices.contains(&index) {
                    assert_eq!(block[index], Scalar::ZERO);
                } else {
                    assert_ne!(block[index], Scalar::ZERO);
                }
            }
        }
    }

    #[test]
    fn test_vanishing_poly_erasure_pattern_equiv_random() {
        let indices = vec![0, 1];

        const POLY_LEN: usize = 64;
        const EXPANSION_FACTOR: usize = 2;
        const BLOCK_SIZE: usize = 4;

        let rs = ReedSolomon::new(POLY_LEN, EXPANSION_FACTOR, BLOCK_SIZE);
        let got_z_x =
            rs.construct_vanishing_poly_from_block_erasures(BlockErasureIndices(indices.clone()));
        let got_z_x_lagrange_form = rs.evaluation_domain.fft_scalars(got_z_x);

        let blocks: Vec<_> = got_z_x_lagrange_form.chunks(BLOCK_SIZE).collect();

        let mut all_indices = Vec::new();
        for index in indices {
            for i in 0..blocks.len() {
                all_indices.push(index + i * BLOCK_SIZE)
            }
        }
        let z_x = rs
            .construct_vanishing_poly_from_erasure_pattern(ErasurePattern::Random {
                indices: all_indices,
            })
            .unwrap();

        let expected_z_x_lagrange_form = rs.evaluation_domain.fft_scalars(z_x);
        assert_eq!(expected_z_x_lagrange_form, got_z_x_lagrange_form)
    }

    #[test]
    fn smoke_test_recovery_upto_num_acceptable_random_erasures() {
        const POLY_LEN: usize = 16;
        const EXPANSION_FACTOR: usize = 2;
        const BLOCK_SIZE: usize = 1; // Note: This is not used for random erasures

        let rs = ReedSolomon::new(POLY_LEN, EXPANSION_FACTOR, BLOCK_SIZE);
        let poly_coeff = (0..POLY_LEN)
            .map(|i| Scalar::from(i as u64))
            .collect::<Vec<_>>();

        let original_codewords = rs.encode(poly_coeff.clone()).unwrap();
        let acceptable_num_erasures: Vec<_> = (0..=rs.acceptable_num_random_erasures()).collect();
        for num_erasures in acceptable_num_erasures {
            let mut codewords_with_erasures = original_codewords.clone();

            // zero out `num_erasures` amount of evaluations to simulate erasures
            let mut missing_indices = Vec::new();
            for (index, codewords_with_erasure) in codewords_with_erasures
                .iter_mut()
                .enumerate()
                .take(num_erasures)
            {
                *codewords_with_erasure = Scalar::ZERO;
                missing_indices.push(index);
            }

            let recovered_poly_coeff = rs
                .recover_polynomial_coefficient_random_erasure(
                    codewords_with_erasures,
                    missing_indices,
                )
                .unwrap();
            assert_eq!(recovered_poly_coeff.len(), poly_coeff.len());
            assert_eq!(recovered_poly_coeff, poly_coeff)
        }
    }

    #[test]
    fn smoke_test_recovery_upto_num_acceptable_block_erasures() {
        const POLY_LEN: usize = 128;
        const EXPANSION_FACTOR: usize = 2;
        const BLOCK_SIZE: usize = 4;

        let rs = ReedSolomon::new(POLY_LEN, EXPANSION_FACTOR, BLOCK_SIZE);
        let poly_coeff = (0..POLY_LEN)
            .map(|i| Scalar::from(i as u64))
            .collect::<Vec<_>>();

        let original_codewords = rs.encode(poly_coeff.clone()).unwrap();
        let num_block_erasures: Vec<_> = (0..=BLOCK_SIZE).collect();

        for num_block_erasures in num_block_erasures {
            let mut blocks: Vec<Vec<Scalar>> = original_codewords
                .chunks(BLOCK_SIZE)
                .map(|block| block.to_vec())
                .collect();

            // zero out `num_erasures` amount of evaluations to simulate erasures
            let mut missing_block_indices = Vec::new();
            for index in 0..num_block_erasures {
                for block in &mut blocks {
                    block[index] = Scalar::ZERO
                }
                missing_block_indices.push(index);
            }

            let codeword_with_erasures = blocks.into_iter().flatten().collect();

            let maybe_recovered_poly_coeff = rs.recover_polynomial_coefficient(
                codeword_with_erasures,
                BlockErasureIndices(missing_block_indices),
            );
            if num_block_erasures <= rs.acceptable_num_block_erasures() {
                let recovered_poly_coeff = maybe_recovered_poly_coeff.unwrap();
                assert_eq!(recovered_poly_coeff.len(), poly_coeff.len());
                assert_eq!(recovered_poly_coeff, poly_coeff)
            } else {
                assert!(maybe_recovered_poly_coeff.is_err())
            }
        }
    }
}
