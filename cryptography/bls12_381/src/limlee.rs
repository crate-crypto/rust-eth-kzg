use core::num;

use blstrs::{Fp, G1Affine, G1Projective, Scalar};
use ff::{Field, PrimeField};
use group::{prime::PrimeCurveAffine, Group, WnafScalar};

use crate::{
    batch_add::{batch_addition, multi_batch_addition, multi_batch_addition_diff_stride},
    g1_batch_normalize,
    wnaf::wnaf_form,
};

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
    precomputed_points: Vec<Vec<Vec<G1Affine>>>,
}

impl LimLee {
    pub fn new(h: u32, v: u32, points: &[G1Affine]) -> LimLee {
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
            "v must be a factor of a, so that b can be equally sized v={v}, a={a}",
        );
        // Compute `b`
        let b = a.div_ceil(v);

        let mut ll = LimLee {
            h,
            a,
            v,
            b,
            precomputed_points: Vec::new(),
            l,
        };
        use rayon::prelude::*;
        let precomputed = points
            .into_par_iter()
            .map(|point| ll.precompute_point(*point))
            .collect();
        ll.precomputed_points = precomputed;

        ll
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

        let mut total_len = 0;
        for g in &g_s {
            total_len += g.len()
        }
        dbg!(total_len);

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

    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        // Convert scalars to bits
        // let now = std::time::Instant::now();
        let scalars_bits: Vec<_> = scalars
            .into_iter()
            .map(|scalar| {
                let mut scalar_bits = scalar_to_bits(*scalar).to_vec();
                scalar_bits.extend(vec![0u8; self.l as usize - scalar_bits.len()]);
                scalar_bits
            })
            .collect();
        // dbg!("scalar conversion", now.elapsed().as_micros());
        let mut window: Vec<Vec<G1Affine>> = vec![vec![]; self.b as usize];

        // let now = std::time::Instant::now();
        for (scalar_index, scalar_bits) in scalars_bits.iter().enumerate() {
            for t in (0..self.b) {
                for j in 0..self.v {
                    let i_jt = self.compute_i_jt(&scalar_bits, j, t);
                    if i_jt != 0 {
                        window[t as usize]
                            .push(self.precomputed_points[scalar_index][j as usize][i_jt]);
                    }
                }
            }
        }

        let mut result = G1Projective::identity();
        let summed_windows = multi_batch_addition_diff_stride(window);

        for (window) in summed_windows.into_iter().rev() {
            result = result.double();
            result += window;
        }

        // dbg!(now.elapsed().as_micros());
        result
    }

    fn num_precomputed_points(&self) -> usize {
        let mut total = 0;
        for set_of_points in &self.precomputed_points {
            for row in set_of_points {
                total += row.len();
            }
        }
        total
    }

    fn precompute_point(&self, point: G1Affine) -> Vec<Vec<G1Affine>> {
        let point = G1Projective::from(point);
        // Precomputations
        let mut precomputations = Vec::new();
        precomputations.push(point);
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
        g_s
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

type PrecomputedPoints = Vec<Vec<G1Affine>>;

#[derive(Debug)]
pub struct TsaurChou {
    // These are not the same as LimLee
    //
    //
    omega: usize,
    v: usize,
    a: usize,
    b: usize,
    num_bits: usize,

    precomputed_points: Vec<PrecomputedPoints>,
}

impl TsaurChou {
    pub fn new(omega: usize, v: usize, points: &[G1Affine]) -> TsaurChou {
        let num_bits = Scalar::NUM_BITS + 1;

        // This is the padded number of bits needed to make sure division
        // by omega is exact.
        let num_bits = Self::calculate_padded_size(num_bits as usize, omega);

        let a = num_bits / omega;

        // assert!(a % v == 0, "a={} v={}", a, v);
        let b = a.div_ceil(v);

        let mut precomputed_points = Vec::new();
        for point in points {
            precomputed_points.push(Self::precompute_point(*point, omega, b, v))
        }

        Self {
            omega,
            v,
            a,
            b,
            num_bits,
            precomputed_points,
        }
    }

    fn calculate_padded_size(l: usize, w: usize) -> usize {
        let a = (l + w - 1) / w; // This is ⌈l/ω⌉
        let padded_size = a * w;
        // TODO: if statement not needed, if we do div_ceil
        let padding_zeros = if l % w == 0 { 0 } else { padded_size - l };
        padding_zeros + l
    }

    fn num_precomputed_points(&self) -> usize {
        let mut result = 0;
        for points in &self.precomputed_points {
            for p in points.iter() {
                result += p.len()
            }
        }
        result
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
                let k_jbt = &wnaf_digits[start_index..end_index.min(wnaf_digits.len())];
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
        let point = G1Affine::generator();
        let mut result = G1Projective::identity();
        // TODO: I think we need to pad here after wnaf

        // 1. Compute the precomputations
        // Precomputation
        let precomp = Self::precompute_point(point, self.omega, self.b, self.v);

        let now = std::time::Instant::now();

        let mut windows = vec![vec![]; self.b];
        // 2. iterate `w` bits and compute the scalar_mul
        for t in 0..self.b {
            for j in 0..self.v {
                let start_index = (j * self.b + t) * self.omega;
                let end_index = start_index + self.omega;
                let k_jbt = &wnaf_digits[start_index..end_index];

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
                    let mut chosen_point = precomp[j]
                        [Self::sd_to_index(s_exponent, abs_digit as usize, self.omega as u32)];
                    if digit < 0 {
                        chosen_point = -chosen_point;
                    }
                    windows[t].push(chosen_point);
                }
            }
        }

        // Combine each sum in each window
        let windows: Vec<_> = windows
            .into_iter()
            .map(|window| batch_addition(window))
            .collect();

        // Combine windows
        // for (t, window) in windows.into_iter().enumerate() {
        //     if t * self.omega == 0 {
        //         result += window
        //     } else if t * self.omega == 1 {
        //         result += G1Projective::from(window).double();
        //     } else {
        //         // let inner_sum: G1Affine = window.into();

        //         let inner_sum = direct_doubling(t * self.omega, window);

        //         result += inner_sum;
        //     }
        // }

        for window in windows.into_iter().rev() {
            for _ in 0..self.omega {
                result = result.double()
            }

            result += window;
        }

        dbg!(now.elapsed().as_micros());

        result
    }

    // This is closer to the cleaned up version that does not have
    // the precomps being done internally.
    //
    // These are computed in the constructor
    pub fn mul_naive_better_wnaf_precomputations_final_msm(
        &self,
        scalars: &[Scalar],
    ) -> G1Projective {
        fn scalar_to_wnaf(scalar: Scalar, num_bits: usize, omega: usize) -> Vec<i64> {
            let mut wnaf_digits = vec![];
            let mut scalar_bytes = scalar.to_bytes_le();
            // scalar_bytes.extend(vec![0u8; num_bits / 8 + 1 - scalar_bytes.len()]); // TODO: double check for rounding error
            wnaf_form(&mut wnaf_digits, scalar_bytes, omega);
            wnaf_digits
        }
        // let now = std::time::Instant::now();
        let scalars_wnaf_digits: Vec<_> = scalars
            .into_iter()
            .map(|scalar| scalar_to_wnaf(*scalar, self.num_bits, self.omega))
            .collect();
        // dbg!(now.elapsed().as_micros());
        // let wnaf_digits = scalar_to_wnaf(scalars[0], self.num_bits, self.omega);
        // Convert scalar to wnaf
        // let wnaf_digits = scalar_to_bits(*scalar);
        let mut result = G1Projective::identity();

        // let now = std::time::Instant::now();

        let mut windows = vec![vec![]; self.b];
        // 2. iterate `w` bits and compute the scalar_mul
        for t in 0..self.b {
            for j in 0..self.v {
                for (scalar_index, wnaf_digits) in scalars_wnaf_digits.iter().enumerate() {
                    let start_index = (j * self.b + t) * self.omega;
                    let end_index = start_index + self.omega;
                    if start_index > wnaf_digits.len() {
                        continue;
                    }

                    let k_jbt = &wnaf_digits[start_index..end_index.min(wnaf_digits.len())];

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
                        let mut chosen_point = self.precomputed_points[scalar_index][j]
                            [Self::sd_to_index(s_exponent, abs_digit as usize, self.omega as u32)];
                        if digit < 0 {
                            chosen_point = -chosen_point;
                        }
                        windows[t].push(chosen_point);
                    }
                }
            }
        }

        // Combine each sum in each window
        // let windows: Vec<_> = windows
        //     .into_iter()
        //     .map(|window| batch_addition(window))
        //     .collect();
        // let now = std::time::Instant::now();
        let windows = multi_batch_addition_diff_stride(windows);
        // dbg!(now.elapsed().as_micros());
        // Combine windows
        // for (t, window) in windows.into_iter().enumerate() {
        //     if t * self.omega == 0 {
        //         result += window
        //     } else if t * self.omega == 1 {
        //         result += G1Projective::from(window).double();
        //     } else {
        //         // let inner_sum: G1Affine = window.into();

        //         let inner_sum = direct_doubling(t * self.omega, window);

        //         result += inner_sum;
        //     }
        // }
        for window in windows.into_iter().rev() {
            for _ in 0..self.omega {
                result = result.double()
            }

            result += window;
        }

        // dbg!(now.elapsed().as_micros());

        result
    }

    fn sd_to_index(s_exp: usize, d: usize, w: u32) -> usize {
        s_exp * (1 << (w - 2)) + (d - 1) / 2
    }

    fn precompute_point_old(
        point: G1Affine,
        omega: usize,
        b: usize,
        v: usize,
    ) -> Vec<Vec<G1Affine>> {
        let point = G1Projective::from(point);

        let inner_size = omega * (1 << (omega - 2));
        let mut precomp = vec![vec![G1Projective::identity(); inner_size]; v];

        for s in 0..omega {
            for d in (1..1 << (omega - 1)).step_by(2) {
                let index = s * (1 << (omega - 2)) + (d - 1) / 2;
                let sd = (1 << s) * d;
                precomp[0][index] = point * (&Scalar::from(sd as u64));
            }
        }

        for j in 1..v {
            let factor = Scalar::from(2u64).pow(&[(j * omega * b) as u64]);
            for index in 0..inner_size {
                precomp[j][index] = precomp[0][index] * (&factor);
            }
        }

        let precomp: Vec<_> = precomp
            .into_iter()
            .map(|points| g1_batch_normalize(&points))
            .collect();

        precomp
    }

    fn precompute_point(point: G1Affine, omega: usize, b: usize, v: usize) -> Vec<Vec<G1Affine>> {
        // d in the paper is just odd multiples
        fn precompute_odd_multiples(base: G1Affine, w: usize) -> Vec<G1Projective> {
            let base = G1Projective::from(base);
            let num_points = (1 << (w - 1)) / 2; // (2^(w-1)) / 2 points to compute
            let mut results = vec![G1Projective::identity(); num_points];

            // Compute 2P
            let double_base = base.double();

            // 1P is just the base point
            results[0] = base;

            // Compute odd multiples: 3P, 5P, ..., (2^(w-1) - 1)P
            for i in 1..num_points {
                results[i] = results[i - 1] + double_base;
            }

            results
        }

        // let inner_size = omega * (1 << (omega - 2));
        let mut precomp = Vec::new();

        let d_vec = precompute_odd_multiples(point, omega);
        use rayon::prelude::*;
        // Compute G_0
        let mut inner = Vec::new();
        inner.push(d_vec.clone());
        for s_exp in 1..omega {
            let doubled = inner
                .last()
                .unwrap()
                .par_iter()
                .map(|p| p.double())
                .collect();
            inner.push(doubled)
        }
        precomp.push(inner.into_iter().flatten().collect::<Vec<_>>());

        // Now scale those G_j
        for j in 1..v {
            let mut scaled_inner: Vec<_> = precomp
                .last()
                .unwrap()
                .par_iter()
                .map(|inner| {
                    let mut res = *inner;
                    for _ in 0..omega * b {
                        res = res.double();
                    }
                    res
                })
                .collect();

            precomp.push(scaled_inner.into_iter().collect::<Vec<_>>())
        }

        let precomp: Vec<_> = precomp
            .into_iter()
            .map(|points| g1_batch_normalize(&points))
            .collect();

        precomp
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

fn random_points(num_points: usize) -> Vec<G1Affine> {
    (0..num_points)
        .into_iter()
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect()
}

#[test]
fn tsaur_chau() {
    let ts = TsaurChou::new(5, 26, &[G1Affine::generator()]);
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

    let result = ts.mul_naive_better_wnaf_precomputations_final_msm(&[scalar]);
    assert!(result == expected);
}

#[test]
fn tsaur_chau_msm() {
    let num_points = 64;
    let points = random_points(num_points);
    let ts = TsaurChou::new(5, 7, &points); // (5,7), (5,4), (4,12), (6,3), (8,2), (8,4), (8,1)
    dbg!(ts.num_precomputed_points());

    let scalars: Vec<_> = (0..num_points)
        .into_iter()
        .map(|_| Scalar::random(&mut rand::thread_rng()))
        .collect();

    let mut expected = G1Projective::identity();
    for (scalar, point) in scalars.iter().zip(points.iter()) {
        expected += G1Projective::from(*point) * scalar
    }
    let now = std::time::Instant::now();
    let result = ts.mul_naive_better_wnaf_precomputations_final_msm(&scalars);
    dbg!(now.elapsed().as_micros());
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
    let ll = LimLee::new(8, 8, &[]);
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

#[test]
fn smoke_test_lim_lee_msm() {
    let num_points = 1;
    let points = random_points(num_points);
    let ll = LimLee::new(8, 2, &points); // (8,2), (4,16), (5,4)

    let scalars: Vec<_> = (0..num_points)
        .into_iter()
        .map(|i| Scalar::from(i as u64))
        // .map(|i| Scalar::random(&mut rand::thread_rng()))
        .collect();

    let mut expected = G1Projective::identity();
    for (scalar, point) in scalars.iter().zip(points.iter()) {
        expected += G1Projective::from(*point) * scalar
    }
    let now = std::time::Instant::now();
    let got = ll.msm(&scalars);
    dbg!(now.elapsed().as_micros());
    dbg!(ll.num_precomputed_points());
    assert_eq!(got, expected);
}

pub fn scalar_to_bits(s: Scalar) -> [u8; 256] {
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
