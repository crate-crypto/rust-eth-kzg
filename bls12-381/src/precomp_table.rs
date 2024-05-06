use crate::{G1Projective, Scalar};
use ark_ff::BigInteger;
use blstrs::G1Affine;
use group::Group;

pub struct PrecomputedTable {
    base: usize,
    // The first row signifies the bottom row, ie points of the form
    // G_i, 2G_i, 3_G_i, ..., 2^base-1 G_i
    pub bases: Vec<Vec<G1Affine>>,
}

impl PrecomputedTable {
    pub fn new(gen: G1Projective, base: usize) -> Self {
        // NUM_BITS / base_bits
        let num_rows = (256 / base) + 256 % base;

        // let num_rows = (1 << Scalar::NUM_BITS) / (1 << base);
        // Rows will contain G_i, 2G_i,..., 2^base-1 G_i
        let length_of_row = ((1 << base) - 1) / 2;
        // The bottom row goes from 1 to 2^base-1
        let bottom_row: Vec<G1Affine> = (1..(1 << base))
            .map(|i| (gen * Scalar::from(i)).into())
            .collect();

        let mut rows = vec![bottom_row];

        // Start from one since we computed the bottom row
        for _ in 1..num_rows {
            let mut row = Vec::with_capacity(length_of_row);
            // To compute the next row, we take the last row and multiply every point by 2^base
            for i in 0..length_of_row {
                let mut point: G1Projective = rows.last().unwrap()[i].into();
                // double the previous row base times to get 2^base * previous point
                for _ in 0..base {
                    point = point.double();
                }
                let point: G1Affine = point.into();
                row.push(point);
            }
            rows.push(row);
        }
        PrecomputedTable { bases: rows, base }
    }

    pub fn scalar_mul(&self, scalar: Scalar) -> G1Projective {
        // Recode scalars
        use ff::PrimeField;
        let bigint = convert_scalar_to_arkworks_bigint(&scalar);
        let scalar_bytes =
            make_digits(&bigint, self.base, Scalar::NUM_BITS as usize);

        // Iterate over the precomputed table rows and scalar bytes simultaneously
        let mut result: G1Projective = G1Projective::identity();
        for (row, byte) in self.bases.iter().zip(scalar_bytes).filter(|(_, digit)| *digit != 0) {
            // Add the corresponding precomputed point from the current row
            if byte < 0 {
                result -= row[(-byte) as usize - 1];
            } else {
                // Minus one because we skip the zero window
                result += row[(byte as usize) - 1];
            }
        }

        result
    }

    pub fn scalar_mul_batch_addition(&self, scalar: Scalar) -> G1Projective {
        use ff::PrimeField;
        let bigint = convert_scalar_to_arkworks_bigint(&scalar);
        let scalar_bytes =
            make_digits(&bigint, self.base, Scalar::NUM_BITS as usize);
    
        // Iterate over the precomputed table rows and scalar bytes simultaneously
        let points_to_add: Vec<_> = self
            .bases
            .iter()
            .zip(scalar_bytes)
            .filter(|(_, digit)| *digit != 0)
            .map(|(row, digit)| {
                // Add the corresponding precomputed point from the current row
                if digit < 0 {
                    // Minus one because we skip the zero window
                    -row[(-digit) as usize - 1]
                } else {
                    row[(digit) as usize - 1]
                }
            })
            .collect();
        let res = crate::batch_point_addition::batch_addition(points_to_add).into();
        return res;
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
    use ff::Field;
    use group::Group;

    #[test]
    fn smoke_test_precomp_table() {
        let table = PrecomputedTable::new(G1Projective::generator(), 9);

        let scalar = Scalar::random(&mut rand::thread_rng());
        let expected = G1Projective::generator() * scalar;
        let got = table.scalar_mul(scalar);
        assert_eq!(expected, got);
    }
}
