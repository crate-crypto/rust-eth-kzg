use blstrs::{G1Affine, G1Projective, Scalar};
use ff::{Field, PrimeField};
use group::Group;

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

        let l = Self::compute_padded_scalar_bits(Scalar::NUM_BITS, h);
        // First of all check that h < l
        assert!(h < l);
        let a = l / h;

        assert!(v <= a);
        // Compute `b`
        let b = a / v;

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

    pub fn scalar_mul(&self, scalar: Scalar) -> G1Projective {
        let mut scalar_bits = scalar_to_bits(scalar).to_vec();

        // Pad the scalar, if the value of `l` necesitates it
        scalar_bits.extend(vec![0u8; self.l as usize - 256]); // 256 here because we convert to bytes and then bits

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
                let bit = scalar_bits[index as usize];

                let exponent = j * self.b + i * self.a;
                let mut tmp = G1Projective::generator();
                for _ in 0..exponent {
                    tmp = tmp.double();
                }
                result += tmp * Scalar::from(bit as u64);
            }
        }

        result
    }
}

#[test]
fn smoke_test_generator_scalar_mul() {
    let ll = LimLee::new(32, 8);
    let result = ll.scalar_mul(Scalar::from(2u64));
    assert!(result == G1Projective::generator().double());
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
