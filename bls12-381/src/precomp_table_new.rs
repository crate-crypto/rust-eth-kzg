use std::cell::RefCell;

use crate::{batch_point_addition::{batch_addition, batch_addition_mut}, G1Projective, Scalar};
use ark_ff::BigInteger;
use blstrs::{Fp, G1Affine};
use group::{prime::PrimeCurveAffine, Group};

// This is a precomp table that will store all of the precomputed values for a range of points
// ie for points P1, P2, P3
// we store P1 + P2, P1 + P2, P1 + P3, P2 + P3, P1, P2, P3 (7 elements)
// We then use these points to compute the scalar sum, and double as needed
//
// 5 * P1
pub struct PrecompTableNew {
    bases: Vec<Vec<G1Affine>>,
    base: usize,
    scratch_pad: Vec<G1Affine>
}


impl PrecompTableNew {
    pub fn new(generators: Vec<G1Projective>, base: usize) -> Self {
        // Rows will contain -2G_i, -G_i, G_i, 2G_i,..., 2^base-2 G_i
        // ie they will contain half the number of points as the base
        // The scalar range will then be -2^(base-1) to 2^(base-1) - 1
        // instead of 0 to 2^base - 1
        let length_of_row = ((1 << base) - 1) / 2;
        let rows : Vec<_>= generators
            .into_iter()
            .map(|gen| Self::compute_sequential_powers_of_point(gen.into(), base - 1))
            .collect();

        let estimate = rows.len() * 64 * 32;
        let scratch_pad = Vec::with_capacity(estimate);

        PrecompTableNew {
            bases: rows,
            base: base - 1,
            scratch_pad,
        }
    }

    /// Computes [G, 2G, 3G, 4G, ..., 2^{base_exponent - 1} G]
    fn compute_sequential_powers_of_point(gen: G1Affine, base_exponent: usize) -> Vec<G1Affine> {
        let mut powers_of_g = Vec::with_capacity(1 << base_exponent);
        let mut powers_of_two = Vec::new();

        let mut cur = G1Projective::from(gen);
        powers_of_two.push(cur);

        for _ in 1..base_exponent {
            cur = cur.double().into();
            powers_of_two.push(cur.into());
        }

        powers_of_g.push(G1Projective::from(gen));
        for i in 1..(1 << base_exponent) - 1 {
            let mut temp = G1Projective::from(gen);
            for j in 0..base_exponent {
                if (i & (1 << j)) != 0 {
                    temp = (temp + powers_of_two[j]).into();
                }
            }
            powers_of_g.push(temp);
        }
        use group::prime::PrimeCurveAffine;
        use group::Curve;
        let mut powers_of_g_affine = vec![G1Affine::identity(); powers_of_g.len()];
        G1Projective::batch_normalize(&powers_of_g, &mut powers_of_g_affine);

        powers_of_g_affine
    }

    pub fn scalar_mul(&self, scalar: Scalar) -> G1Projective {
        // Recode scalars
        use ff::PrimeField;
        // let now = std::time::Instant::now();
        let bigint = convert_scalar_to_arkworks_bigint(&scalar);
        let mut scalar_bytes: Vec<_> =
            make_digits(&bigint, self.base, Scalar::NUM_BITS as usize).collect();
        scalar_bytes.reverse();
        // Iterate over the precomputed table rows and scalar bytes simultaneously
        let mut result: G1Projective = G1Projective::identity();

        for byte in scalar_bytes.into_iter() {
            let now = std::time::Instant::now();
            for _ in 0..self.base {
                result = result.double();
            }
            println!("Time to double: {:?}", now.elapsed());

            // Add the corresponding precomputed point from the current row
            if byte < 0 {
                // let now = std::time::Instant::now();
                let tmp = self.bases[0][(-byte) as usize - 1];
                // println!("Time to get point: {:?}", now.elapsed());
                // let now = std::time::Instant::now();
                result -= tmp;
                // println!("Time to add point: {:?}", now.elapsed());
            } else if byte > 0 {
                // Minus one because we skip the zero window
                // let now = std::time::Instant::now();
                let tmp = self.bases[0][(byte as usize) - 1];
                // println!("Time to get point: {:?}", now.elapsed());
                // let now = std::time::Instant::now();
                result += tmp;
                // println!("Time to add point: {:?}", now.elapsed());
            }
        }

        result
    }

    pub fn msm(&mut self, scalars: Vec<Scalar>) -> G1Projective {
        // Recode scalars
        use ff::PrimeField;
        let now = std::time::Instant::now();
        let scalars_bytes: Vec<_> = scalars
            .into_iter()
            .map(|scalar| {
                let bigint = convert_scalar_to_arkworks_bigint(&scalar);
                let mut scalar_decomp: Vec<_> =
                    make_digits(&bigint, self.base, Scalar::NUM_BITS as usize).collect();
                scalar_decomp.reverse();
                scalar_decomp
            })
            .collect();
        println!("Time to convert scalars {:?}", now.elapsed());
        let num_iterations = scalars_bytes[0].len();
        // let mut scratch_space = Vec::new();
        // Iterate over the precomputed table rows and scalar bytes simultaneously
        let mut result = G1Projective::identity();
        // TODO: can we collect all of the window sums and then do a 
        //TODO: batch doubling?
        // let mut sums = vec![];
        let mut total_batch_addition_time = std::time::Duration::default();
        for window_index in 0..num_iterations {
            self.scratch_pad.clear();
            for _ in 0..self.base {
                result = result.double();
            }
            for (j, scalar_decomp) in scalars_bytes.iter().enumerate() {
                let byte = scalar_decomp[window_index];

                // Add the corresponding precomputed point from the current row
                if byte < 0 {
                    // let now = std::time::Instant::now();
                    let tmp = self.bases[j][(-byte) as usize - 1];
                    // println!("Time to get point: {:?}", now.elapsed());
                    // let now = std::time::Instant::now();
                    self.scratch_pad.push(-tmp);
                    // window_sum -= tmp;
                    // println!("Time to add point: {:?}", now.elapsed());
                } else if byte > 0 {
                    // Minus one because we skip the zero window
                    // let now = std::time::Instant::now();
                    let tmp = self.bases[j][(byte as usize) - 1];
                    self.scratch_pad.push(tmp);
                    // println!("Time to get point: {:?}", now.elapsed());
                    // let now = std::time::Instant::now();
                    // window_sum += tmp;
                    // println!("Time to add point: {:?}", now.elapsed());
                }
            }
            // sums.push(window_sum)
            let now = std::time::Instant::now();
            result += batch_addition_mut(&mut self.scratch_pad);
            total_batch_addition_time += now.elapsed();
            // println!("Time taken to batch addition {:?}", now.elapsed());
        }
        // for byte in scalar_bytes.into_iter() {
        //     let now = std::time::Instant::now();
        //     for _ in 0..self.base {
        //         result = result.double();
        //     }
        //     println!("Time to double: {:?}", now.elapsed());

        //     // Add the corresponding precomputed point from the current row
        //     if byte < 0 {
        //         // let now = std::time::Instant::now();
        //         let tmp = self.bases[0][(-byte) as usize - 1];
        //         // println!("Time to get point: {:?}", now.elapsed());
        //         // let now = std::time::Instant::now();
        //         result -= tmp;
        //         // println!("Time to add point: {:?}", now.elapsed());
        //     } else if byte > 0 {
        //         // Minus one because we skip the zero window
        //         // let now = std::time::Instant::now();
        //         let tmp = self.bases[0][(byte as usize) - 1];
        //         // println!("Time to get point: {:?}", now.elapsed());
        //         // let now = std::time::Instant::now();
        //         result += tmp;
        //         // println!("Time to add point: {:?}", now.elapsed());
        //     }
        // }
        // sums.into_iter().sum()
        println!("Time taken to batch addition {:?}", total_batch_addition_time.as_micros());

        result
    }
}

pub fn convert_scalar_to_arkworks_bigint(scalar: &Scalar) -> ark_ff::BigInteger256 {
    // We are piggy backing off of arkworks data structures to recode the scalar
    let bytes = scalar.to_bytes_le();

    fn u256_to_u64s(bytes: [u8; 32]) -> [u64; 4] {
        let mut result = [0u64; 4];

        for i in 0..4 {
            let start = i * 8;
            let end = start + 8;
            let chunk = &bytes[start..end];
            result[i] = u64::from_le_bytes(chunk.try_into().unwrap());
        }

        result
    }

    let u64_limbs = u256_to_u64s(bytes);
    use ark_ff::BigInteger256;

    BigInteger256::new(u64_limbs)
}

// Copied from https://github.com/arkworks-rs/algebra/blob/065cd24fc5ae17e024c892cee126ad3bd885f01c/ec/src/scalar_mul/variable_base/mod.rs#L255
pub fn make_digits(a: &impl BigInteger, w: usize, num_bits: usize) -> impl Iterator<Item = i64> + '_ {
    let scalar = a.as_ref();
    let radix: u64 = 1 << w;
    let window_mask: u64 = radix - 1;

    let mut carry = 0u64;
    let num_bits = if num_bits == 0 {
        a.num_bits() as usize
    } else {
        num_bits
    };
    let digits_count = (num_bits + w - 1) / w;

    (0..digits_count).into_iter().map(move |i| {
        // Construct a buffer of bits of the scalar, starting at `bit_offset`.
        let bit_offset = i * w;
        let u64_idx = bit_offset / 64;
        let bit_idx = bit_offset % 64;
        // Read the bits from the scalar
        let bit_buf = if bit_idx < 64 - w || u64_idx == scalar.len() - 1 {
            // This window's bits are contained in a single u64,
            // or it's the last u64 anyway.
            scalar[u64_idx] >> bit_idx
        } else {
            // Combine the current u64's bits with the bits from the next u64
            (scalar[u64_idx] >> bit_idx) | (scalar[1 + u64_idx] << (64 - bit_idx))
        };

        // Read the actual coefficient value from the window
        let coef = carry + (bit_buf & window_mask); // coef = [0, 2^r)

        // Recenter coefficients from [0,2^w) to [-2^w/2, 2^w/2)
        carry = (coef + radix / 2) >> w;
        let mut digit = (coef as i64) - (carry << w) as i64;

        if i == digits_count - 1 {
            digit += (carry << w) as i64;
        }
        digit
    })
}

#[cfg(test)]
mod tests {
    use super::PrecompTableNew;
    use crate::{lincomb::g1_lincomb, G1Projective, Scalar};
    use ff::Field;
    use group::Group;
    use rand::thread_rng;
    #[test]
    fn check_bug_new() {
        let table = PrecompTableNew::new(vec![G1Projective::generator()], 8);
        let scalar = Scalar::from_bytes_be(&[
            1, 104, 131, 152, 164, 85, 161, 208, 61, 231, 140, 132, 127, 77, 195, 102, 31, 254,
            194, 28, 121, 72, 125, 117, 198, 153, 89, 110, 205, 196, 144, 132,
        ])
        .unwrap();
        // let scalar = Scalar::from(129u64);
        let expected = G1Projective::generator() * scalar;
        let now = std::time::Instant::now();
        let got = table.scalar_mul(scalar);
        println!("Time to compute scalar mul: {:?}", now.elapsed());
        assert_eq!(expected, got);
        // for i in 0..10_000 {
        // }
    }

    #[test]
    fn check_msm() {
        let length = 4;
        let generators : Vec<_>= (0..length).map(|_| G1Projective::random(&mut rand::thread_rng())).collect();
        let scalars : Vec<_>= (0..length).map(|_| Scalar::random(&mut thread_rng())).collect();
        let now = std::time::Instant::now();
        let expected = g1_lincomb(&generators, &scalars);
        println!("Time to compute scalar mul with g1 lincomb: {:?}", now.elapsed());

        let mut table = PrecompTableNew::new(generators, 8);
        // let scalar = Scalar::from_bytes_be(&[
        //     1, 104, 131, 152, 164, 85, 161, 208, 61, 231, 140, 132, 127, 77, 195, 102, 31, 254,
        //     194, 28, 121, 72, 125, 117, 198, 153, 89, 110, 205, 196, 144, 132,
        // ])
        // .unwrap();
        // let scalar_a = Scalar::from(1u64);
        // let scalar_b = Scalar::from(2u64);
        // let expected = G1Projective::generator() * scalar_a + G1Projective::generator() * scalar_b;
        // let now = std::time::Instant::now();
        let now = std::time::Instant::now();
        let got = table.msm(scalars);
        println!("Time to compute scalar mul: {:?}", now.elapsed());
        assert_eq!(expected, got);
        // for i in 0..10_000 {
        // }
    }

}
