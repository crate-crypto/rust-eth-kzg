use crate::group::Group;
use crate::{G1Projective, G2Projective, Scalar};

// A multi-scalar multiplication algorithm over G1 elements
pub fn g1_lincomb_unsafe(points: &[G1Projective], scalars: &[Scalar]) -> G1Projective {
    // TODO: Spec says we should panic, but as a lib its better to return result
    assert_eq!(points.len(), scalars.len());

    // blst does not use multiple threads
    // This method as a whole seems to be non-optimal.
    // TODO: We should implement the naive bucket method and see if it is faster
    G1Projective::multi_exp(points, scalars)
}

// A multi-scalar multiplication algorithm over G2 elements
pub fn g2_lincomb_unsafe(points: &[G2Projective], scalars: &[Scalar]) -> G2Projective {
    assert_eq!(points.len(), scalars.len());
    G2Projective::multi_exp(points, scalars)
}

/// This method is a safe wrapper around `g1_lincomb_unsafe`
/// It filters out any points that are the identity, since blst does not handle them correctly
pub fn g1_lincomb(points: &[G1Projective], scalars: &[Scalar]) -> G1Projective {
    let mut points_filtered = Vec::with_capacity(points.len());
    let mut scalars_filtered = Vec::with_capacity(scalars.len());
    for (point, scalar) in points.iter().zip(scalars) {
        let is_identity: bool = point.is_identity().into();
        if !is_identity {
            points_filtered.push(*point);
            scalars_filtered.push(*scalar);
        }
    }
    if points_filtered.is_empty() {
        return G1Projective::identity();
    }
    g1_lincomb_unsafe(&points_filtered, &scalars_filtered)
}
/// This method is a safe wrapper around `g2_lincomb_unsafe`
/// It filters out any points that are the identity, since blst does not handle them correctly
pub fn g2_lincomb(points: &[G2Projective], scalars: &[Scalar]) -> G2Projective {
    let mut points_filtered = Vec::with_capacity(points.len());
    let mut scalars_filtered = Vec::with_capacity(scalars.len());
    for (point, scalar) in points.iter().zip(scalars) {
        let is_identity: bool = point.is_identity().into();
        if !is_identity {
            points_filtered.push(*point);
            scalars_filtered.push(*scalar);
        }
    }
    if points_filtered.is_empty() {
        return G2Projective::identity();
    }
    g2_lincomb_unsafe(&points_filtered, &scalars_filtered)
}

#[cfg(test)]
mod tests {
    use crate::ff::Field;
    use crate::group::Group;
    use crate::{G1Projective, Scalar};

    use crate::lincomb::{g1_lincomb, g1_lincomb_unsafe};

    #[test]
    fn blst_footgun() {
        let points = vec![G1Projective::generator(), G1Projective::identity()];
        let scalars = vec![Scalar::ONE, Scalar::ONE];

        // Ideally, we expect the answer to be:
        // 1 * G + 1 * 0 = G
        // However, since one of the points is the identity, the answer is 0 for blst

        let result = g1_lincomb_unsafe(&points, &scalars);
        assert_eq!(result, G1Projective::identity());

        // Doing it with the g1_lincomb method will give the correct result
        let result = g1_lincomb(&points, &scalars);
        assert_eq!(result, G1Projective::generator());
    }
}
