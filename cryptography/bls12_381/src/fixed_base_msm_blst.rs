use std::time::{Duration, Instant};

use crate::{
    batch_add::{
        batch_addition, batch_addition_diff_stride, multi_batch_addition,
        multi_batch_addition_diff_stride,
    },
    booth_encoding::{self, get_booth_index},
    fixed_base_msm_pippenger::FixedBaseMSMPippenger,
    g1_batch_normalize, G1Projective, Scalar,
};
use blstrs::{Fp, G1Affine};
use ff::PrimeField;
use group::prime::PrimeCurveAffine;
use group::Group;

// FixedBasePrecomp blst way with some changes
#[derive(Debug)]
pub struct FixedBaseMSMPrecompBLST {
    table: Vec<Vec<G1Affine>>, // TODO: Make this a Vec<> and then just do the maths in msm function for offsetting
    // table: Vec<G1Affine>,
    wbits: usize,
}

impl FixedBaseMSMPrecompBLST {
    pub fn new(points: &[G1Affine], wbits: usize) -> Self {
        // For every point `P`, wbits indicates that we should compute
        // 1 * P, ..., (2^{wbits} - 1) * P
        //
        // The total amount of memory is roughly (numPoints * 2^{wbits} - 1)
        // where each point is 64 bytes.
        //
        //
        let mut precomputed_points: Vec<_> = points
            .into_iter()
            .map(|point| Self::precompute_points(wbits, *point))
            .collect();

        Self {
            table: precomputed_points,
            wbits,
        }
    }
    // Given a point, we precompute P,..., (2^w -1) * P
    fn precompute_points(wbits: usize, point: G1Affine) -> Vec<G1Affine> {
        let mut lookup_table = Vec::with_capacity(1 << (wbits - 1));

        // Convert to projective for faster operations
        let mut current = G1Projective::from(point);

        // Compute and store multiples
        for _ in 0..(1 << (wbits - 1)) {
            lookup_table.push(current);
            current += point;
        }

        g1_batch_normalize(&lookup_table)
    }

    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        let scalars_bytes: Vec<_> = scalars.iter().map(|a| a.to_bytes_le()).collect();
        let number_of_windows = Scalar::NUM_BITS as usize / self.wbits + 1;

        let mut windows_of_points = vec![Vec::with_capacity(scalars.len()); number_of_windows];

        for window_idx in 0..number_of_windows {
            for (scalar_idx, scalar_bytes) in scalars_bytes.iter().enumerate() {
                let sub_table = &self.table[scalar_idx];
                let point_idx = get_booth_index(window_idx, self.wbits, scalar_bytes.as_ref());

                if point_idx == 0 {
                    continue;
                }
                let sign = point_idx.is_positive();
                let point_idx = point_idx.unsigned_abs() as usize - 1;
                let mut point = sub_table[point_idx];
                // let mut point = self.table[(scalar_idx * 1 << self.wbits) + point_idx];
                if !sign {
                    point = -point;
                }

                windows_of_points[window_idx].push(point);
            }
        }

        // For each window, lets add all of the points together.
        // let accumulated_points: Vec<_> = windows_of_points
        //     .into_iter()
        //     .map(|wp| batch_addition_diff_stride(wp))
        //     .collect();
        let accumulated_points = multi_batch_addition_diff_stride(windows_of_points);

        // Now accumulate the windows by doubling wbits times
        let mut result: G1Projective = *accumulated_points.last().unwrap();
        for point in accumulated_points.into_iter().rev().skip(1) {
            // Double the result 'wbits' times
            for _ in 0..self.wbits {
                result = result.double();
            }
            // Add the accumulated point for this window
            result += point;
        }

        result
    }

    // Returns all of the unreduced windows
    fn partial_msm_part1(&self, scalars: &[Scalar]) -> Vec<Vec<G1Affine>> {
        let scalars_bytes: Vec<_> = scalars.iter().map(|a| a.to_bytes_le()).collect();
        let number_of_windows = Scalar::NUM_BITS as usize / self.wbits + 1;

        let mut windows_of_points = vec![Vec::with_capacity(scalars.len()); number_of_windows];

        for window_idx in 0..number_of_windows {
            for (scalar_idx, scalar_bytes) in scalars_bytes.iter().enumerate() {
                let sub_table = &self.table[scalar_idx];
                let point_idx = get_booth_index(window_idx, self.wbits, scalar_bytes.as_ref());

                if point_idx == 0 {
                    continue;
                }
                let sign = point_idx.is_positive();
                let point_idx = point_idx.unsigned_abs() as usize - 1;
                let mut point = sub_table[point_idx];
                // let mut point = self.table[(scalar_idx * 1 << self.wbits) + point_idx];
                if !sign {
                    point = -point;
                }

                windows_of_points[window_idx].push(point);
            }
        }

        windows_of_points
    }

    fn partial_msm_part2(&self, accumulated_points: &[G1Projective]) -> G1Projective {
        // Now accumulate the windows by doubling wbits times
        let mut result: G1Projective = *accumulated_points.last().unwrap();
        for point in accumulated_points.into_iter().rev().skip(1) {
            // Double the result 'wbits' times
            for _ in 0..self.wbits {
                result = result.double();
            }
            // Add the accumulated point for this window
            result += point;
        }

        result
    }

    fn partial_msm_part2_affine(&self, accumulated_points: &[G1Affine]) -> G1Projective {
        // Now accumulate the windows by doubling wbits times
        let mut result: G1Projective = G1Projective::from(*accumulated_points.last().unwrap());
        for point in accumulated_points.into_iter().rev().skip(1) {
            // Double the result 'wbits' times
            for _ in 0..self.wbits {
                result = result.double();
            }
            // Add the accumulated point for this window
            result += point;
        }

        result
    }
}

#[derive(Debug)]
pub struct FixedBaseMultiMSMPrecompBLST {
    msms: Vec<FixedBaseMSMPrecompBLST>,
}

impl FixedBaseMultiMSMPrecompBLST {
    pub fn new(generator_sets: Vec<Vec<G1Affine>>, wbits: usize) -> Self {
        let msms: Vec<_> = generator_sets
            .into_iter()
            .map(|generators| FixedBaseMSMPrecompBLST::new(&generators, wbits))
            .collect();
        Self { msms }
    }

    pub fn multi_msm(&self, scalar_sets: Vec<Vec<Scalar>>) -> Vec<G1Projective> {
        let num_results = scalar_sets.len();

        let number_of_windows = Scalar::NUM_BITS as usize / self.msms[0].wbits + 1;

        let multiple_windows: Vec<_> = scalar_sets
            .into_iter()
            .zip(&self.msms)
            .flat_map(|(scalars, msm)| msm.partial_msm_part1(&scalars))
            .collect();

        let accumulated_points = multi_batch_addition_diff_stride(multiple_windows);

        let mut results = Vec::with_capacity(num_results);

        for (set_of_windows, msm) in accumulated_points
            .chunks_exact(number_of_windows)
            .into_iter()
            .zip(&self.msms)
        {
            results.push(msm.partial_msm_part2(set_of_windows));
        }

        results
    }
}

#[test]
fn precomp_lookup_table() {
    use group::Group;
    let lookup_table = FixedBaseMSMPrecompBLST::precompute_points(7, G1Affine::generator());

    for i in 1..lookup_table.len() {
        let expected = G1Projective::generator() * Scalar::from((i + 1) as u64);
        assert_eq!(lookup_table[i], expected.into(),)
    }
}
use ff::Field;

#[test]
fn msm_blst_precomp() {
    let length = 64;
    let generators: Vec<_> = (0..length)
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect();
    let scalars: Vec<_> = (0..length)
        .map(|_| Scalar::random(&mut rand::thread_rng()))
        .collect();

    let res = crate::lincomb::g1_lincomb(&generators, &scalars)
        .expect("number of generators and number of scalars is equal");

    let fbm = FixedBaseMSMPrecompBLST::new(&generators, 7);
    let result = fbm.msm(&scalars);

    assert_eq!(res, result);
}

#[test]
fn bench_window_sizes_msm() {
    let length = 64;
    let generators: Vec<_> = (0..length)
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect();
    let scalars: Vec<_> = (0..length)
        .map(|_| Scalar::random(&mut rand::thread_rng()))
        .collect();

    for i in 2..=14 {
        let fbm = FixedBaseMSMPrecompBLST::new(&generators, i);
        fbm.msm(&scalars);
    }
}
