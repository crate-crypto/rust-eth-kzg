use crate::{
    batch_addition::multi_batch_addition_binary_tree_stride, booth_encoding::get_booth_index,
    g1_batch_normalize, G1Projective, Scalar,
};
use blstrs::G1Affine;
use ff::PrimeField;
use group::Group;

/// A precomputed window-based structure for fast fixed-base multi-scalar multiplication (MSM) in G1.
///
/// This structure uses a windowed Booth encoding strategy, identical to BLST's approach,
/// to efficiently compute MSM when the base points (generators) are known and fixed.
///
/// It precomputes signed multiples of each generator point, enabling fast MSM
/// by replacing scalar multiplication with simple table lookups, doublings, and additions.
/// The trade-off is higher memory usage in exchange for reduced runtime.
///
/// - Let `wbits` be the window size in bits.
/// - Each scalar is split into windows of `wbits`, and for each window, only the relevant
///   signed multiple is selected from the precomputed table.
/// - The result is accumulated via batched additions for better performance.
///
/// Precomputation per point:
/// ```text
///     P_i \text{table}_i = \{P_i, 2P_i, ..., (2^{wbits - 1} - 1)P_i\}
/// ```
///
/// Total memory per point: 2^{wbits - 1} - 1 entries.
#[derive(Debug)]
pub struct FixedBaseMSMPrecompWindow {
    /// A 2D table where each row contains precomputed signed multiples of a single G1 base point.
    ///
    /// For each generator P_i, this stores:
    /// \{P_i, 2P_i, ..., (2^{wbits - 1} - 1)P_i\}
    /// in affine form, to support efficient lookup and minimal memory overhead.
    table: Vec<Vec<G1Affine>>,
    /// Number of bits per window (window size), determining the precomputation granularity.
    ///
    /// - Larger `wbits` → fewer windows, more precomputation.
    /// - Smaller `wbits` → more windows, less memory usage.
    ///
    /// Each scalar is broken into roughly:
    /// Scalar::NUM_BITS / wbits windows.
    wbits: usize,
}

impl FixedBaseMSMPrecompWindow {
    /// Constructs a new `FixedBaseMSMPrecompWindow` by precomputing scalar multiples of input G1 points.
    ///
    /// For each point, it computes odd multiples up to (2^{wbits - 1} - 1)P to enable
    /// efficient fixed-base MSM using Booth encoding.
    ///
    /// - `points`: G1 base points to precompute.
    /// - `wbits`: Number of bits per window in the scalar decomposition.
    pub fn new(points: &[G1Affine], wbits: usize) -> Self {
        // For every point `P`, wbits indicates that we should compute
        // 1 * P, ..., (2^{wbits} - 1) * P
        //
        // The total amount of memory is roughly (numPoints * 2^{wbits} - 1)
        // where each point is 64 bytes.
        let table = points
            .iter()
            .map(|point| Self::precompute_points(wbits, *point))
            .collect();

        Self { table, wbits }
    }

    /// Given a point, we precompute P,..., (2^{w-1}-1) * P
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

    /// Computes a fixed-base multi-scalar multiplication (MSM) using precomputed window tables.
    ///
    /// This method uses Booth window encoding to slice each scalar into signed digit windows.
    /// For each window, the appropriate signed multiple is selected from the precomputed table,
    /// and batched additions are performed across all scalars in the same window to accelerate accumulation.
    ///
    /// The MSM result is reconstructed from the window-wise accumulations using repeated doublings.
    ///
    ///
    /// # Parameters
    /// - `scalars`: A slice of scalar field elements, one per base point.
    ///
    /// # Returns
    /// - A `G1Projective` element representing the result of the MSM:
    ///   `∑ scalar_i * base_point_i`
    ///
    /// # Panics
    /// - Panics if `scalars.len()` does not match the number of precomputed base points (`self.table.len()`).
    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        // Convert each scalar to little-endian byte representation
        let scalars_bytes: Vec<_> = scalars.iter().map(|a| a.to_bytes_le()).collect();
        // Number of scalar "windows" (i.e., chunks of `wbits` bits per scalar)
        let number_of_windows = Scalar::NUM_BITS as usize / self.wbits + 1;

        // Initialize a vector to collect all points contributing to each window
        let mut windows_of_points = vec![Vec::with_capacity(scalars.len()); number_of_windows];

        // For each window index
        for (window_idx, windows_of_point) in windows_of_points
            .iter_mut()
            .enumerate()
            .take(number_of_windows)
        {
            // For each scalar and its byte representation
            for (scalar_idx, scalar_bytes) in scalars_bytes.iter().enumerate() {
                // Extract Booth-encoded digit at the given window position
                let point_idx = get_booth_index(window_idx, self.wbits, scalar_bytes.as_ref());

                // Skip zero digits (no contribution in this window)
                if point_idx == 0 {
                    continue;
                }

                // Determine sign and index for lookup
                let is_scalar_negative = point_idx.is_negative();
                let point_idx = point_idx.unsigned_abs() as usize - 1;

                // Fetch the multiple from the table, and negate if necessary
                let mut point = self.table[scalar_idx][point_idx];
                if is_scalar_negative {
                    point = -point;
                }

                // Add the contribution to the current window bucket
                windows_of_point.push(point);
            }
        }

        // Batch-add all points in each window bucket
        let accumulated_points = multi_batch_addition_binary_tree_stride(windows_of_points);

        // Now accumulate the windows by doubling wbits times
        let mut result = *accumulated_points.last().unwrap();
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

    #[test]
    fn test_msm_zero_scalars_returns_identity() {
        let generators: Vec<_> = (0..10)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();
        let scalars = vec![Scalar::ZERO; generators.len()];

        let msm = FixedBaseMSMPrecompWindow::new(&generators, 7);
        let result = msm.msm(&scalars);

        assert_eq!(result, G1Projective::identity());
    }

    #[test]
    fn test_msm_all_negative_scalars() {
        let generators: Vec<_> = (0..10)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();
        let scalars: Vec<_> = (0..10)
            .map(|_| -Scalar::random(&mut rand::thread_rng()))
            .collect();

        let naive_result: G1Projective = generators
            .iter()
            .zip(&scalars)
            .map(|(p, s)| G1Projective::from(*p) * s)
            .sum();

        let msm = FixedBaseMSMPrecompWindow::new(&generators, 7);
        let result = msm.msm(&scalars);

        assert_eq!(result, naive_result);
    }

    #[test]
    fn test_msm_single_generator() {
        let generator = G1Projective::random(&mut rand::thread_rng()).into();
        let scalar = Scalar::random(&mut rand::thread_rng());

        let expected = G1Projective::from(generator) * scalar;

        let msm = FixedBaseMSMPrecompWindow::new(&[generator], 5);
        let result = msm.msm(&[scalar]);

        assert_eq!(result, expected);
    }
}
