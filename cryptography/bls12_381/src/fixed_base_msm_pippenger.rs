use blstrs::G1Affine;
use blstrs::G1Projective;
use blstrs::Scalar;
use ff::PrimeField;
use group::Group;

use crate::booth_encoding::get_booth_index;
use crate::g1_batch_normalize;
use crate::G1Point;
#[derive(Debug, Clone)]
pub struct FixedBaseMSMPippenger {
    precomputed_points: Vec<G1Affine>,
    window_size: usize,
}

impl FixedBaseMSMPippenger {
    pub fn new(points: &[G1Affine]) -> FixedBaseMSMPippenger {
        // The +2 was empirically seen to give better results
        let window_size = (f64::from(points.len() as u32)).ln().ceil() as usize + 2;
        let number_of_windows = Scalar::NUM_BITS as usize / window_size + 1;
        let precomputed_points = precompute(window_size, number_of_windows, points);

        FixedBaseMSMPippenger {
            precomputed_points,
            window_size,
        }
    }

    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        pippenger_fixed_base_msm(scalars, &self.precomputed_points, self.window_size)
    }
}

pub fn precompute(
    window_size: usize,
    number_of_windows: usize,
    points: &[G1Point],
) -> Vec<G1Point> {
    // For each point, we compute number_of_windows-1 points
    let mut results = Vec::new();
    for point in points {
        // First add the original point
        results.push(point.into());

        // Then scale each successive point by 2^window_size
        for _ in 0..number_of_windows - 1 {
            let mut last_point_scaled_window_size: G1Projective = *results.last().unwrap();
            for _ in 0..window_size {
                last_point_scaled_window_size = last_point_scaled_window_size.double()
            }
            results.push(last_point_scaled_window_size)
        }
    }
    g1_batch_normalize(&results)
}

pub fn pippenger_fixed_base_msm(
    coeffs: &[Scalar],
    bases_precomputed: &[G1Point],
    window_size: usize,
) -> G1Projective {
    // assert_eq!(coeffs.len(), bases.len());

    let c = window_size;

    // coeffs to byte representation
    let coeffs: Vec<_> = coeffs.iter().map(|a| a.to_bytes_le()).collect();

    // Information on the points we want to add
    let mut all_information = vec![vec![]; 1 << (c - 1)];

    // number of windows
    let number_of_windows = Scalar::NUM_BITS as usize / c + 1;

    for window_idx in 0..number_of_windows {
        for (base_idx, coeff) in coeffs.iter().enumerate() {
            let buck_idx = get_booth_index(window_idx, c, coeff.as_ref());

            if buck_idx != 0 {
                // parse bucket index
                let sign = buck_idx.is_positive();
                let buck_idx = buck_idx.unsigned_abs() as usize - 1;
                //
                // Since we are using precomputed points, the base_idx is augmented
                //
                // We need to modify the base index to take into account:
                // - The window, so we fetch the precomputed base for that window
                // - The position of the point in the precomputed bases,
                // relative to the original bases vector
                //
                // If you imagine we had:
                // [P1, P2, P3]
                // precomp = [P1, c*P1,..., (num_window-1)*c*P1, P2,...]
                //
                // The index of P1, P2, etc can be computed by:
                // augmented_base_idx = base_idx * num_windows
                // Then in order to get the correct point, we do:
                // augmented_base_idx += window_idx
                let base_idx = (base_idx * number_of_windows) + window_idx;

                let point = if sign {
                    bases_precomputed[base_idx as usize]
                } else {
                    -bases_precomputed[base_idx as usize]
                };

                all_information[buck_idx].push(point);
            }
        }
    }

    // All of the above costs about 200 microseconds on 64 points.
    // Using a vector is about 3 times faster, but the points are not ordered by bucket index
    // so we could try and do a second pass on the vector to see if thats quicker for small numPoints
    //
    // Note: for duplicate points, we could either put them in the running sum
    // or use the optimized formulas
    // let mut all_points = Vec::new();
    // let mut bucket_indices = Vec::new();
    let (bucket_indices, all_information): (Vec<_>, Vec<_>) = all_information
        .into_iter()
        .enumerate()
        .filter(|(_, points)| !points.is_empty())
        .map(|(index, points)| (((index + 1) as u64), points))
        .collect();

    let buckets_added = crate::batch_add::multi_batch_addition(all_information);
    let res = subsum_accumulation(&bucket_indices, &buckets_added);
    res
}

pub fn multi_msm(
    matrix_coeffs: &[&[Scalar]],
    bases_precomputed: &[G1Point],
    window_size: usize,
) -> Vec<G1Projective> {
    // assert_eq!(coeffs.len(), bases.len());

    let c = window_size;

    // coeffs to byte representation
    let matrix_coeffs: Vec<_> = matrix_coeffs
        .iter()
        .map(|a| {
            a.iter()
                .map(|coeff| coeff.to_bytes_le())
                .collect::<Vec<_>>()
        })
        .collect();

    // Information on the points we want to add
    let mut all_information = vec![vec![]; (1 << (c - 1)) * matrix_coeffs.len()];

    // number of windows
    let number_of_windows = Scalar::NUM_BITS as usize / c + 1;

    for window_idx in 0..number_of_windows {
        for (msm_index, coeffs) in matrix_coeffs.iter().enumerate() {
            for (base_idx, coeff) in coeffs.iter().enumerate() {
                let buck_idx = get_booth_index(window_idx, c, coeff.as_ref());

                if buck_idx != 0 {
                    // parse bucket index
                    let sign = buck_idx.is_positive();
                    let buck_idx = buck_idx.unsigned_abs() as usize - 1;
                    //
                    // Since we are using precomputed points, the base_idx is augmented
                    //
                    // We need to modify the base index to take into account:
                    // - The window, so we fetch the precomputed base for that window
                    // - The position of the point in the precomputed bases,
                    // relative to the original bases vector
                    //
                    // If you imagine we had:
                    // [P1, P2, P3]
                    // precomp = [P1, c*P1,..., (num_window-1)*c*P1, P2,...]
                    //
                    // The index of P1, P2, etc can be computed by:
                    // augmented_base_idx = base_idx * num_windows
                    // Then in order to get the correct point, we do:
                    // augmented_base_idx += window_idx
                    let base_idx = (base_idx * number_of_windows) + window_idx;

                    let point = if sign {
                        bases_precomputed[base_idx as usize]
                    } else {
                        -bases_precomputed[base_idx as usize]
                    };

                    all_information[buck_idx + (msm_index * (1 << (c - 1)))].push(point);
                }
            }
        }
    }

    // All of the above costs about 200 microseconds on 64 points.
    // Using a vector is about 3 times faster, but the points are not ordered by bucket index
    // so we could try and do a second pass on the vector to see if thats quicker for small numPoints
    //
    // Note: for duplicate points, we could either put them in the running sum
    // or use the optimized formulas
    let (chunked_bucket_indices, all_information): (Vec<Vec<u64>>, Vec<_>) = all_information
        .chunks(1 << (c - 1))
        .into_iter()
        .map(|chunk| {
            let chunked_indices: Vec<u64> = chunk
                .iter()
                .enumerate()
                .filter(|(_, points)| !points.is_empty())
                .map(|(index, _)| (index + 1) as u64)
                .collect();

            let all_info: Vec<_> = chunk
                .iter()
                .filter(|points| !points.is_empty())
                .flat_map(|points| points.iter().cloned()) // Flatten the points directly
                .collect();

            (chunked_indices, all_info)
        })
        .collect();

    let buckets_added = crate::batch_add::multi_batch_addition(all_information);

    let mut result = Vec::new();
    let mut start = 0;
    for bucket_indices in chunked_bucket_indices {
        result.push(subsum_accumulation(
            &bucket_indices,
            &buckets_added[start..start + bucket_indices.len()],
        ));
        start += bucket_indices.len()
    }
    result
}

// Algorithm1 from the LFG paper
// TODO: Fix later, this algorithm is broken in the POC and the paper
// fn subsum_accumulation(b: &[u64], s: &[G1Affine]) -> G1Projective {
//     assert_eq!(b.len(), s.len(), "Input arrays must have the same length");
//     let d = *b.iter().max().unwrap() as usize;

//     // Define a length-(d + 1) array tmp = [0] × (d + 1)
//     let mut tmp_d = vec![G1Projective::identity(); d + 1];
//     let mut tmp = G1Projective::identity();

//     // Iterate from |B| to 1 by -1
//     for i in (1..b.len()).rev() {
//         // tmp[0] = tmp[0] + S_i
//         tmp += s[i];

//         // k = b_i - b_{i-1}
//         let k = (b[i] - b[i - 1]) as usize;

//         // if k >= 1 then tmp[k] = tmp[k] + tmp[0]
//         // if k >= 1 {
//         //     let t0 = tmp_d[0];
//         //     tmp_d[k] += t0;
//         // }
//         tmp_d[k] += tmp;
//     }

//     // The original paper has a bug and does not deal with the case
//     // when there is only 1 point
//     if b.len() == 1 {
//         tmp_d[(b[0] - 1) as usize] = s[0].into()
//     }

//     // Now do running sum stuff
//     // summation by parts
//     // e.g. 3a + 2b + 1c = a +
//     //                    (a) + b +
//     //                    ((a) + b) + c
//     let mut running_sum = G1Projective::identity();
//     let mut res = G1Projective::identity();
//     // for i in (0..d).rev() {
//     //     running_sum += &tmp_d[i];
//     //     res += &running_sum;
//     // }
//     // We can use d to skip top buckets that are empty (done above)
//     tmp_d.into_iter().rev().for_each(|b| {
//         running_sum += &b;
//         res += &running_sum;
//     });
//     res
// }

// This is poormans version of Algorithm 1 from LFG
//
// It seems to be faster, but thats likely because the actual one is not implemented
// correctly and does not have the short cuts for bucket sizes 0 and 1
fn subsum_accumulation(b: &[u64], s: &[G1Affine]) -> G1Projective {
    assert_eq!(b.len(), s.len());

    // If we only have one, then we can return the scalar multiplication
    // This is an assumption that LFG was making too.
    if b.len() == 0 {
        return G1Projective::identity();
    }
    if b.len() == 1 {
        return s[0] * Scalar::from(b[0]);
    }

    // Now do running sum stuff
    // summation by parts but it does not need to be continuos
    let mut running_sum = G1Projective::identity();
    let mut res = G1Projective::identity();

    s.into_iter().enumerate().rev().for_each(|(index, point)| {
        running_sum += point;
        res += &running_sum;

        // Check that we are not at the last point
        if index > 0 {
            // We cannot fail here since we know the length of b is atleast 2
            let diff = b[index] - b[index - 1] - 1; // Note the -1 because if we have 2a + 1b, the diff will be 0 and the for loop will be skipped
                                                    // Before going to the next point, we need to account
                                                    // for the possible difference in scalars.
                                                    // ie we could be doing 3 * a + 1 * b
            for _ in 0..diff {
                res += running_sum
            }
        } else {
            //Check the diff between the last scalar and 1
            // This is so that we "finish" the horner sum.

            let diff = b[index] - 1;
            for _ in 0..diff {
                res += running_sum
            }
        }
    });
    res
}

// summation by parts
// e.g. 3a + 2b + 1c = a +
//                    (a) + b +
//                    ((a) + b) + c
//
// Note: This assumes the points are in ascending order.
// ie 1 * points[0] + 2 * points[1] + ... + n * points[n-1]
#[inline(always)]
fn horners_rule_sum(points: &[G1Point]) -> G1Projective {
    let mut running_sum = G1Projective::identity();
    let mut res = G1Projective::identity();
    points.into_iter().rev().for_each(|b| {
        running_sum += b;
        res += &running_sum;
    });
    res
}

#[cfg(test)]
mod test {

    use crate::{
        fixed_base_msm_pippenger::{
            horners_rule_sum, pippenger_fixed_base_msm as msm_best2, precompute,
        },
        G1Point, G1Projective, Scalar,
    };

    use blstrs::G1Affine;
    use group::{prime::PrimeCurveAffine, Group};

    use super::subsum_accumulation;

    #[test]
    fn subsum_smoke_test() {
        let result = subsum_accumulation(&[1], &[G1Affine::generator()]);
        assert_eq!(G1Projective::generator(), result);

        let result = subsum_accumulation(&[2], &[G1Affine::generator()]);
        assert_eq!(G1Projective::generator() * Scalar::from(2u64), result);

        let result = subsum_accumulation(&[1, 2], &[G1Affine::generator(), G1Affine::generator()]);
        assert_eq!(G1Projective::generator() * Scalar::from(3u64), result);

        let result = subsum_accumulation(&[1, 3], &[G1Affine::generator(), G1Affine::generator()]);
        assert_eq!(G1Projective::generator() * Scalar::from(4u64), result);

        let result =
            subsum_accumulation(&[1, 300], &[-G1Affine::generator(), G1Affine::generator()]);
        assert_eq!(G1Projective::generator() * Scalar::from(299u64), result);

        let result = subsum_accumulation(
            &[1, 2, 3, 4, 10, 22, 100],
            &[
                G1Affine::generator(),
                G1Affine::generator(),
                G1Affine::generator(),
                G1Affine::generator(),
                G1Affine::generator(),
                G1Affine::generator(),
                G1Affine::generator(),
            ],
        );
        assert_eq!(
            G1Projective::generator() * Scalar::from(1 + 2 + 3 + 4 + 10 + 22 + 100),
            result
        );
    }

    fn naive_subsum_accumulation(b: &[u64], s: &[G1Affine]) -> G1Projective {
        let mut res = G1Projective::identity();
        for (scalar, point) in b.iter().zip(s) {
            res += G1Projective::from(point) * Scalar::from(*scalar)
        }
        res
    }

    #[test]
    fn subsum_regression_test() {
        let indices = [2, 3];
        let points = vec![G1Affine::generator(); 2];
        let got = subsum_accumulation(&indices, &points);
        let expected = naive_subsum_accumulation(&indices, &points);
        assert_eq!(got, expected);
    }

    #[test]
    fn horners_sum_smoke_test() {
        let result = horners_rule_sum(&[G1Affine::generator()]);
        assert_eq!(G1Projective::generator(), result);

        let result = horners_rule_sum(&[
            -G1Affine::generator(),
            G1Affine::generator(),
            G1Affine::generator(),
        ]);
        assert_eq!(
            G1Projective::generator() * Scalar::from(3u64)
                + G1Projective::generator() * Scalar::from(2u64)
                + -G1Projective::generator(),
            result
        );
    }

    #[test]
    fn smoke_test_msm_best2() {
        use crate::ff::PrimeField;
        let window_size = 7;
        let number_of_windows = Scalar::NUM_BITS as usize / window_size + 1;

        let precomp_bases = precompute(window_size, number_of_windows, &[G1Point::generator()]);
        let scalar = -Scalar::from(2);

        let res = msm_best2(&[scalar], &precomp_bases, window_size);
        assert_eq!(res, G1Projective::generator() * scalar);
    }

    #[test]
    fn smoke_test_msm_best2_neg() {
        use crate::ff::PrimeField;
        let window_size = 7;
        let number_of_windows = Scalar::NUM_BITS as usize / window_size + 1;

        let input_points = vec![G1Point::generator(), G1Point::generator()];
        let input_scalars = vec![-Scalar::from(1), -Scalar::from(2)];
        let precomp_bases = precompute(window_size, number_of_windows, &input_points);

        let res = msm_best2(&input_scalars, &precomp_bases, window_size);
        assert_eq!(res, naive_msm(&input_points, &input_scalars));
    }

    #[test]
    fn smoke_test_msm_best2_double_scalar() {
        use crate::ff::PrimeField;
        let window_size = 7;
        let number_of_windows = Scalar::NUM_BITS as usize / window_size + 1;

        let point_b: G1Affine = (G1Projective::generator() + G1Projective::generator()).into();
        let point_c: G1Affine =
            (G1Projective::generator().double() + G1Projective::generator().double()).into();
        let input_points = vec![G1Point::generator(), point_b, point_c];
        let input_scalars = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3u64)];
        let precomp_bases = precompute(window_size, number_of_windows, &input_points);

        let res = msm_best2(&input_scalars, &precomp_bases, window_size);
        assert_eq!(res, naive_msm(&input_points, &input_scalars));
    }

    fn naive_msm(points: &[G1Point], scalars: &[Scalar]) -> G1Projective {
        assert!(points.len() == scalars.len());
        let mut result = G1Projective::identity();
        for (scalar, point) in scalars.into_iter().zip(points) {
            result += point * scalar
        }
        result
    }
}
