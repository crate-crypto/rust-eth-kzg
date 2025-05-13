use blstrs::{Fp, G1Affine, G1Projective};

use crate::{
    batch_inversion::{batch_inverse, batch_inverse_scratch_pad},
    traits::*,
};

/// Adds two elliptic curve points (affine coordinates) using the point addition/doubling formula.
///
/// Note: The inversion is precomputed and passed as a parameter.
///
/// This function handles both addition of distinct points and point doubling.
#[inline(always)]
fn point_add_double(p1: G1Affine, p2: G1Affine, inv: &Fp) -> G1Affine {
    let lambda = if p1 == p2 {
        p1.x().square().mul3() * inv
    } else {
        (p2.y() - p1.y()) * inv
    };

    let x = lambda.square() - p1.x() - p2.x();
    let y = lambda * (p1.x() - x) - p1.y();

    G1Affine::from_raw_unchecked(x, y, false)
}

/// Chooses between point addition and point doubling based on the input points (affine coordinates).
///
/// Note: This does not handle the case where p1 == -p2.
///
/// This case is unlikely for our usecase, and is not trivial to handle.
#[inline(always)]
fn choose_add_or_double(p1: G1Affine, p2: G1Affine) -> Fp {
    if p1 == p2 {
        p2.y().double()
    } else {
        p2.x() - p1.x()
    }
}

/// This is the threshold to which batching the inversions in affine
/// formula costs more than doing mixed addition.
///
/// WARNING: Should be >= 2.
///     - The threshold cannot be below the number of points needed for group addition
const BATCH_INVERSE_THRESHOLD: usize = 16;

/// Efficiently computes the sum of many elliptic curve points (in affine form)
/// using a binary-tree-style reduction with batched inversions.
///
/// This method groups pairs of points, adds them using the standard EC formulas
/// (addition or doubling), and applies batch inversion to amortize the cost
/// of computing the slope (λ) denominators.
///
/// The reduction is repeated in-place until the number of points falls below a
/// threshold, at which point the remaining points are summed sequentially.
///
/// Returns the total sum of all input points as a `G1Projective`.
///
/// # Panics
///
/// It panics if any point `points` is identity point, or if the addition trace
/// has `[O, O]` or `[G, -G]` pair due to computing inverse of 0.
///
/// # Undefined returned value
///
/// It returns non-sense value if the trace of addition has any identity point,
/// which has negligible chance to happen if the points are not close to each
/// other.
// TODO(benedikt): top down balanced tree idea - benedikt
// TODO: search tree for sorted array
#[allow(dead_code)]
pub(crate) fn batch_addition_binary_tree_stride(mut points: Vec<G1Affine>) -> G1Projective {
    // We return the identity element if the input is empty
    if points.is_empty() {
        return G1Projective::identity();
    }

    debug_assert!(points.iter().all(|point| !bool::from(point.is_identity())));

    // Stores denominators for slope calculations
    let mut denominators = Vec::with_capacity(points.len());
    // Accumulates the final result (in projective form)
    let mut sum = G1Projective::identity();

    // Repeat the batch reduction until the number of points is small
    while points.len() > BATCH_INVERSE_THRESHOLD {
        // If there's an odd number of points, remove the last one
        // and add it directly to the accumulator (can't be paired)
        if points.len() % 2 != 0 {
            sum += points
                .pop()
                .expect("infallible; since points has an odd length");
        }

        // Clear and refill the denominators for this round
        denominators.clear();

        // For each pair of points, compute the denominator of λ
        for pair in points.chunks(2) {
            if let [p1, p2] = pair {
                denominators.push(choose_add_or_double(*p1, *p2));
            }
        }

        // Batch invert all denominators in one shot (amortized inversion)
        batch_inverse(&mut denominators);
        // Perform the actual addition or doubling using the precomputed λ
        for (i, inv) in (0..).zip(&denominators) {
            let p1 = points[2 * i];
            let p2 = points[2 * i + 1];
            points[i] = point_add_double(p1, p2, inv);
        }

        // The latter half of the vector is now unused,
        // all results are stored in the former half.
        points.truncate(denominators.len());
    }

    // Once below threshold, do a regular sequential addition of the rest
    for point in points {
        sum += point;
    }

    sum
}

/// Performs multi-batch addition of multiple sets of elliptic curve points.
///
/// This function efficiently adds multiple sets of points amortizing the cost of the
/// inversion over all of the sets, using the same binary tree approach with striding
/// as the single-batch version.
///
/// # Panics
///
/// It panics if any point `multi_points` is identity point, or if the addition
/// trace has `[O, O]` or `[G, -G]` pair due to computing inverse of 0.
///
/// # Undefined returned value
///
/// It returns non-sense value if the trace of addition has any identity point,
/// which has negligible chance to happen if the points are not close to each
/// other.
pub(crate) fn multi_batch_addition_binary_tree_stride(
    mut multi_points: Vec<Vec<G1Affine>>,
) -> Vec<G1Projective> {
    // Computes the total number of point pairs across all batches
    // (i.e., the total number of λ slope denominators needed)
    #[inline(always)]
    fn compute_threshold(points: &[Vec<G1Affine>]) -> usize {
        points.iter().map(|p| p.len() / 2).sum()
    }

    debug_assert!(multi_points
        .iter()
        .all(|points| points.iter().all(|point| !bool::from(point.is_identity()))));

    // Total number of points across all batches (used for scratchpad allocation)
    let total_num_points = multi_points.iter().map(Vec::len).sum();
    let mut scratchpad = Vec::with_capacity(total_num_points);

    // Find the largest set size — used to size the denominator buffer.
    //
    // This will be the bottleneck for the number of iterations
    let max_bucket_length = multi_points.iter().map(Vec::len).max().unwrap_or(0);

    // Preallocate space for denominators (reused across iterations)
    let mut denominators = Vec::with_capacity(max_bucket_length);
    let mut total_amount_of_work = compute_threshold(&multi_points);

    // Output accumulator: one result per set
    let mut sums = vec![G1Projective::identity(); multi_points.len()];

    // Keep reducing each set in-place until all fall below the threshold
    //
    // TODO: total_amount_of_work does not seem to be changing performance that much
    while total_amount_of_work > BATCH_INVERSE_THRESHOLD {
        // Make each set even-length by popping the last element and adding it directly
        for (points, sum) in multi_points.iter_mut().zip(sums.iter_mut()) {
            // Make the number of points even
            if points.len() % 2 != 0 {
                *sum += points.pop().expect("underflow");
            }
        }

        denominators.clear();

        // Collect slope denominators (either x2 - x1 or 2y1) from all point pairs
        for points in &multi_points {
            for pair in points.chunks(2).take(points.len() / 2) {
                if let [p1, p2] = pair {
                    denominators.push(choose_add_or_double(*p1, *p2));
                }
            }
        }

        // Batch invert all collected denominators using a shared scratchpad
        batch_inverse_scratch_pad(&mut denominators, &mut scratchpad);

        let mut denominators_offset = 0;

        // Apply point_add_double to each pair in each set, using inverted slopes
        for points in &mut multi_points {
            if points.len() < 2 {
                continue;
            }
            for (i, inv) in (0..=points.len() - 2)
                .step_by(2)
                .zip(&denominators[denominators_offset..])
            {
                let p1 = points[i];
                let p2 = points[i + 1];
                points[i / 2] = point_add_double(p1, p2, inv);
            }

            let num_points = points.len() / 2;
            // The latter half of the vector is now unused,
            // all results are stored in the former half.
            points.truncate(num_points);
            denominators_offset += num_points;
        }

        total_amount_of_work = compute_threshold(&multi_points);
    }

    // Final pass: add the few remaining points in each batch sequentially
    for (sum, points) in sums.iter_mut().zip(multi_points) {
        for point in points {
            *sum += point;
        }
    }

    sums
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_batch_addition() {
        let num_points = 101;
        let points: Vec<G1Affine> = (0..num_points)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();

        let expected_result: G1Affine = points
            .iter()
            .fold(G1Projective::identity(), |acc, p| acc + p)
            .into();

        let got_result = batch_addition_binary_tree_stride(points);
        assert_eq!(expected_result, got_result.into());
    }

    #[test]
    fn test_multi_batch_addition_binary_stride() {
        let num_points = 99;
        let num_sets = 5;
        let random_sets_of_points: Vec<Vec<G1Affine>> = (0..num_sets)
            .map(|_| {
                (0..num_points)
                    .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
                    .collect()
            })
            .collect();
        let random_sets_of_points_clone = random_sets_of_points.clone();

        let expected_results: Vec<G1Projective> = random_sets_of_points
            .into_iter()
            .map(batch_addition_binary_tree_stride)
            .collect();

        let got_results = multi_batch_addition_binary_tree_stride(random_sets_of_points_clone);
        assert_eq!(got_results, expected_results);
    }

    /// Strategy that produces random G1Projective points
    fn arb_g1_projective() -> impl Strategy<Value = G1Projective> {
        any::<u64>().prop_map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            G1Projective::random(&mut rng)
        })
    }

    proptest! {
        #[test]
        fn prop_batch_addition_matches_naive(
            points in proptest::collection::vec(arb_g1_projective(), 1..200)
        ) {
            let affine_points: Vec<G1Affine> = points.iter().copied().map(Into::into).collect();

            // Reference sum: naive sequential addition
            let expected: G1Projective = points.iter().copied().sum();

            // Test: batch addition
            let got = batch_addition_binary_tree_stride(affine_points);

            prop_assert_eq!(expected, got);
        }

        #[test]
        fn prop_multi_batch_addition_matches_naive(
            batch_sizes in proptest::collection::vec(1usize..50, 1..10),
            seeds in proptest::collection::vec(any::<u64>(), 1..10)
        ) {
            let mut sets = Vec::with_capacity(batch_sizes.len());
            let mut expected = Vec::with_capacity(batch_sizes.len());

            for (i, &size) in batch_sizes.iter().enumerate() {
                let seed = *seeds.get(i % seeds.len()).unwrap_or(&0);
                let mut rng = StdRng::seed_from_u64(seed);

                let proj_points: Vec<G1Projective> =
                    (0..size).map(|_| G1Projective::random(&mut rng)).collect();
                let affine_points: Vec<G1Affine> =
                    proj_points.iter().copied().map(Into::into).collect();

                expected.push(proj_points.iter().copied().sum());
                sets.push(affine_points);
            }

            let got = multi_batch_addition_binary_tree_stride(sets);
            prop_assert_eq!(got, expected);
        }
    }
}
