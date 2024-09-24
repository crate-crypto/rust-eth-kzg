// Implements https://www.mdpi.com/1424-8220/13/7/9483

use crate::batch_add::multi_batch_addition_diff_stride;
use crate::g1_batch_normalize;
use crate::limlee::scalar_to_bits;
use crate::wnaf::wnaf_form;
use blstrs::G1Affine;
use blstrs::{G1Projective, Scalar};
use ff::Field;
use ff::PrimeField;
use group::prime::PrimeCurveAffine;
use group::Group;
use rayon::prelude::*;

pub struct SeoKim {
    w: usize,
    a: usize,
    l: usize,
    z: usize,

    precomputed_points: Vec<PrecomputationsForPoint>,
}

// Precomputations needed for a single point
type PrecomputationsForPoint = Vec<Vec<G1Affine>>;

impl SeoKim {
    pub fn new(omega: usize, points: &[G1Affine]) -> Self {
        let num_bits = Self::calculate_padded_size((Scalar::NUM_BITS + 1) as usize, omega * omega);
        let a = num_bits.div_ceil(omega);

        let z = a.div_ceil(omega);

        // let mut precomputed_points: Vec<PrecomputationsForPoint> = Vec::new();
        // for point in points {
        //     precomputed_points.push(Self::precompute_point(*point, omega, z));
        // }

        let mut precomputed_points: Vec<PrecomputationsForPoint> = points
            .into_par_iter()
            .map(|point| Self::precompute_point(*point, omega, z))
            .collect();

        Self {
            w: omega,
            a: a as usize,
            l: num_bits,
            z,
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

    fn scalar_mul_naive(&self, scalar: &Scalar) -> G1Projective {
        // Convert scalar to bits and pad it to the necessary length
        let mut wnaf_digits = scalar_to_bits(*scalar).to_vec();
        wnaf_digits.extend(vec![0u8; self.l - wnaf_digits.len()]);

        let point = G1Projective::generator();

        let mut result = G1Projective::identity();

        for t in 0..self.z {
            // t is used to scan a square
            let square_offset = t * self.w * self.w;
            for i in 0..self.w {
                // i is used to scan a particular row
                //
                //

                // Collect all of the necessary bits that differ by a factor of omega
                let digits = select_elements(&wnaf_digits, self.w as usize, t as usize, i as usize);
                // I need to figure out the bit position for this
                for (index, digit) in digits.into_iter().enumerate() {
                    let exponent = square_offset + i + index * self.w;
                    result += point
                        * Scalar::from(*digit as u64)
                        * Scalar::from(2u64).pow(&[exponent as u64]);
                }
            }
        }
        result
    }

    fn scalar_mul_naive_wnaf(&self, scalar: &Scalar) -> G1Projective {
        fn scalar_to_wnaf(scalar: Scalar, num_bits: usize, omega: usize) -> Vec<i64> {
            let mut wnaf_digits = vec![];
            let scalar_bytes = scalar.to_bytes_le().to_vec();
            wnaf_form(&mut wnaf_digits, scalar_bytes, omega);

            // TODO: the wnaf algorithm will pad unecessary zeroes
            // which then makes the padding algorithm below pad it even more in some cases.
            // We can either fix wnaf_form or remove the extra omega zeroes and then pad

            // Pad wnaf_digits to the next multiple of w^2
            let len = wnaf_digits.len();
            let w_squared = omega * omega;
            let num_sectors = (len + w_squared - 1) / w_squared;
            let padded_len = num_sectors * w_squared;
            wnaf_digits.extend(vec![0i64; padded_len - len]);

            wnaf_digits
        }
        // Convert scalar to bits and pad it to the necessary length
        // let mut wnaf_digits = scalar_to_bits(*scalar).to_vec();
        // wnaf_digits.extend(vec![0u8; self.l - wnaf_digits.len()]);

        let mut wnaf_digits = scalar_to_wnaf(*scalar, self.l, self.w);

        let point = G1Projective::generator();

        let mut result = G1Projective::identity();

        for t in 0..self.z {
            // t is used to scan a square
            let square_offset = t * self.w * self.w;
            for i in 0..self.w {
                // i is used to scan a particular row
                //
                //

                // Collect all of the necessary bits that differ by a factor of omega
                let digits = select_elements(&wnaf_digits, self.w as usize, t as usize, i as usize);

                for (index, digit) in digits.into_iter().enumerate() {
                    if *digit == 0 {
                        continue;
                    }

                    let is_negative = digit.is_negative();

                    let exponent = square_offset + i + index * self.w;
                    let two_pow = Scalar::from(2u64).pow(&[exponent as u64]);
                    let digit = Scalar::from(digit.unsigned_abs());

                    if is_negative {
                        result -= point * digit * two_pow;
                    } else {
                        result += point * digit * two_pow;
                    }
                }
            }
        }
        result
    }

    fn scalar_mul_naive_wnaf_iterated(&self, scalar: &Scalar) -> G1Projective {
        fn scalar_to_wnaf(scalar: Scalar, num_bits: usize, omega: usize) -> Vec<i64> {
            let mut wnaf_digits = vec![];
            let scalar_bytes = scalar.to_bytes_le().to_vec();
            wnaf_form(&mut wnaf_digits, scalar_bytes, omega);

            // TODO: the wnaf algorithm will pad unnecessary zeroes
            // which then makes the padding algorithm below pad it even more in some cases.
            // We can either fix wnaf_form or remove the extra omega zeroes and then pad

            // Pad wnaf_digits to the next multiple of w^2
            let len = wnaf_digits.len();
            let w_squared = omega * omega;
            let num_sectors = (len + w_squared - 1) / w_squared;
            let padded_len = num_sectors * w_squared;
            wnaf_digits.extend(vec![0i64; padded_len - len]);

            wnaf_digits
        }

        let mut result = G1Projective::identity();
        let point = G1Projective::generator();

        let mut wnaf_digits = scalar_to_wnaf(*scalar, self.l, self.w);
        for t in 0..self.z {
            // t is used to scan a square
            let square_offset = t * self.w * self.w;
            for i in 0..self.w {
                // i is used to scan a particular row
                //
                //

                // Collect all of the necessary bits that differ by a factor of omega
                let digits = select_elements(&wnaf_digits, self.w as usize, t as usize, i as usize);

                let mut total_value = 0;
                for (index, digit) in digits.iter().enumerate() {
                    total_value += (**digit as i64) * (1 << index as i64 * self.w as i64);
                }

                if total_value == 0 {
                    continue;
                }

                let is_negative = total_value.is_negative();
                let two_pow_offset = Scalar::from(2u64).pow(&[square_offset as u64]);
                let two_pow_i = Scalar::from(2u64).pow(&[i as u64]);

                if is_negative {
                    result -= point
                        * Scalar::from(total_value.unsigned_abs())
                        * two_pow_offset
                        * two_pow_i;
                } else {
                    result += point
                        * Scalar::from(total_value.unsigned_abs())
                        * two_pow_offset
                        * two_pow_i;
                }
            }
        }
        result
    }

    fn scalar_mul_precomps_wnaf(&self, scalar: &Scalar) -> G1Projective {
        fn scalar_to_wnaf(scalar: Scalar, num_bits: usize, omega: usize) -> Vec<i64> {
            let mut wnaf_digits = vec![];
            let scalar_bytes = scalar.to_bytes_le().to_vec();
            wnaf_form(&mut wnaf_digits, scalar_bytes, omega);

            // TODO: the wnaf algorithm will pad unnecessary zeroes
            // which then makes the padding algorithm below pad it even more in some cases.
            // We can either fix wnaf_form or remove the extra omega zeroes and then pad

            // Pad wnaf_digits to the next multiple of w^2
            let len = wnaf_digits.len();
            let w_squared = omega * omega;
            let num_sectors = (len + w_squared - 1) / w_squared;
            let padded_len = num_sectors * w_squared;
            wnaf_digits.extend(vec![0i64; padded_len - len]);

            wnaf_digits
        }

        let mut result = G1Projective::identity();
        let point = G1Projective::generator();

        let mut square_precomputations = Vec::new();
        let mut precomputations = Vec::new();
        // numbers are of the form a_0 + 2^w a_1 + 2^2w a_2 +... a_w 2^w*w
        for i in 1..(1 << self.w * self.w) {
            precomputations.push(point * Scalar::from(i as u64));
        }
        square_precomputations.push(precomputations);

        // Precompute the values across rows, across the square
        for k in 0..self.z {
            // Take the last
            let last_square = square_precomputations.last().unwrap().clone();
            // double all elements in the last square w*w times
            let shifted_square: Vec<_> = last_square
                .into_par_iter()
                .map(|mut point| {
                    for _ in 0..(self.w * self.w) {
                        point = point.double();
                    }
                    point
                })
                .collect();

            square_precomputations.push(shifted_square);
        }

        let mut wnaf_digits = scalar_to_wnaf(*scalar, self.l, self.w);
        for i in (0..self.w).rev() {
            result = result.double();

            for t in (0..self.z) {
                // t is used to scan a square
                // i is used to scan a particular row
                //
                //

                // Collect all of the necessary bits that differ by a factor of omega
                let digits = select_elements(&wnaf_digits, self.w as usize, t as usize, i as usize);

                let mut total_value = 0;
                for (index, digit) in digits.iter().enumerate() {
                    total_value += (**digit as i64) * (1 << index as i64 * self.w as i64);
                }

                if total_value == 0 {
                    continue;
                }

                let is_negative = total_value.is_negative();
                // let two_pow_offset = Scalar::from(2u64).pow(&[square_offset as u64]);
                // let two_pow_i = Scalar::from(2u64).pow(&[i as u64]);

                let mut chosen_point =
                    square_precomputations[t][(total_value.unsigned_abs() as usize - 1)];

                // for _ in 0..i {
                //     chosen_point = chosen_point.double()
                // }
                // for _ in 0..square_offset {
                //     chosen_point = chosen_point.double()
                // }

                if is_negative {
                    result -= chosen_point;
                } else {
                    result += chosen_point;
                }
            }
        }
        result
    }
    fn precompute_point(point: G1Affine, omega: usize, z: usize) -> PrecomputationsForPoint {
        let point = G1Projective::from(point);

        let mut square_precomputations = Vec::new();
        let mut precomputations = Vec::new();
        // numbers are of the form a_0 + 2^w a_1 + 2^2w a_2 +... a_w 2^w*w
        for i in 1..(1 << omega * omega) {
            precomputations.push(point * Scalar::from(i as u64));
        }
        square_precomputations.push(precomputations);

        // Precompute the values across rows, across the square
        for k in 0..z {
            // Take the last
            let last_square = square_precomputations.last().unwrap().clone();
            // double all elements in the last square w*w times
            let shifted_square: Vec<_> = last_square
                .into_par_iter()
                .map(|mut point| {
                    for _ in 0..(omega * omega) {
                        point = point.double();
                    }
                    point
                })
                .collect();

            square_precomputations.push(shifted_square);
        }

        square_precomputations
            .into_par_iter()
            .map(|sp| g1_batch_normalize(&sp))
            .collect()
    }
    fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        fn scalar_to_wnaf(scalar: Scalar, num_bits: usize, omega: usize) -> Vec<i64> {
            let mut wnaf_digits = vec![];
            let scalar_bytes = scalar.to_bytes_le().to_vec();
            wnaf_form(&mut wnaf_digits, scalar_bytes, omega);

            // TODO: the wnaf algorithm will pad unnecessary zeroes
            // which then makes the padding algorithm below pad it even more in some cases.
            // We can either fix wnaf_form or remove the extra omega zeroes and then pad

            // Pad wnaf_digits to the next multiple of w^2
            let len = wnaf_digits.len();
            let w_squared = omega * omega;
            let num_sectors = (len + w_squared - 1) / w_squared;
            let padded_len = num_sectors * w_squared;
            wnaf_digits.extend(vec![0i64; padded_len - len]);

            wnaf_digits
        }

        let mut result = G1Projective::identity();
        let now = std::time::Instant::now();
        let scalars_wnaf_digits: Vec<_> = scalars
            .into_iter()
            .map(|scalar| scalar_to_wnaf(*scalar, self.l, self.w))
            .collect();
        let mut rows = vec![vec![]; self.w];

        for (scalar_index, wnaf_digits) in scalars_wnaf_digits.into_iter().enumerate() {
            for i in (0..self.w).rev() {
                for t in (0..self.z) {
                    // t is used to scan a square
                    // i is used to scan a particular row
                    //
                    //

                    // Collect all of the necessary bits that differ by a factor of omega
                    let digits =
                        select_elements_iter(&wnaf_digits, self.w as usize, t as usize, i as usize);

                    let mut total_value = 0;
                    for (index, digit) in digits.enumerate() {
                        total_value += (digit as i64) * (1 << index as i64 * self.w as i64);
                    }

                    if total_value == 0 {
                        continue;
                    }

                    let is_negative = total_value.is_negative();
                    // let two_pow_offset = Scalar::from(2u64).pow(&[square_offset as u64]);
                    // let two_pow_i = Scalar::from(2u64).pow(&[i as u64]);

                    let mut chosen_point = self.precomputed_points[scalar_index][t]
                        [(total_value.unsigned_abs() as usize - 1)];

                    let chosen_point = if is_negative {
                        -chosen_point
                    } else {
                        chosen_point
                    };
                    rows[i].push(chosen_point);
                }
            }
        }

        // Sum all rows together
        let summed_rows = multi_batch_addition_diff_stride(rows);

        // Combine rows together (they differ by a factor of 2)
        let mut result = G1Projective::identity();
        for row in summed_rows.into_iter().rev() {
            result = result.double();
            result += row;
        }

        dbg!(now.elapsed().as_micros());

        result
    }
}

fn select_elements<T>(vector: &[T], w: usize, sector: usize, offset: usize) -> Vec<&T> {
    // Calculate the total number of sectors
    let total_sectors = vector.len() / (w * w);

    // Validate that the vector length is a multiple of w squared
    if vector.len() % (w * w) != 0 {
        panic!(
            "The size of the vector must be a multiple of w squared. got = {}, expected = {}",
            vector.len(),
            w * w
        );
    }
    // Validate that the sector index is within the valid range
    if sector >= total_sectors {
        panic!("Sector index out of range.");
    }
    // Validate that the offset is within the valid range
    if offset >= w {
        panic!("Offset must be in the range [0, w - 1].");
    }
    // Calculate the starting index of the sector
    let sector_start = sector * w * w;
    // Collect the selected elements
    let selected_elements: Vec<&T> = (0..w)
        .map(|k| &vector[sector_start + offset + k * w])
        .collect();
    selected_elements
}
fn select_elements_iter<'a, T: Copy + Clone>(
    vector: &'a [T],
    w: usize,
    sector: usize,
    offset: usize,
) -> impl Iterator<Item = T> + 'a {
    // Calculate the total number of sectors
    let total_sectors = vector.len() / (w * w);

    // Validate that the vector length is a multiple of w squared
    if vector.len() % (w * w) != 0 {
        panic!(
            "The size of the vector must be a multiple of w squared. got = {}, expected = {}",
            vector.len(),
            w * w
        );
    }
    // Validate that the sector index is within the valid range
    if sector >= total_sectors {
        panic!("Sector index out of range.");
    }
    // Validate that the offset is within the valid range
    if offset >= w {
        panic!("Offset must be in the range [0, w - 1].");
    }
    // Calculate the starting index of the sector
    let sector_start = sector * w * w;
    // Collect the selected elements
    (0..w).map(move |k| vector[sector_start + offset + k * w])
}

#[test]
fn test_debug_vector_selector() {
    let w = 4;
    let num_sectors = 4;
    // Create a vector with 3 sectors, each of size 16 (4*4), total size 48
    let vector: Vec<String> = (0..(num_sectors * w * w))
        .map(|i| format!("b_{}", i))
        .collect();

    let sector = 1; // Choose the sector index (0-based)
    let offset = 2; // Starting offset within the sector

    let selected = select_elements(&vector, w, sector, offset);

    let t = sector;
    let i = offset;
    let square_offset = t * w * w;

    for index_ in 0..w {
        let exp = square_offset + i + index_ * w;
        dbg!(exp);
    }

    println!(
        "Selected elements from sector {} with offset {}:",
        sector, offset
    );
    println!("{:?}", selected);
}

#[test]
fn test_seo_kim_naive_scalar_mul() {
    let scalar = -Scalar::from(2u64);
    let result = G1Projective::generator() * scalar;

    let w = 4;
    let sk = SeoKim::new(w, &[G1Affine::generator()]);

    let got = sk.scalar_mul_naive(&scalar);
    assert_eq!(got, result);

    let got = sk.scalar_mul_naive_wnaf(&scalar);
    assert_eq!(got, result);

    let got = sk.scalar_mul_naive_wnaf_iterated(&scalar);
    assert_eq!(got, result);

    // let got = sk.scalar_mul_precomps_wnaf(&scalar);
    // assert_eq!(got, result);

    let got = sk.msm(&[scalar]);
    assert_eq!(got, result);
}

#[test]
fn test_seokim_msm() {
    let num_points = 64;
    let points = random_points(num_points);

    let w = 4;
    let sk = SeoKim::new(w, &points);

    let scalars: Vec<_> = (0..num_points)
        .into_iter()
        .map(|_| Scalar::random(&mut rand::thread_rng()))
        .collect();

    let mut expected = G1Projective::identity();
    for (scalar, point) in scalars.iter().zip(points.iter()) {
        expected += G1Projective::from(*point) * scalar
    }

    let got = sk.msm(&scalars);
    assert_eq!(got, expected);
}

fn random_points(num_points: usize) -> Vec<G1Affine> {
    (0..num_points)
        .into_iter()
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect()
}
