use crate::batch_inversion::{batch_inverse, batch_inverse_scratch_pad};
use blstrs::{Fp, G1Affine, G1Projective};
use ff::Field;
use group::Group;

#[inline(always)]
fn point_add_double(p1: G1Affine, p2: G1Affine, inv: &blstrs::Fp) -> G1Affine {
    use ff::Field;

    let lambda = if p1 == p2 {
        p1.x().square().mul3() * inv
    } else {
        (p2.y() - p1.y()) * inv
    };

    let x = lambda.square() - p1.x() - p2.x();
    let y = lambda * (p1.x() - x) - p1.y();

    G1Affine::from_raw_unchecked(x, y, false)
}

#[inline(always)]
// Note: We do not handle the case where p1 == -p2
fn choose_add_or_double(p1: G1Affine, p2: G1Affine) -> Fp {
    if p1 == p2 {
        p2.y().double()
    } else {
        p2.x() - p1.x()
    }
}

const BATCH_INVERSE_THRESHOLD: usize = 16;

// TODO(benedikt): top down balanced tree idea - benedikt
// TODO: search tree for sorted array
pub fn batch_addition_binary_tree_stride(mut points: Vec<G1Affine>) -> G1Projective {
    if points.is_empty() {
        return G1Projective::identity();
    }

    let mut new_differences = Vec::with_capacity(points.len());

    let mut sum = G1Projective::identity();

    while points.len() > BATCH_INVERSE_THRESHOLD {
        if points.len() % 2 != 0 {
            sum += points
                .pop()
                .expect("infallible; since points has an odd length");
        }
        new_differences.clear();

        for i in (0..=points.len() - 2).step_by(2) {
            let p1 = points[i];
            let p2 = points[i + 1];
            new_differences.push(choose_add_or_double(p1, p2));
        }

        batch_inverse(&mut new_differences);

        for (i, inv) in (0..=points.len() - 2).step_by(2).zip(&new_differences) {
            let p1 = points[i];
            let p2 = points[i + 1];
            points[i / 2] = point_add_double(p1, p2, inv);
        }

        // The latter half of the vector is now unused,
        // all results are stored in the former half.
        points.truncate(new_differences.len())
    }

    for point in points {
        sum += point
    }

    sum
}

pub fn multi_batch_addition_binary_tree_stride(
    mut multi_points: Vec<Vec<G1Affine>>,
) -> Vec<G1Projective> {
    let total_num_points: usize = multi_points.iter().map(|p| p.len()).sum();
    let mut scratchpad = Vec::with_capacity(total_num_points);

    // Find the largest buckets, this will be the bottleneck for the number of iterations
    let mut max_bucket_length = 0;
    for points in multi_points.iter() {
        max_bucket_length = std::cmp::max(max_bucket_length, points.len());
    }

    // Compute the total number of "unit of work"
    // In the single batch addition case this is analogous to
    // the batch inversion threshold
    #[inline(always)]
    fn compute_threshold(points: &[Vec<G1Affine>]) -> usize {
        points
            .iter()
            .map(|p| {
                if p.len() % 2 == 0 {
                    p.len() / 2
                } else {
                    (p.len() - 1) / 2
                }
            })
            .sum()
    }

    let mut new_differences = Vec::with_capacity(max_bucket_length);
    let mut total_amount_of_work = compute_threshold(&multi_points);

    let mut sums = vec![G1Projective::identity(); multi_points.len()];

    // TODO: total_amount_of_work does not seem to be changing performance that much
    while total_amount_of_work > BATCH_INVERSE_THRESHOLD {
        // For each point, we check if they are odd and pop off
        // one of the points
        for (points, sum) in multi_points.iter_mut().zip(sums.iter_mut()) {
            // Make the number of points even
            if points.len() % 2 != 0 {
                *sum += points.pop().unwrap();
            }
        }

        new_differences.clear();

        // For each pair of points over all
        // vectors, we collect them and put them in the
        // inverse array
        for points in multi_points.iter() {
            if points.len() < 2 {
                continue;
            }
            for i in (0..=points.len() - 2).step_by(2) {
                new_differences.push(choose_add_or_double(points[i], points[i + 1]));
            }
        }

        batch_inverse_scratch_pad(&mut new_differences, &mut scratchpad);

        let mut new_differences_offset = 0;

        for points in multi_points.iter_mut() {
            if points.len() < 2 {
                continue;
            }
            for (i, inv) in (0..=points.len() - 2)
                .step_by(2)
                .zip(&new_differences[new_differences_offset..])
            {
                let p1 = points[i];
                let p2 = points[i + 1];
                points[i / 2] = point_add_double(p1, p2, inv);
            }

            let num_points = points.len() / 2;
            // The latter half of the vector is now unused,
            // all results are stored in the former half.
            points.truncate(num_points);
            new_differences_offset += num_points
        }

        total_amount_of_work = compute_threshold(&multi_points);
    }

    for (sum, points) in sums.iter_mut().zip(multi_points) {
        for point in points {
            *sum += point
        }
    }

    sums
}

#[cfg(test)]
mod tests {

    use crate::batch_add::{
        batch_addition_binary_tree_stride, multi_batch_addition_binary_tree_stride,
    };

    use blstrs::{G1Affine, G1Projective};
    use group::Group;

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

        let got_result = batch_addition_binary_tree_stride(points.clone());
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
            .map(|points| batch_addition_binary_tree_stride(points).into())
            .collect();

        let got_results = multi_batch_addition_binary_tree_stride(random_sets_of_points_clone);
        assert_eq!(got_results, expected_results);
    }
}
