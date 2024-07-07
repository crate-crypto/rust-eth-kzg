use bls12_381::Scalar;
use polynomial::domain::Domain;

/// Generate all of the field elements needed to generate the cosets.
///
/// - num_points denotes how many points we want to open the polynomial at.
/// - num_cosets denotes how many cosets we want to generate, analogously how many proofs we want to produce.
///
/// Setting bit_reversed to true will generate the cosets in bit-reversed order.
pub(crate) fn coset_gens(num_points: usize, num_cosets: usize, bit_reversed: bool) -> Vec<Scalar> {
    use bls12_381::ff::Field;

    // Compute the generator for the group containing all of the points.
    // TODO: generating the whole group, just to get the generator is inefficient
    let domain = Domain::new(num_points);
    let coset_gen = domain.generator;

    // TODO: We have this method duplicated in a few places, we should deduplicate
    // TODO: FFT and verifier.rs has it
    fn reverse_bits(n: usize, bits: u32) -> usize {
        let mut n = n;
        let mut r = 0;
        for _ in 0..bits {
            r = (r << 1) | (n & 1);
            n >>= 1;
        }
        r
    }

    fn log2_power_of_2(x: u64) -> u32 {
        assert!(x.is_power_of_two(), "Input must be a power of 2");
        x.trailing_zeros()
    }

    // The coset generators are just powers
    // of the generator
    let mut coset_gens = Vec::new();
    for i in 0..num_cosets {
        let generator = if bit_reversed {
            // TODO: We could just bit-reverse the `coset_gens` method instead
            let rev_i = reverse_bits(i, log2_power_of_2(num_cosets as u64)) as u64;
            coset_gen.pow_vartime(&[rev_i])
        } else {
            coset_gen.pow_vartime(&[i as u64])
        };
        coset_gens.push(generator);
    }

    coset_gens
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
fn generate_cosets(
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

    use crate::{
        fk20::{cosets::generate_cosets, take_every_nth},
        reverse_bit_order,
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

            // A set will remove duplicates, for sanity, lets check that the lengths are the same
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

        // Lets explain how the data is distributed:
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

        // Lets now extract the original data
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

    fn transpose<T: Clone>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
        if v.is_empty() || v[0].is_empty() {
            return Vec::new();
        }

        let rows = v.len();
        let cols = v[0].len();

        let mut result = vec![Vec::with_capacity(rows); cols];

        for row in v {
            for (i, elem) in row.into_iter().enumerate() {
                result[i].push(elem);
            }
        }

        result
    }
}
