use crate::{G1Projective, Scalar};
use ark_ff::BigInteger;
use blstrs::G1Affine;
use group::Group;

pub struct PrecomputedTable {
    base: usize,
    // The first row signifies the bottom row, ie points of the form
    // G_i, 2G_i, 3_G_i, ..., 2^base-1 G_i
    pub bases: Vec<G1Affine>,

    // Length of each row
    row_length: usize,
}

impl PrecomputedTable {
    pub fn new(gen: G1Projective, base: usize) -> Self {
        const NUM_BITS_SCALAR_NEAREST_POW_2: usize = 256;
        let num_rows =
            (NUM_BITS_SCALAR_NEAREST_POW_2 / base) + NUM_BITS_SCALAR_NEAREST_POW_2 % base;

        // Rows will contain -2G_i, -G_i, G_i, 2G_i,..., 2^base-2 G_i
        // ie they will contain half the number of points as the base
        // The scalar range will then be -2^(base-1) to 2^(base-1) - 1
        // instead of 0 to 2^base - 1
        let length_of_row = ((1 << base) - 1) / 2;

        let bottom_row = Self::compute_sequential_powers_of_point(gen.into(), base - 1);

        let mut rows = vec![bottom_row];

        // Start from one since we computed the bottom row
        for _ in 1..num_rows {
            let mut row = Vec::with_capacity(length_of_row);
            // To compute the next row, we take the last row and multiply every point by 2^base
            for i in 0..length_of_row {
                let mut point: G1Projective = rows.last().unwrap()[i].into();
                // Double the previous row base times to get 2^base * previous point
                for _ in 0..base - 1 {
                    point = point.double();
                }
                let point: G1Affine = point.into();
                row.push(point);
            }
            rows.push(row);
        }
        PrecomputedTable {
            bases: rows.into_iter().flatten().collect(),
            base,
            row_length: length_of_row,
        }
    }

    pub fn new_par(gen: G1Projective, base: usize) -> Self {
        const NUM_BITS_SCALAR_NEAREST_POW_2: usize = 256;
        let num_rows =
            (NUM_BITS_SCALAR_NEAREST_POW_2 / base) + NUM_BITS_SCALAR_NEAREST_POW_2 % base;

        // Rows will contain -2G_i, -G_i, G_i, 2G_i,..., 2^base-2 G_i
        // ie they will contain half the number of points as the base
        // The scalar range will then be -2^(base-1) to 2^(base-1) - 1
        // instead of 0 to 2^base - 1
        let length_of_row = ((1 << base) - 1) / 2;
        // The start of each column will be G, 2^base G, 2^(2*base) G, ...
        let mut column_elements_proj = vec![gen];
        for _ in 1..num_rows {
            let mut cur = *column_elements_proj.last().unwrap();

            // Compute 2^base * previous point
            for _ in 0..base - 1 {
                cur = cur.double();
            }
            column_elements_proj.push(cur);
        }

        use group::prime::PrimeCurveAffine;
        use group::Curve;
        let mut column_elements = vec![G1Affine::identity(); column_elements_proj.len()];
        G1Projective::batch_normalize(&column_elements_proj, &mut column_elements);

        use rayon::prelude::*;

        let rows: Vec<_> = column_elements
            .into_par_iter()
            // Minus one from base since we want the row length to be half the base,
            // due to the signed digit representation
            .map(|row_generator| Self::compute_sequential_powers_of_point(row_generator, base - 1))
            .collect();

        for row in &rows {
            assert_eq!(row.len(), length_of_row);
        }

        PrecomputedTable {
            bases: rows.into_iter().flatten().collect(),
            base: base,
            row_length: length_of_row,
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
        let now = std::time::Instant::now();
        let bigint = convert_scalar_to_arkworks_bigint(&scalar);
        let scalar_bytes = make_digits(&bigint, self.base - 1, Scalar::NUM_BITS as usize);
        dbg!(scalar_bytes.collect::<Vec<i64>>());
        let scalar_bytes = make_digits(&bigint, self.base - 1, Scalar::NUM_BITS as usize);
        println!("Time to convert scalar: {:?}", now.elapsed());
        // Iterate over the precomputed table rows and scalar bytes simultaneously
        let mut result: G1Projective = G1Projective::identity();
        for (row_index, byte) in scalar_bytes.enumerate().filter(|(_, digit)| *digit != 0) {
            // Add the corresponding precomputed point from the current row
            if byte < 0 {
                let now = std::time::Instant::now();
                let tmp = self.bases[(-byte) as usize - 1 + row_index * self.row_length];
                println!("Time to get point: {:?}", now.elapsed());
                let now = std::time::Instant::now();
                result -= tmp;
                println!("Time to add point: {:?}", now.elapsed());
            } else {
                // Minus one because we skip the zero window
                let now = std::time::Instant::now();
                let tmp = self.bases[(byte as usize) - 1 + row_index * self.row_length];
                println!("Time to get point: {:?}", now.elapsed());
                let now = std::time::Instant::now();
                result += tmp;
                println!("Time to add point: {:?}", now.elapsed());
            }
        }

        result
    }

    pub fn scalar_mul_batch_addition(&self, scalar: Scalar) -> G1Projective {
        use ff::PrimeField;
        let bigint = convert_scalar_to_arkworks_bigint(&scalar);
        let scalar_bytes = make_digits(&bigint, self.base - 1, Scalar::NUM_BITS as usize);

        // Iterate over the precomputed table rows and scalar bytes simultaneously
        let points_to_add: Vec<_> = scalar_bytes
            .enumerate()
            .filter(|(_, digit)| *digit != 0)
            .map(|(row_index, digit)| {
                // Add the corresponding precomputed point from the current row
                if digit < 0 {
                    // Minus one because we skip the zero window
                    -self.bases[(-digit) as usize - 1 + row_index * self.row_length]
                } else {
                    self.bases[(digit) as usize - 1 + row_index * self.row_length]
                }
            })
            .collect();
        crate::batch_point_addition::batch_addition(points_to_add).into()
    }
}

fn convert_scalar_to_arkworks_bigint(scalar: &Scalar) -> ark_ff::BigInteger256 {
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
fn make_digits(a: &impl BigInteger, w: usize, num_bits: usize) -> impl Iterator<Item = i64> + '_ {
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
    use crate::{precomp_table::PrecomputedTable, G1Projective, Scalar};
    use blstrs::G1Affine;
    use ff::Field;
    use group::Group;

    #[test]
    fn smoke_test_precomp_table() {
        let table = PrecomputedTable::new(G1Projective::generator(), 9);
        for i in 0..10_000 {
            let scalar = Scalar::random(&mut rand::thread_rng());
            dbg!(scalar.to_bytes_be());
            let expected = G1Projective::generator() * scalar;
            let got = table.scalar_mul(scalar);
            assert_eq!(expected, got);
        }
    }

    #[test]
    fn check_bug() {
        let table = PrecomputedTable::new(G1Projective::generator(), 9);
        // let scalar = Scalar::from_bytes_be(&[
        //     1, 104, 131, 152, 164, 85, 161, 208, 61, 231, 140, 132, 127, 77, 195, 102, 31, 254,
        //     194, 28, 121, 72, 125, 117, 198, 153, 89, 110, 205, 196, 144, 132,
        // ]).unwrap();
        let scalar = Scalar::from(128u64);
        let expected = G1Projective::generator() * scalar;
        let got = table.scalar_mul(scalar);
        assert_eq!(expected, got);
        for i in 0..10_000 {}
    }

    #[test]
    fn test_compute_sequential_powers_of_g() {
        let base_exponent = 8;
        use group::prime::PrimeCurveAffine;
        let generator = G1Affine::generator();
        let got = PrecomputedTable::compute_sequential_powers_of_point(generator, base_exponent);

        let expected: Vec<G1Affine> = (1..(1 << base_exponent))
            .map(|i| (generator * Scalar::from(i)).into())
            .collect::<Vec<_>>();

        assert_eq!(expected.len(), got.len());
        assert_eq!(expected, got);
    }

    #[test]
    fn test_create_parallel_table() {
        let table = PrecomputedTable::new(G1Projective::generator(), 9);

        let par_table = PrecomputedTable::new_par(G1Projective::generator(), 9);

        assert_eq!(table.bases.len(), par_table.bases.len());
        assert_eq!(table.bases, par_table.bases);
        assert_eq!(table.bases, par_table.bases);
    }
}
