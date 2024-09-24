use crate::{
    batch_add::{batch_addition, batch_addition_diff_stride},
    booth_encoding::get_booth_index,
    g1_batch_normalize, G1Projective, Scalar,
};
use rayon::prelude::*;

use blstrs::G1Affine;
use ff::{Field, PrimeField};
use group::Group;

// Note: This is the same strategy that blst uses
#[derive(Debug)]
pub struct FixedBaseMSMPrecompAllWindow {
    tables: Vec<Vec<G1Affine>>,
    window_size: usize,
    num_windows: usize,
}

impl FixedBaseMSMPrecompAllWindow {
    pub fn new(points: &[G1Affine], window_size: usize) -> Self {
        let num_windows = Scalar::NUM_BITS as usize / window_size + 1;

        let precomputed_points: Vec<_> = points
            .iter()
            .map(|point| Self::precompute_points(window_size, num_windows, *point))
            .collect();

        Self {
            tables: precomputed_points,
            window_size,
            num_windows,
        }
    }

    fn precompute_points(
        window_size: usize,
        number_of_windows: usize,
        point: G1Affine,
    ) -> Vec<G1Affine> {
        let window_size_scalar = Scalar::from(1 << window_size);

        use rayon::prelude::*;

        let all_tables: Vec<_> = (0..number_of_windows)
            .into_par_iter()
            .flat_map(|window_index| {
                let window_scalar = window_size_scalar.pow(&[window_index as u64]);
                let mut lookup_table = Vec::with_capacity(1 << (window_size - 1));
                let point = G1Projective::from(point) * window_scalar;
                let mut current = point;
                // Compute and store multiples
                for _ in 0..(1 << (window_size - 1)) {
                    lookup_table.push(current);
                    current += point;
                }
                g1_batch_normalize(&lookup_table)
            })
            .collect();

        all_tables
    }

    // Given a point, we precompute P,..., (2^{w-1}-1) * P
    // fn precompute_points(wbits: usize, point: G1Affine) -> Vec<G1Affine> {
    //     let mut lookup_table = Vec::with_capacity(1 << (wbits - 1));

    //     // Convert to projective for faster operations
    //     let mut current = G1Projective::from(point);

    //     // Compute and store multiples
    //     for _ in 0..(1 << (wbits - 1)) {
    //         lookup_table.push(current);
    //         current += point;
    //     }

    //     g1_batch_normalize(&lookup_table)
    // }

    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        let scalars_bytes: Vec<_> = scalars.iter().map(|a| a.to_bytes_le()).collect();

        let mut points_to_add = Vec::new();

        for window_idx in 0..self.num_windows {
            for (scalar_idx, scalar_bytes) in scalars_bytes.iter().enumerate() {
                let sub_table = &self.tables[scalar_idx];
                let point_idx =
                    get_booth_index(window_idx, self.window_size, scalar_bytes.as_ref());

                if point_idx == 0 {
                    continue;
                }
                let sign = point_idx.is_positive();
                let point_idx = point_idx.unsigned_abs() as usize - 1;

                // Scale the point index by the window index to figure out whether
                // we need P, 2^wP, 2^{2w}P, etc
                let scaled_point_index = window_idx * (1 << (self.window_size - 1)) + point_idx;
                let mut point = sub_table[scaled_point_index];

                if !sign {
                    point = -point;
                }

                points_to_add.push(point);
            }
        }

        batch_addition(points_to_add).into()
    }
}

#[cfg(test)]
mod all_windows_tests {
    use super::*;
    use ff::Field;
    use group::prime::PrimeCurveAffine;

    #[test]
    fn precomp_lookup_table() {
        use group::Group;
        let lookup_table =
            FixedBaseMSMPrecompAllWindow::precompute_points(7, 1, G1Affine::generator());

        for i in 1..lookup_table.len() {
            let expected = G1Projective::generator() * Scalar::from((i + 1) as u64);
            assert_eq!(lookup_table[i], expected.into(),)
        }
    }

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

        let fbm = FixedBaseMSMPrecompAllWindow::new(&generators, 7);
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
            let fbm = FixedBaseMSMPrecompAllWindow::new(&generators, i);
            fbm.msm(&scalars);
        }
    }
}
