use bls12_381::{ff::Field, Scalar};
use polynomial::domain::Domain;

/// Reverses the least significant `bits` of the given number `n`.
///
/// `n` - The input number whose bits are to be reversed.
/// `bits` - The number of least significant bits to reverse.
///
/// Returns a new `usize` with the specified number of bits reversed.
pub(crate) fn reverse_bits(n: usize, bits: u32) -> usize {
    let mut n = n;
    let mut r = 0;
    for _ in 0..bits {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

/// Computes log2 of an integer.
///
/// Panics if the integer is not a power of two
pub(crate) fn log2(x: u32) -> u32 {
    assert!(x > 0 && x.is_power_of_two(), "x must be a power of two.");
    x.trailing_zeros()
}

// Taken and modified from: https://github.com/filecoin-project/ec-gpu/blob/bdde768d0613ae546524c5612e2ad576a646e036/ec-gpu-gen/src/fft_cpu.rs#L10C8-L10C18
pub fn reverse_bit_order<T>(a: &mut [T]) {
    let n = a.len() as u32;
    assert!(n.is_power_of_two(), "n must be a power of two");
    let log_n = log2(n);

    for k in 0..n {
        let rk = reverse_bits(k as usize, log_n) as u32;
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }
}

/// Generate all of the field elements needed to generate the cosets.
///
/// - num_points denotes how many points we want to open the polynomial at.
///   `num_points` can also be seen as the size of the domain.
/// - num_cosets denotes how many cosets we want to generate, analogously how many proofs we want to produce.
///
/// Returns a `Vec<Scalar>` containing the generated coset elements with length `num_cosets`
///
/// Note: Setting bit_reversed to true will generate the cosets in bit-reversed order.
pub fn coset_gens(num_points: usize, num_cosets: usize, bit_reversed: bool) -> Vec<Scalar> {
    use bls12_381::ff::Field;

    // Compute the generator for the group containing all of the points.
    //
    // Note: generating the whole group, just to get the generator is inefficient
    // However, this code is not on the hot path, so we don't optimize it.
    let domain = Domain::new(num_points);
    let coset_gen = domain.generator;

    // The coset generators are just powers
    // of the generator
    let mut coset_gens = Vec::new();
    for i in 0..num_cosets {
        let generator = if bit_reversed {
            // Note: We could bit-reverse the `coset_gens` vector instead
            // instead of bit-reversing the exponent.
            let rev_i = reverse_bits(i, log2(num_cosets as u32)) as u64;
            coset_gen.pow_vartime([rev_i])
        } else {
            coset_gen.pow_vartime([i as u64])
        };
        coset_gens.push(generator);
    }

    coset_gens
}

/// Given a group of coset evaluations, this method will return/reorder the evaluations as if
/// we evaluated them on the relevant extended domain.
///
/// Note: `domain_order` refers to the order that the evaluations would be in, if they were evaluated on
/// the roots of unity. This is different to the order that we return them in; bit-reversed order.
///
/// The coset indices returned can be used to locate the coset_evaluations in the new flattened order:
///   - The idea is that a particular coset evaluation is evenly distributed across the set of flattened
///     evaluations.
///   Example:
///     - Let's say we have `k` cosets. Each coset holds `m` values. Each coset will have an associated index.
///     - Once this method has completed, we will be given a flattened set of evaluations where the
///       `m` values in each coset are now a distance of `k` values apart from each other.
///     - The first value that was in the first coset, will be in position `0`.
///     - The second value that was in the first coset, will be in position `k`
///     - The third value that was in the first coset, will be in position `2k`
///     - The first value that was in the second coset, will NOT be in position `1`
///        Instead it will be in position `t = reverse_bit_order(1, k)`.
///     - This value of `t` is what the function returns alongside the flattened evaluations,
///       allowing the caller to deduce the other positions.
///
//
// Note: For evaluations that are missing, this method will fill these in with zeroes.
//
// Note: It is the callers responsibility to ensure that there are no duplicate
// coset indices.
pub fn recover_evaluations_in_domain_order(
    domain_size: usize,
    coset_indices: Vec<usize>,
    coset_evaluations: Vec<Vec<Scalar>>,
) -> Option<(Vec<usize>, Vec<Scalar>)> {
    assert_eq!(coset_indices.len(), coset_evaluations.len());

    if coset_indices.is_empty() {
        return None;
    }

    let mut elements = vec![Scalar::ZERO; domain_size];

    // Check that each coset has the same size
    let coset_len = coset_evaluations[0].len();
    let same_len = coset_evaluations
        .iter()
        .all(|coset| coset.len() == coset_len);
    if !same_len {
        return None;
    }

    // Check that none of the indices are "out of bounds"
    // This would result in the subsequent indexing operations to panic
    //
    // The greatest index we will be using is:
    // `t = coset_index * coset_len`
    // Let's denote the returned vectors length as `k`
    // We want t < k
    // => coset_index * coset_len < k
    // => coset_index < k / coset_len
    let index_bound = domain_size / coset_len;
    let all_coset_indices_within_bound = coset_indices
        .iter()
        .all(|coset_index| *coset_index < index_bound);
    if !all_coset_indices_within_bound {
        return None;
    }

    // Iterate over each coset evaluation set and place the evaluations in the correct locations
    for (&coset_index, coset_evals) in coset_indices.iter().zip(coset_evaluations) {
        let start = coset_index * coset_len;
        let end = start + coset_len;

        elements[start..end].copy_from_slice(&coset_evals);
    }

    // Now bit reverse the result, so we get the evaluations as if we had just done
    // and FFT on them. ie we computed the evaluation set and did not do a reverse bit order.
    reverse_bit_order(&mut elements);

    // The order of the coset indices in the returned vector will be different.
    // The new indices of the cosets can be figured out by reverse bit ordering
    // the existing indices.
    let cosets_per_full_domain = domain_size / coset_len;
    let num_bits_coset_per_full_domain = log2(cosets_per_full_domain as u32);

    let new_coset_indices: Vec<_> = coset_indices
        .into_iter()
        .map(|rbo_coset_index| reverse_bits(rbo_coset_index, num_bits_coset_per_full_domain))
        .collect();

    Some((new_coset_indices, elements))
}

/// Generate k = `num_points / points_per_coset` amount of cosets, each containing `points_per_coset` points.
/// The points in each coset will be roots of unity.
/// For FK20, this is a hard requirement for efficient proof generation.
///
/// Note: This method is not exposed because we just bit_reverse the full subgroup
/// It will create the same cosets as calling this method with bit_reversed = true
/// However the ordering inside of the cosets will be different.
/// Note: `bit_reverse` on the full group is more concise.
#[cfg(test)]
pub(crate) fn generate_cosets(
    num_points: usize,
    points_per_coset: usize,
    bit_reversed: bool,
) -> Vec<Vec<Scalar>> {
    let subgroup = Domain::new(points_per_coset).roots;

    let num_cosets = num_points / points_per_coset;

    let generators = coset_gens(num_points, num_cosets, bit_reversed);

    // Manually generate cosets
    let mut cosets = Vec::with_capacity(generators.len());

    for generator in generators {
        let coset: Vec<Scalar> = subgroup
            .iter()
            .map(|&element| generator * element)
            .collect();
        cosets.push(coset);
    }

    cosets
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bls12_381::Scalar;
    use polynomial::{domain::Domain, monomial::poly_eval};

    use crate::fk20::{
        batch_toeplitz::transpose,
        cosets::{
            generate_cosets, log2, recover_evaluations_in_domain_order, reverse_bit_order,
            reverse_bits,
        },
        h_poly::take_every_nth,
    };

    #[test]
    fn bit_reverse_cosets_equality() {
        // This is a test to document the bit reversal technique that is prolific in FFT
        // and how it links to the cosets we generate in FK20.
        //
        // generate_cosets is the general way to generate the cosets used in FK20, where
        // each coset contains roots of unity.
        //
        // If we set bit_reversed = true, then we just re-order the coset generators
        // in "bit reversed" order.
        //
        // bit-reversed order has more utility because once you bit-reverse the roots in the full domain,
        // you can create arbitrary cosets by taking chunks of the bit-reversed roots (One of its main use cases in FFT).
        //
        // This is equivalent to the generic way to generate cosets, the test below shows this.

        let num_points = 8192;
        let points_per_coset = 64;
        // Although you can modify this tests to use different numbers, each coset should have the same number of points
        assert_eq!(num_points % points_per_coset, 0);

        let is_bit_reversed = true;
        let cosets = super::generate_cosets(num_points, points_per_coset, is_bit_reversed);

        // Generate the cosets by reversing the full domain and grouping the bit reversed roots.
        let mut full_domain_roots = Domain::new(num_points).roots;
        reverse_bit_order(&mut full_domain_roots);
        let chunked_bit_reversed_roots: Vec<_> =
            full_domain_roots.chunks(points_per_coset).collect();

        // The two should be equal. The second one is the more efficient way to generate the cosets
        //
        // Note: however that although they should be equal as sets, the order of the elements
        // in each coset is not preserved. So when doing equality, we will do set equality

        // First of all each coset should have the same number of elements
        assert_eq!(cosets.len(), chunked_bit_reversed_roots.len());
        for (coset, bit_reversed_coset) in cosets.iter().zip(chunked_bit_reversed_roots.iter()) {
            assert_eq!(coset.len(), bit_reversed_coset.len());
        }

        // Now we check that the cosets are equal as sets -- this means the ordering in each coset does not matter
        for (coset, bit_reversed_coset) in cosets.iter().zip(chunked_bit_reversed_roots.iter()) {
            let coset_len = coset.len();

            let coset_set: HashSet<_> = coset.iter().map(|s| s.to_bytes_be()).collect();
            let bit_reversed_set: HashSet<_> =
                bit_reversed_coset.iter().map(|s| s.to_bytes_be()).collect();

            assert_eq!(coset_set, bit_reversed_set);

            // A set will remove duplicates, for sanity, let's check that the lengths are the same
            // after we converted the vectors to sets.
            assert_eq!(coset_set.len(), coset_len);
            assert_eq!(bit_reversed_set.len(), coset_len);
        }
    }

    #[test]
    fn test_data_distribution_bit_reverse_order() {
        // This test shows and checks how the original data is distributed amongst the cosets,
        // when we use reverse_bit_order

        let original_data: Vec<_> = (0..4096).map(|i| Scalar::from(i as u64)).collect();

        // First bit-reverse the original data
        let mut bit_reversed_data = original_data.clone();
        reverse_bit_order(&mut bit_reversed_data);

        // Interpolate the bit reversed data
        let domain = Domain::new(4096);
        let poly_coeff = domain.ifft_scalars(bit_reversed_data.clone());

        // Evaluate the poly_coeff on a larger domain
        let extended_domain = Domain::new(4096 * 2);
        let mut extended_data = extended_domain.fft_scalars(poly_coeff);

        // Bit reverse the extended data
        reverse_bit_order(&mut extended_data);

        // The first part of the extended data should match the original data
        let first_half_extended_data = &extended_data[0..original_data.len()];
        assert_eq!(first_half_extended_data, original_data)
    }

    #[test]
    fn test_data_distribution_original_cosets() {
        // This test shows how the data is distributed if we don't use bit-reverse order
        // and instead use the natural cosets we get.
        let original_data: Vec<_> = (0..4096).map(|i| Scalar::from(i as u64)).collect();

        // Interpolate the original data
        let domain = Domain::new(4096);
        let poly_coeff = domain.ifft_scalars(original_data.clone());

        let cosets = generate_cosets(4096 * 2, 64, false);

        let mut coset_evaluations = Vec::new();
        for coset in &cosets {
            let evaluations: Vec<_> = coset
                .iter()
                .map(|coset_element| poly_eval(&poly_coeff, coset_element))
                .collect();
            coset_evaluations.push(evaluations)
        }

        // Let's explain how the data is distributed:
        //
        // Because the cosets are formed by essentially shifting a smaller subgroup
        // by \omega^0, \omega^1, \omega^2, each point in the coset is equally spaced
        // and each coset is simply an offset of the previous.

        /*
         An example using: num_points = 8192 and size of each coset = 64

         Coset 0: [f(\omega^0), f(\omega^128), f(\omega^256), f(\omega^384),...]
         Coset 1: [f(\omega^1), f(\omega^129), f(\omega^257), f(\omega^385),...]
         ...
         Coset K: [f(\omega^{0 * 128 + K}), f(\omega^{1 * 128 + K}), f(\omega^{2 * 128 + K}), f(\omega^{3 * 128 + K}),...f(\omega^{n * t + K})]
         `n` ranges from 0 to the size of the coset (for 64 cosets, n would range from 0 to 63)
         `K` ranges from 0 to the number of cosets
         `t` is the number of cosets we have.
        */

        // Notice that to extract the original data, we would need to take the elements column-wise
        // The repercussions meaning that making a statement over the original data will require
        // all of the cosets. In fact, its a bit more complicated than this because we used a different
        // domain to do the IFFT on the original data than what we used to evaluate the polynomials.
        //
        // Since that domain was half the size of the domain we are using to evaluate the polynomial
        // The original data will live at every even powered evaluation.
        //
        // Generate the evaluations using a faster method
        let extended_evaluations = Domain::new(4096 * 2).fft_scalars(poly_coeff.clone());
        let got_coset_evaluations = take_every_nth(&extended_evaluations, 128);
        assert_eq!(got_coset_evaluations, coset_evaluations);

        // Let's now extract the original data
        let transposed_coset_evaluations = transpose(got_coset_evaluations);
        let flattened_transposed_evaluations: Vec<_> =
            transposed_coset_evaluations.into_iter().flatten().collect();
        // Take the even indexed evaluations
        let even_indexed_evaluations: Vec<_> = flattened_transposed_evaluations
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, v)| v.clone())
            .collect();

        let first_half_even_indexed_evals = &even_indexed_evaluations[0..original_data.len()];
        assert_eq!(first_half_even_indexed_evals, original_data);
    }

    #[test]
    fn check_valid_cosets() {
        let num_points = 8192;
        let num_points_per_coset = 64;

        let cosets = generate_cosets(num_points, num_points_per_coset, false);

        let cosets_flattened: Vec<_> = cosets.into_iter().flatten().collect();

        // Check that there were no duplicates, since cosets should be disjoint
        // Converting the vector to a set will remove duplicates
        let vec_len = cosets_flattened.len();
        let cosets_flattened_set: HashSet<_> = cosets_flattened
            .into_iter()
            .map(|s| s.to_bytes_be())
            .collect();
        let set_len = cosets_flattened_set.len();
        assert_eq!(vec_len, set_len);

        // When we combine the cosets, it should equal the larger subgroup
        let full_subgroup = Domain::new(num_points).roots;
        let full_subgroup_set: HashSet<_> =
            full_subgroup.into_iter().map(|s| s.to_bytes_be()).collect();

        assert_eq!(full_subgroup_set, cosets_flattened_set)
    }

    #[test]
    fn show_data_distribution_on_recover_evaluations_in_domain_order() {
        use bls12_381::ff::Field;

        const DOMAIN_SIZE: usize = 32;
        const POINTS_PER_COSET: usize = 4;
        const NUM_COSETS: usize = 8;

        // Let's pretend that we've generated the coset_evaluations in bit-reversed order
        let bit_reversed_evaluations: Vec<_> = (0..DOMAIN_SIZE)
            .map(|i| Scalar::from((i + 1) as u64))
            .collect();
        let mut bit_reversed_coset_evaluations: Vec<Vec<Scalar>> = bit_reversed_evaluations
            .chunks(POINTS_PER_COSET)
            .map(|chunk| chunk.to_vec())
            .collect();

        // We have 32 values and 4 points per coset, so we have 8 cosets.
        let coset_indices: Vec<_> = (0..NUM_COSETS).collect();

        // Zero out the first coset
        let first_coset = &mut bit_reversed_coset_evaluations[0];
        for evaluation in first_coset {
            *evaluation = Scalar::ZERO
        }
        // Zero out the 4th coset
        let fourth_coset = &mut bit_reversed_coset_evaluations[3];
        for evaluation in fourth_coset {
            *evaluation = Scalar::ZERO
        }

        // Now let's simulate the first and fourth coset missing
        let coset_evaluations_missing: Vec<_> = bit_reversed_coset_evaluations
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 0 && *i != 3)
            .map(|(_, coset)| coset)
            .collect();
        let coset_indices_missing: Vec<_> = coset_indices
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 0 && *i != 3)
            .map(|(_, coset)| coset)
            .collect();

        let (coset_indices_normal_order, coset_evaluations_normal_order) =
            recover_evaluations_in_domain_order(
                DOMAIN_SIZE,
                coset_indices_missing,
                coset_evaluations_missing,
            )
            .unwrap();

        let missing_coset_index_0 = reverse_bits(0, log2(NUM_COSETS as u32));
        let missing_coset_index_3 = reverse_bits(3, log2(NUM_COSETS as u32));

        // Let's show what happened to the evaluations in the first and fourth cosets which were missing
        //
        // It was in the first coset, so the idea is that there will be zeroes in every `rbo(0) + NUM_COSET * i` position
        // where i ranges from 0 to NUM_COSET.
        //
        // The same is also the case for the fourth missing coset, ie we would also have 0s in every `rbo(4) + NUM_COSET * i` position.
        //
        // In general, if the `k`th coset is missing, then this function will return the evaluations with 0s
        // in the `rbo(k) + NUM_COSET  * i`'th positions.
        for block in coset_evaluations_normal_order.chunks(8) {
            for (index, element) in block.into_iter().enumerate() {
                if index == missing_coset_index_0 || index == missing_coset_index_3 {
                    assert_eq!(*element, Scalar::ZERO)
                } else {
                    assert_ne!(*element, Scalar::ZERO)
                }
            }
        }

        // We also note that the coset indices that are returned will not have `missing_coset_index_3` or
        // missing_coset_index_0
        assert!(!coset_indices_normal_order.contains(&missing_coset_index_0));
        assert!(!coset_indices_normal_order.contains(&missing_coset_index_3));
    }

    #[test]
    fn bit_reverse_fuzz() {
        fn naive_bit_reverse(n: u32, l: u32) -> u32 {
            assert!(l.is_power_of_two());
            let num_bits = l.trailing_zeros();
            n.reverse_bits() >> (32 - num_bits)
        }

        for i in 0..10 {
            for k in (1..31).map(|exponent| 2u32.pow(exponent)) {
                let expected = naive_bit_reverse(i, k);
                let got = reverse_bits(i as usize, log2(k)) as u32;
                assert_eq!(expected, got)
            }
        }
    }
}
