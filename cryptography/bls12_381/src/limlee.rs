use blstrs::{G1Affine, G1Projective, Scalar};
use ff::{Field, PrimeField};
use group::Group;

use crate::g1_batch_normalize;

// Reference: http://mhutter.org/papers/Mohammed2012ImprovedFixedBase.pdf
//
// For now I will use the variables used in the paper, and then we can
// rename them to be more descriptive.
#[derive(Debug, Clone)]
pub struct LimLee {
    l: u32,
    // For a scalar with `l` bits,
    // We choose a splitting parameter `h` such that
    // the `l` bits of the scalar is split into `a = l/h` bits
    //
    h: u32,
    // The scalars bits are grouped into `a` bit-groups of size `a`
    a: u32,
    // For the `bit-groups of size `a`,
    // We choose a splitting parameter `v` such that
    // the `a` bits are split into `b = a / v` bits
    v: u32,
    //
    b: u32,
    //
    points: Vec<G1Affine>,
}

impl LimLee {
    pub fn new(h: u32, v: u32) -> LimLee {
        // Compute `a`.

        // TODO: Add one so that we view all scalars as 256 bit numbers.
        // We can modify it to view everything as 255 bits with a tiny bit of refactoring
        // when we pad the decomposed scalar
        let l = Self::compute_padded_scalar_bits(Scalar::NUM_BITS + 1, h);
        // First of all check that h < l
        assert!(h < l);
        let a = l.div_ceil(h);

        assert!(v <= a);
        assert!(
            a % v == 0,
            "v must be a factor of a, so that b can be equally sized"
        );
        // Compute `b`
        let b = a.div_ceil(v);

        LimLee {
            h,
            a,
            v,
            b,
            points: Vec::new(),
            l,
        }
    }

    // we want to compute scalar_size / divider but pad by zeroes
    // if the scalar_size does not divide 'divider'
    //
    // This method returns the padded size of the scalar
    fn compute_padded_scalar_bits(scalar_size: u32, divider: u32) -> u32 {
        scalar_size.div_ceil(divider) * divider
    }

    // This corresponds to the naive sum in 3.1 where there is no pre-computation
    // and P is the generator
    pub fn scalar_mul_naive(&self, scalar: Scalar) -> G1Projective {
        dbg!(&self);
        let mut scalar_bits = scalar_to_bits(scalar).to_vec();

        // Pad the scalar, if the value of `l` necesitates it
        scalar_bits.extend(vec![0u8; self.l as usize - scalar_bits.len()]); // 256 here because we convert to bytes and then bits

        // Group the scalar bits into `a` chunks
        assert!(scalar_bits.len() as u32 % self.b == 0);
        let mut b_chunks: Vec<_> = scalar_bits.chunks_exact(self.b as usize).collect();
        let scalar_bits: Vec<_> = b_chunks.into_iter().map(|b| bits_to_byte(b)).collect();

        // For the columns
        let mut result = G1Projective::identity();

        for j in 0..self.v {
            for i in 0..self.h {
                // We use a flat array, but the algorithm
                // is based off of a matrix, so compute the flattened index
                let index = i * self.v + j;
                let digit = scalar_bits[index as usize];

                let exponent = j * self.b + i * self.a;
                let mut tmp = G1Projective::generator();
                for _ in 0..exponent {
                    tmp = tmp.double();
                }
                result += tmp * Scalar::from(digit as u64);
            }
        }

        result
    }

    // This corresponds to equation 3 on page 347
    pub fn scalar_mul_eq3(&self, scalar: Scalar) -> G1Projective {
        let mut scalar_bits = scalar_to_bits(scalar).to_vec();

        // Pad the scalar, if the value of `l` necessitates it
        scalar_bits.extend(vec![0u8; self.l as usize - 256]); // 256 here because we convert to bytes and then bits

        // Group the scalar bits into `a` chunks
        assert!(scalar_bits.len() as u32 % self.b == 0);
        let mut b_chunks: Vec<_> = scalar_bits.chunks_exact(self.b as usize).collect();
        let scalar_bits: Vec<_> = b_chunks.into_iter().map(|b| bits_to_byte(b)).collect();

        // Precomputations
        let mut precomputations = Vec::new();
        precomputations.push(G1Projective::generator());
        for i in 0..self.h {
            let two_pow_a = Scalar::from(2u64).pow(&[self.a as u64]);
            precomputations.push(precomputations.last().unwrap() * two_pow_a);
        }

        // For the columns
        let mut result = G1Projective::identity();

        for j in 0..self.v {
            for i in 0..self.h {
                // We use a flat array, but the algorithm
                // is based off of a matrix, so compute the flattened index
                let index = i * self.v + j;
                let digit = scalar_bits[index as usize];

                let exponent = j * self.b;
                let mut tmp = precomputations[i as usize];
                for _ in 0..exponent {
                    tmp = tmp.double();
                }
                result += tmp * Scalar::from(digit as u64);
            }
        }

        result
    }

    // This corresponds to eq4 on page 347
    pub fn scalar_mul_eq4(&self, scalar: Scalar) -> G1Projective {
        let mut scalar_bits = scalar_to_bits(scalar).to_vec();

        // Pad the scalar, if the value of `l` necessitates it
        scalar_bits.extend(vec![0u8; self.l as usize - 256]); // 256 here because we convert to bytes and then bits

        // Precomputations
        let mut precomputations = Vec::new();
        precomputations.push(G1Projective::generator());
        for i in 0..self.h {
            let two_pow_a = Scalar::from(2u64).pow(&[self.a as u64]);
            precomputations.push(precomputations.last().unwrap() * two_pow_a);
        }

        let mut result = G1Projective::identity();
        // For the columns

        for t in 0..self.b {
            let mut double_inner_sum = G1Projective::identity();
            for j in 0..self.v {
                for i in 0..self.h {
                    // We use a flat array, but the algorithm
                    // is based off of a matrix, so compute the flattened index
                    let index = i * self.v * self.b + j * self.b + t;
                    let digit = scalar_bits[index as usize];

                    let exponent = j * self.b;
                    let mut tmp = precomputations[i as usize];
                    for _ in 0..exponent {
                        tmp = tmp.double();
                    }
                    double_inner_sum += tmp * Scalar::from(digit as u64);
                }
            }

            for _ in 0..t {
                double_inner_sum = double_inner_sum.double()
            }
            result += double_inner_sum;
        }

        result
    }

    // This corresponds to eq5 on page 347
    pub fn scalar_mul_eq5(&self, scalar: Scalar) -> G1Projective {
        let mut scalar_bits = scalar_to_bits(scalar).to_vec();

        // Pad the scalar, if the value of `l` necessitates it
        scalar_bits.extend(vec![0u8; self.l as usize - 256]); // 256 here because we convert to bytes and then bits

        // Precomputations
        let mut precomputations = Vec::new();
        precomputations.push(G1Projective::generator());
        for i in 0..self.h {
            let two_pow_a = Scalar::from(2u64).pow(&[self.a as u64]);
            precomputations.push(precomputations.last().unwrap() * two_pow_a);
        }

        let mut g_s =
            vec![vec![G1Projective::identity(); (1 << self.h) as usize]; (self.v as usize)];

        // Initialize the j==0 case
        // Compute G[0][s] for all s
        for s in 1..(1 << self.h) {
            let mut g0s = G1Projective::identity();
            for i in 0..self.h {
                if (s & (1 << i)) != 0 {
                    g0s += precomputations[i as usize];
                }
            }
            g_s[0][s] = g0s;
        }

        // Compute G[j][s] for j > 0
        let two_pow_b = Scalar::from(2u64).pow(&[self.b as u64]);
        for j in 1..self.v as usize {
            for s in 1..(1 << self.h) as usize {
                g_s[j][s] = g_s[j - 1][s] * two_pow_b;
            }
        }

        let g_s: Vec<_> = g_s
            .into_iter()
            .map(|g_s_i| g1_batch_normalize(&g_s_i))
            .collect();

        let mut result = G1Projective::identity();

        for t in 0..self.b {
            let mut double_inner_sum = G1Projective::identity();
            for j in 0..self.v {
                let i_jt = self.compute_i_jt(&scalar_bits, j, t);
                double_inner_sum += g_s[j as usize][i_jt];
            }

            for _ in 0..t {
                double_inner_sum = double_inner_sum.double()
            }
            result += double_inner_sum;
        }
        result
    }

    fn compute_i_jt(&self, k: &[u8], j: u32, t: u32) -> usize {
        let mut i_jt = 0;
        for i in 0..self.h {
            let bit_index = (i * self.v * self.b + j * self.b + t) as usize;
            if bit_index < k.len() && (k[bit_index] == 1) {
                i_jt |= 1 << i;
            }
        }
        i_jt as usize
    }
}

#[test]
fn smoke_test_generator_scalar_mul() {
    let ll = LimLee::new(8, 8);
    dbg!(&ll);
    let scalar = -Scalar::from(2u64);

    let expected = G1Projective::generator() * scalar;

    let result = ll.scalar_mul_naive(scalar);
    assert!(result == expected);

    let got = ll.scalar_mul_eq3(scalar);
    assert_eq!(got, result);

    let got = ll.scalar_mul_eq4(scalar);
    assert_eq!(got, result);

    let got = ll.scalar_mul_eq5(scalar);
    assert_eq!(got, result)
}

fn scalar_to_bits(s: Scalar) -> [u8; 256] {
    let scalar_bytes = s.to_bytes_le();
    bytes_to_bits(scalar_bytes)
}
fn bytes_to_bits(bytes: [u8; 32]) -> [u8; 256] {
    let mut bit_vector = Vec::with_capacity(256);
    for byte in bytes {
        for i in 0..8 {
            bit_vector.push(((byte >> i) & 0x01) as u8)
        }
    }
    bit_vector.try_into().unwrap()
}

fn bits_to_byte(bits: &[u8]) -> u8 {
    assert!(
        bits.len() <= 8,
        "currently we are returning a u8, so can only do 8 bits."
    );
    bits.iter()
        .rev()
        .fold(0, |acc, &bit| (acc << 1) | (bit & 1))
}

#[test]
fn compute_padded_scalar() {
    struct TestCase {
        scalar_size: u32,
        divider: u32,
        expected: u32,
    }

    let cases = vec![
        // TODO: remove this and generalize
        TestCase {
            scalar_size: 255,
            divider: 4,
            expected: 256,
        },
        TestCase {
            scalar_size: 256,
            divider: 4,
            expected: 256,
        },
        TestCase {
            scalar_size: 100,
            divider: 3,
            expected: 102,
        },
    ];

    for case in cases {
        let got = LimLee::compute_padded_scalar_bits(case.scalar_size, case.divider);
        assert_eq!(got, case.expected)
    }
}