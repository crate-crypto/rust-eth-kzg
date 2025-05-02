use crate::{
    batch_addition::multi_batch_addition_binary_tree_stride, booth_encoding::get_booth_index,
    g1_batch_normalize, G1Projective, Scalar,
};
use blstrs::G1Affine;
use ff::PrimeField;
use group::Group;

// Note: This is the same strategy that blst uses
#[derive(Debug)]
pub struct FixedBaseMSMPrecompWindow {
    table: Vec<Vec<G1Affine>>,
    wbits: usize,
}

impl FixedBaseMSMPrecompWindow {
    pub fn new(points: &[G1Affine], wbits: usize) -> Self {
        // For every point `P`, wbits indicates that we should compute
        // 1 * P, ..., (2^{wbits} - 1) * P
        //
        // The total amount of memory is roughly (numPoints * 2^{wbits} - 1)
        // where each point is 64 bytes.
        //
        let precomputed_points: Vec<_> = points
            .iter()
            .map(|point| Self::precompute_points(wbits, *point))
            .collect();

        Self {
            table: precomputed_points,
            wbits,
        }
    }
    // Given a point, we precompute P,..., (2^{w-1}-1) * P
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
        let scalars_bytes: Vec<_> = scalars.iter().map(Scalar::to_bytes_le).collect();
        let number_of_windows = Scalar::NUM_BITS as usize / self.wbits + 1;

        let mut windows_of_points = vec![Vec::with_capacity(scalars.len()); number_of_windows];

        for (window_idx, windows_of_point) in windows_of_points
            .iter_mut()
            .enumerate()
            .take(number_of_windows)
        {
            for (scalar_idx, scalar_bytes) in scalars_bytes.iter().enumerate() {
                let sub_table = &self.table[scalar_idx];
                let point_idx = get_booth_index(window_idx, self.wbits, scalar_bytes.as_ref());

                if point_idx == 0 {
                    continue;
                }
                let is_scalar_positive = point_idx.is_positive();
                let point_idx = point_idx.unsigned_abs() as usize - 1;
                let mut point = sub_table[point_idx];
                if !is_scalar_positive {
                    point = -point;
                }

                windows_of_point.push(point);
            }
        }

        let accumulated_points = multi_batch_addition_binary_tree_stride(windows_of_points);

        // Now accumulate the windows by doubling wbits times
        let mut result: G1Projective = *accumulated_points
            .last()
            .expect("at least one window required");
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

#[cfg(test)]
mod tests {
    use super::*;
    use ff::Field;
    use group::prime::PrimeCurveAffine;

    #[test]
    fn precomp_lookup_table() {
        use group::Group;
        let lookup_table = FixedBaseMSMPrecompWindow::precompute_points(7, G1Affine::generator());

        for (i, l) in lookup_table.iter().enumerate().skip(1) {
            let expected = G1Projective::generator() * Scalar::from((i + 1) as u64);
            assert_eq!(*l, expected.into());
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

        let fbm = FixedBaseMSMPrecompWindow::new(&generators, 7);
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
            let fbm = FixedBaseMSMPrecompWindow::new(&generators, i);
            fbm.msm(&scalars);
        }
    }
}
