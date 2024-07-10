use crate::group::Group;
use crate::{G1Projective, G2Projective, Scalar};

/// A multi-scalar multiplication algorithm over G1 elements
///
/// Note: unsafe refers to the fact that blst will return the identity
/// element, if any of the points are the identity element.
///
/// Calling this method means that the caller is aware that there are no
/// identity elements amongst their points.
///
/// See test below named `blst_footgun` for the edge case.
pub fn g1_lincomb_unsafe(points: &[G1Projective], scalars: &[Scalar]) -> Option<G1Projective> {
    if points.len() != scalars.len() {
        return None;
    }
    Some(G1Projective::multi_exp(points, scalars))
}

/// A multi-scalar multiplication algorithm over G2 elements
///
/// Returns None if the points and the scalars are not the
/// same length.
///
/// Note: unsafe refers to the fact that blst will return the identity
/// element, if any of the points are the identity element.
///
/// Calling this method means that the caller is aware that there are no
/// identity elements amongst their points.
///
/// See test below named `blst_footgun` for the edge case.
pub fn g2_lincomb_unsafe(points: &[G2Projective], scalars: &[Scalar]) -> Option<G2Projective> {
    if points.len() != scalars.len() {
        return None;
    }
    Some(G2Projective::multi_exp(points, scalars))
}

/// A multi-scalar multiplication algorithm over G1 elements
///
/// Returns None if the points and the scalars are not the
/// same length.
///
/// This method is a safe wrapper around `g1_lincomb_unsafe`.
///
/// It filters out any points that are the identity.
// TODO: Make this take either G1Point or Into<G1Projective> (Ideally the latter)
pub fn g1_lincomb(points: &[G1Projective], scalars: &[Scalar]) -> Option<G1Projective> {
    let (points_filtered, scalars_filtered): (Vec<_>, Vec<_>) = points
        .iter()
        .zip(scalars)
        .filter(|(point, _)| !(bool::from(point.is_identity())))
        .map(|(point, scalar)| (*point, *scalar))
        .unzip();
    if points_filtered.is_empty() {
        return Some(G1Projective::identity());
    }
    g1_lincomb_unsafe(&points_filtered, &scalars_filtered)
}

/// A multi-scalar multiplication algorithm over G2 elements
///
/// Returns None if the points and the scalars are not the
/// same length.
///
/// This method is a safe wrapper around `g2_lincomb_unsafe`.
///
/// It filters out any points that are the identity.
pub fn g2_lincomb(points: &[G2Projective], scalars: &[Scalar]) -> Option<G2Projective> {
    let (points_filtered, scalars_filtered): (Vec<_>, Vec<_>) = points
        .iter()
        .zip(scalars)
        .filter(|(point, _)| !(bool::from(point.is_identity())))
        .map(|(point, scalar)| (*point, *scalar))
        .unzip();
    if points_filtered.is_empty() {
        return Some(G2Projective::identity());
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

        let result = g1_lincomb_unsafe(&points, &scalars)
            .expect("number of points and number of scalars should be equal");
        assert_eq!(result, G1Projective::identity());

        // Doing it with the g1_lincomb method will give the correct result
        let result = g1_lincomb(&points, &scalars)
            .expect("number of points and number of scalars should be equal");
        assert_eq!(result, G1Projective::generator());
    }
}
