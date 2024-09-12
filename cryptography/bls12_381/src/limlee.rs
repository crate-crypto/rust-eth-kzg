use core::num;

use blstrs::{Fp, G1Affine, G1Projective, Scalar};
use ff::{Field, PrimeField};
use group::{prime::PrimeCurveAffine, Group, WnafScalar};

use crate::{g1_batch_normalize, wnaf::wnaf_form};

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
                if i_jt != 0 {
                    double_inner_sum += g_s[j as usize][i_jt];
                }
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

struct TsaurChou {
    // These are not the same as LimLee
    //
    //
    omega: usize,
    v: usize,
    a: usize,
    b: usize,
    num_bits: usize,
}

impl TsaurChou {
    pub fn new(omega: usize, v: usize) -> TsaurChou {
        let num_bits = Scalar::NUM_BITS + 1;

        // This is the padded number of bits needed to make sure division
        // by omega is exact.
        let num_bits = Self::calculate_padded_size(num_bits as usize, omega);

        let a = num_bits / omega;

        let b = a.div_ceil(v);

        Self {
            omega,
            v,
            a,
            b,
            num_bits,
        }
    }

    fn calculate_padded_size(l: usize, w: usize) -> usize {
        let a = (l + w - 1) / w; // This is ⌈l/ω⌉
        let padded_size = a * w;
        // TODO: if statement not needed, if we do div_ceil
        let padding_zeros = if l % w == 0 { 0 } else { padded_size - l };
        padding_zeros + l
    }

    // On page350, this is the first double summation
    pub fn mul_naive(&self, scalar: &Scalar) -> G1Projective {
        // Convert scalar to wnaf
        // let mut wnaf_digits = vec![];
        // wnaf_form(&mut wnaf_digits, scalar.to_repr(), self.omega);
        let mut wnaf_digits = scalar_to_bits(*scalar).to_vec();
        wnaf_digits.extend(vec![0u8; self.num_bits - wnaf_digits.len()]);
        let point = G1Projective::generator();
        let mut result = G1Projective::identity();
        // TODO: I think we need to pad here after wnaf

        // 1. Compute the precomputations

        // 2. iterate `w` bits and compute the scalar_mul
        for j in 0..self.v {
            for t in 0..self.b {
                // Choose K_jb+t
                let exponent = t * self.omega + j * self.b * self.omega;
                let two_pow_exponent = Scalar::from(2u64).pow(&[exponent as u64]);

                // Index K_jb+t
                let start_index = (j * self.b + t) * self.omega;
                let end_index = start_index + self.omega;
                let k_jbt = &wnaf_digits[start_index..end_index];
                // Convert K_jb+t from NAF to scalar
                let mut digit = Scalar::ZERO;
                for (i, &bit) in k_jbt.iter().enumerate() {
                    if bit > 0 {
                        digit += Scalar::from(bit as u64) * Scalar::from(2u64).pow(&[i as u64]);
                    } else if bit < 0 {
                        digit += -Scalar::from(bit as u64) * Scalar::from(2u64).pow(&[i as u64]);
                    }
                }

                result += point * digit * two_pow_exponent;
            }
        }

        result
    }

    // On page 350, this is the second summation. next to the first one
    // under the matrix. Where we pull out a 2^tw
    pub fn mul_naive_better(&self, scalar: &Scalar) -> G1Projective {
        // Convert scalar to wnaf
        // let mut wnaf_digits = vec![];
        // wnaf_form(&mut wnaf_digits, scalar.to_repr(), self.omega);
        let mut wnaf_digits = scalar_to_bits(*scalar).to_vec();
        wnaf_digits.extend(vec![0u8; self.num_bits - wnaf_digits.len()]);
        let point = G1Projective::generator();
        let mut result = G1Projective::identity();
        // TODO: I think we need to pad here after wnaf

        // 1. Compute the precomputations

        // 2. iterate `w` bits and compute the scalar_mul
        for t in 0..self.b {
            let two_pow_tw = Scalar::from(2u64).pow(&[(t * self.omega) as u64]);
            let mut inner_sum = G1Projective::identity();
            for j in 0..self.v {
                // Choose K_jb+t
                let exponent = j * self.b * self.omega;
                let two_pow_exponent = Scalar::from(2u64).pow(&[exponent as u64]);

                // Index K_jb+t
                let start_index = (j * self.b + t) * self.omega;
                let end_index = start_index + self.omega;
                let k_jbt = &wnaf_digits[start_index..end_index];
                // Convert K_jb+t from NAF to scalar
                let mut digit = Scalar::ZERO;
                for (i, &bit) in k_jbt.iter().enumerate() {
                    if bit > 0 {
                        digit += Scalar::from(bit as u64) * Scalar::from(2u64).pow(&[i as u64]);
                    } else if bit < 0 {
                        digit += -Scalar::from(bit as u64) * Scalar::from(2u64).pow(&[i as u64]);
                    }
                }

                inner_sum += point * digit * two_pow_exponent;
            }

            result += inner_sum * two_pow_tw;
        }

        result
    }

    // This is just the same method but it uses wnaf instead of bits
    pub fn mul_naive_better_wnaf(&self, scalar: &Scalar) -> G1Projective {
        // Convert scalar to wnaf
        let mut wnaf_digits = vec![];
        let mut scalar_bytes = scalar.to_bytes_le().to_vec();
        scalar_bytes.extend(vec![0u8; self.num_bits / 8 + 1 - scalar_bytes.len()]); // TODO: double check for rounding error
        wnaf_form(&mut wnaf_digits, scalar_bytes, self.omega);
        // let wnaf_digits = scalar_to_bits(*scalar);
        let point = G1Projective::generator();
        let mut result = G1Projective::identity();
        // TODO: I think we need to pad here after wnaf

        // 1. Compute the precomputations

        // 2. iterate `w` bits and compute the scalar_mul
        for t in 0..self.b {
            let two_pow_tw = Scalar::from(2u64).pow(&[(t * self.omega) as u64]);
            let mut inner_sum = G1Projective::identity();
            for j in 0..self.v {
                // Choose K_jb+t
                let exponent = j * self.b * self.omega;
                let two_pow_exponent = Scalar::from(2u64).pow(&[exponent as u64]);

                // Index K_jb+t
                let start_index = (j * self.b + t) * self.omega;
                let end_index = start_index + self.omega;
                let k_jbt = &wnaf_digits[start_index..end_index];
                // Convert K_jb+t from NAF to scalar
                let mut digit = Scalar::ZERO;
                for (i, &bit) in k_jbt.iter().enumerate() {
                    if bit > 0 {
                        digit +=
                            Scalar::from(bit.abs() as u64) * Scalar::from(2u64).pow(&[(i) as u64]);
                    } else if bit < 0 {
                        digit +=
                            -Scalar::from(bit.abs() as u64) * Scalar::from(2u64).pow(&[(i) as u64]);
                    }
                }
                inner_sum += point * digit * two_pow_exponent;
            }

            result += inner_sum * two_pow_tw;
        }

        result
    }

    pub fn mul_naive_better_wnaf_precomputations(&self, scalar: &Scalar) -> G1Projective {
        // Convert scalar to wnaf
        let mut wnaf_digits = vec![];
        let mut scalar_bytes = scalar.to_bytes_le().to_vec();
        scalar_bytes.extend(vec![0u8; self.num_bits / 8 + 1 - scalar_bytes.len()]); // TODO: double check for rounding error
        wnaf_form(&mut wnaf_digits, scalar_bytes, self.omega);
        // let wnaf_digits = scalar_to_bits(*scalar);
        let point = G1Projective::generator();
        let mut result = G1Projective::identity();
        // TODO: I think we need to pad here after wnaf

        // 1. Compute the precomputations
        // Precomputation
        let inner_size = self.omega * (1 << (self.omega - 2));
        let mut precomp = vec![vec![G1Projective::identity(); inner_size]; self.v];

        fn sd_to_index(s_exp: usize, d: usize, w: u32) -> usize {
            s_exp * (1 << (w - 2)) + (d - 1) / 2
        }

        // s_exp is the exponent for s
        // for s_exp in 0..self.omega {
        //     let s = Scalar::from(2u64).pow(&[s_exp as u64]);
        //     for d in (1..1 << (self.omega - 1)).step_by(2) {
        //         // Compute sd
        //         let index = sd_to_index(2usize.pow(s_exp as u32), d, self.omega as u32);
        //         precomp[0][index] = point * (s * Scalar::from(d as u64))
        //     }
        // }

        for s in 0..self.omega {
            for d in (1..1 << (self.omega - 1)).step_by(2) {
                let index = s * (1 << (self.omega - 2)) + (d - 1) / 2;
                let sd = (1 << s) * d;
                precomp[0][index] = point * (&Scalar::from(sd as u64));
            }
        }

        // Compute G[j][sd] for j > 0
        // let first_preomp = precomp[0].clone();
        // for j in 1..self.v {
        //     let factor = Scalar::from(2u64).pow(&[(j * self.omega * self.b) as u64]);
        //     let jth_precomp: Vec<_> = first_preomp.iter().map(|point| point * factor).collect();
        //     precomp[j] = jth_precomp;
        // }
        for j in 1..self.v {
            let factor = Scalar::from(2u64).pow(&[(j * self.omega * self.b) as u64]);
            for index in 0..inner_size {
                precomp[j][index] = precomp[0][index] * (&factor);
            }
        }

        let precomp: Vec<_> = precomp
            .into_iter()
            .map(|points| g1_batch_normalize(&points))
            .collect();

        let precomp_size: usize = precomp.iter().map(|pc| pc.len()).sum();
        dbg!(precomp_size);

        let now = std::time::Instant::now();
        // 2. iterate `w` bits and compute the scalar_mul
        for t in 0..self.b {
            // let two_pow_tw = Scalar::from(2u64).pow(&[(t * self.omega) as u64]);
            let mut inner_sum = G1Projective::identity();

            for j in 0..self.v {
                let start_index = (j * self.b + t) * self.omega;
                let end_index = start_index + self.omega;
                let k_jbt = &wnaf_digits[start_index..end_index.min(wnaf_digits.len())]; // TODO: check if min is needed here

                let mut s_exponent = 0;
                let mut digit = 0;

                for (i, &bit) in k_jbt.iter().enumerate() {
                    if bit != 0 {
                        // Use bit shifting for 2^i
                        s_exponent = i;
                        digit = bit;
                        break; // In ω-NAF, only one non-zero digit per window
                    }
                }

                if digit != 0 {
                    let abs_digit = digit.unsigned_abs() as u64;
                    if digit > 0 {
                        inner_sum += precomp[j]
                            [sd_to_index(s_exponent, abs_digit as usize, self.omega as u32)];
                    } else {
                        inner_sum -= precomp[j]
                            [sd_to_index(s_exponent, abs_digit as usize, self.omega as u32)];
                    }
                }
            }

            if t * self.omega == 0 {
                result += inner_sum
            } else if t * self.omega == 1 {
                result += inner_sum.double();
            } else {
                let inner_sum: G1Affine = inner_sum.into();

                let inner_sum = direct_doubling(t * self.omega, inner_sum);

                result += inner_sum;
            }
        }
        dbg!(now.elapsed().as_micros());

        result
    }
}

fn direct_doubling(r: usize, point: G1Affine) -> G1Affine {
    if point.is_identity().into() {
        return G1Affine::identity();
    }

    // The below algorithm assumes r > 0
    // We could simply disallow it and panic
    // I chose to return the the point since 2^0 * P = P
    if r == 0 {
        return point;
    }

    // This is just a optimization, the algorithm, does
    // allow this.
    if r == 1 {
        return G1Projective::from(point).double().into();
    }

    let mut previous_a_i = point.x();
    let mut previous_b_i = point.x().square().mul3();
    let mut previous_c_i = -point.y();
    let mut c_prod = previous_c_i;

    let mut current_a_i = Fp::ZERO;
    let mut current_b_i = Fp::ZERO;
    let mut current_c_i = Fp::ZERO;

    for i in 1..r {
        current_a_i = previous_b_i.square() - previous_a_i.mul8() * previous_c_i.square();
        current_b_i = current_a_i.square().mul3();
        current_c_i = -previous_c_i.square().square().mul8()
            - previous_b_i * (current_a_i - previous_a_i * previous_c_i.square() * Fp::from(4u64));
        c_prod *= current_c_i;

        previous_a_i = current_a_i;
        previous_b_i = current_b_i;
        previous_c_i = current_c_i;
    }

    let a_r = current_a_i;
    let b_r = current_b_i;
    let c_r = current_c_i;

    // TODO: We square the same values etc below multiple times
    // TODO: we could optimize and remove these, see for example c_r.square

    let d_r = a_r.mul3() * Fp::from(4u64) * c_r.square() - b_r.square();

    let mut denom_prod = c_prod;
    let denom = Fp::from(2u64).pow(&[r as u64]) * denom_prod;
    let denom = denom.invert().unwrap();

    let denom_sq = denom.square();
    let denom_cu = denom_sq * denom;

    // Compute x_2r
    let numerator = b_r.square() - c_r.square().mul8() * a_r;
    let x2r = numerator * denom_sq;

    // Compute y_2r
    let numerator = c_r.square().square().mul8() - b_r * d_r;
    let y2r = numerator * denom_cu;

    G1Affine::from_raw_unchecked(x2r, y2r, false)
}

#[test]
fn direct_double() {
    let point = G1Affine::generator();

    for r in 2..10 {
        // let r = 2;
        let expected = (point * Scalar::from(2u64).pow(&[r as u64])).into();

        let got = direct_doubling(r, point);

        assert_eq!(got, expected);
    }
}
#[test]
fn tsaur_chau() {
    let ts = TsaurChou::new(8, 4);
    let scalar = -Scalar::from(1u64);

    let expected = G1Projective::generator() * scalar;

    let result = ts.mul_naive(&scalar);
    assert!(result == expected);
    let result = ts.mul_naive_better(&scalar);
    assert!(result == expected);

    let result = ts.mul_naive_better_wnaf(&scalar);
    assert!(result == expected);

    let result = ts.mul_naive_better_wnaf_precomputations(&scalar);
    assert!(result == expected);
}

#[test]
fn wnaf_smoke_test() {
    let s = Scalar::from(1065142573068u64);
    let mut wnaf = vec![];
    // let mut wnaf_digits = vec![];
    let mut scalar_bytes = s.to_bytes_le().to_vec();
    scalar_bytes.extend(vec![0u8; 258 / 8 + 1 - scalar_bytes.len()]);
    // wnaf_form(&mut wnaf_digits, scalar_bytes, self.omega);
    wnaf_form(&mut wnaf, scalar_bytes, 3);

    dbg!(wnaf.chunks_exact(3).collect::<Vec<_>>());

    let mut result = Scalar::ZERO;
    for (i, digit) in wnaf.into_iter().enumerate() {
        if digit > 0 {
            result += Scalar::from(digit.abs() as u64) * Scalar::from(2u64).pow(&[(i) as u64]);
        } else if digit < 0 {
            result += -Scalar::from(digit.abs() as u64) * Scalar::from(2u64).pow(&[(i) as u64]);
        }
    }
    assert_eq!(result, s);
}

#[test]
fn smoke_test_generator_scalar_mul() {
    let ll = LimLee::new(8, 8);
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
