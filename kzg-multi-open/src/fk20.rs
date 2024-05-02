// [FK20] is a paper by Dmitry Khovratovich and Dankrad Feist that describes a method for
// efficiently opening a set of points when the opening points are roots of unity.

mod batch_toeplitz;
mod toeplitz;

use bls12_381::group::prime::PrimeCurveAffine;
use bls12_381::group::Curve;
use bls12_381::group::Group;
use bls12_381::{G1Point, G1Projective, Scalar};
use polynomial::{domain::Domain, monomial::PolyCoeff};

use crate::fk20::batch_toeplitz::BatchToeplitzMatrixVecMul;
use crate::fk20::toeplitz::ToeplitzMatrix;
use crate::{commit_key::CommitKey, reverse_bit_order};

/// This is doing \floor{f(x) / x^d}
/// which essentially means removing the first d coefficients
///
/// Note: This is just doing a shifting of the polynomial coefficients. However,
/// we refrain from calling this method `shift_polynomial` due to the specs
/// naming a method with different functionality that name.
pub fn divide_by_monomial_floor(poly: &PolyCoeff, degree: usize) -> &[Scalar] {
    let n = poly.len();
    // If the degree of the monomial is greater than or equal to the number of coefficients,
    // the division results in the zero polynomial
    assert!(
        degree < n,
        "degree should be less than the number of coefficients"
    );
    &poly[degree..]
}

/// Naively compute the `h`` polynomials for the FK20 proof.
///
/// See section 3.1.1 of the FK20 paper for more details.
///
/// FK20 computes the commitments to these polynomials in 3.1.1.
pub fn naive_compute_h_poly(polynomial: &PolyCoeff, l: usize) -> Vec<&[Scalar]> {
    assert!(
        l.is_power_of_two(),
        "expected l to be a power of two (its the size of the cosets), found {}",
        l
    );

    let m = polynomial.len();
    assert!(
        m.is_power_of_two(),
        "expected polynomial to have power of 2 number of evaluations. Found {}",
        m
    );
    let k: usize = m / l;
    assert!(
        k.is_power_of_two(),
        "expected k to be a power of two, found {}",
        k
    );

    let mut h_polys = Vec::with_capacity(k - 1);
    for index in 1..k {
        let degree = index * l;
        let h_poly_i = divide_by_monomial_floor(polynomial, degree);
        h_polys.push(h_poly_i);
    }

    assert!(h_polys.len() == k - 1);

    h_polys
}

/// Computes FK20 proofs over multiple cosets without using a toeplitz matrix.
/// of the `h` polynomials and MSMs for computing the proofs.
pub fn naive_fk20_open_multi_point(
    commit_key: &CommitKey,
    proof_domain: &Domain,
    ext_domain: &Domain,
    polynomial: &PolyCoeff,
    l: usize,
) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
    let h_polys = naive_compute_h_poly(polynomial, l);
    let commitment_h_polys = h_polys
        .iter()
        .map(|h_poly| commit_key.commit_g1(h_poly))
        .collect::<Vec<_>>();
    let proofs = proof_domain.fft_g1(commitment_h_polys.clone());

    let mut proofs_affine = vec![G1Point::identity(); proofs.len()];
    // TODO: This does not seem to be using the batch affine trick
    bls12_381::G1Projective::batch_normalize(&proofs, &mut proofs_affine);

    // Compute the evaluations of the polynomial at the cosets by doing an fft
    let mut evaluations = ext_domain.fft_scalars(polynomial.clone());
    reverse_bit_order(&mut evaluations);
    let set_of_output_points: Vec<_> = evaluations
        .chunks_exact(l)
        .map(|slice| slice.to_vec())
        .collect();

    // reverse the order of the proofs, since fft_g1 was applied using
    // the regular order.
    reverse_bit_order(&mut proofs_affine);

    (proofs_affine, set_of_output_points)
}

// This is in the spirit of the toeplitz construction, but does not use circulant matrices yet.
// Checking it into github for prosperity purposes and for those looking to learn.
//
// This function will be slow because it is doing a matrix-vector multiplication for the toeplitz
// matrices.
fn semi_toeplitz_fk20_h_polys(
    commit_key: &CommitKey,
    polynomial: &[Scalar],
    l: usize,
) -> Vec<G1Projective> {
    assert!(
        l.is_power_of_two(),
        "expected l to be a power of two (its the size of the cosets), found {}",
        l
    );

    let m = polynomial.len();
    assert!(
        m.is_power_of_two(),
        "expected polynomial to have power of 2 number of evaluations. Found {}",
        m
    );

    let k = m / l;
    assert!(
        k.is_power_of_two(),
        "expected k to be a power of two, found {}",
        k
    );

    // Compute toeplitz rows for the h_polys
    let mut polynomial = polynomial.to_vec();
    polynomial.reverse();

    let toeplitz_rows = take_every_nth(&polynomial, l);
    let toeplitz_rows: Vec<Vec<_>> = toeplitz_rows
        .into_iter()
        .map(|v| v.into_iter().cloned().collect())
        .collect();

    // Skip the last `l` points in the srs
    let srs_truncated: Vec<_> = commit_key.g1s.clone().into_iter().rev().skip(l).collect();
    let srs_vectors = take_every_nth(&srs_truncated, l);

    // TODO: remove, this is just a .cloned() method since g1_lincomb doesn't take reference
    let mut srs_vectors: Vec<Vec<_>> = srs_vectors
        .into_iter()
        .map(|v| v.into_iter().cloned().collect())
        .collect();

    // Pad srs vectors by the next power of two
    // TODO: prove that length is always l-1 and then we can just pad to `l` or add one identity element
    for srs_vector in &mut srs_vectors {
        let pad_by = srs_vector.len().next_power_of_two();
        srs_vector.resize(pad_by, G1Projective::identity());
    }
    let mut matrices = Vec::with_capacity(toeplitz_rows.len());

    // We want to do `l` toeplitz matrix multiplications
    for row in toeplitz_rows.into_iter() {
        // TODO: We could have a special constructor/Toeplitz struct for the column,
        // TODO: if this allocation shows to be non-performant.
        let mut toeplitz_column = vec![Scalar::from(0u64); row.len()];
        toeplitz_column[0] = row[0];

        matrices.push(ToeplitzMatrix::new(row, toeplitz_column));
    }

    // TODO: This `BatchToeplitzMatrixVecMul`can be cached and reused for multiple proofs
    let bm = BatchToeplitzMatrixVecMul::new(srs_vectors);
    bm.sum_matrix_vector_mul(matrices)
}

/// Given a vector `k` and an integer `l`
/// Where `l` is less than |k|. We return `l-downsampled` groups.
/// Example: k = [a_0, a_1, a_3, a_4, a_5, a_6], l = 2
/// Result = [[a_0, a_3, a_5], [a_1, a_4, a_6]]
#[inline(always)]
fn take_every_nth<T>(list: &[T], n: usize) -> Vec<Vec<&T>> {
    (0..n)
        .map(|i| list.iter().skip(i).step_by(n).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        create_eth_commit_opening_keys,
        fk20::{
            divide_by_monomial_floor, naive_compute_h_poly, semi_toeplitz_fk20_h_polys,
            take_every_nth,
        },
    };
    use bls12_381::group::Group;
    use bls12_381::Scalar;

    #[test]
    fn smoke_test_downsample() {
        let k = vec![5, 4, 3, 2];
        let downsampled_lists = take_every_nth(&k, 2);
        let result = vec![vec![&5, &3], vec![&4, &2]];
        assert_eq!(downsampled_lists, result)
    }

    #[test]
    fn check_divide_by_monomial_floor() {
        // \floor(x^2 + x + 10 / x) = x + 1
        let poly = vec![Scalar::from(10u64), Scalar::from(1u64), Scalar::from(1u64)];
        let result = divide_by_monomial_floor(&poly, 1);
        assert_eq!(result, vec![Scalar::from(1u64), Scalar::from(1u64)]);
    }

    #[test]
    fn check_consistency_of_toeplitz_h_polys() {
        use bls12_381::ff::Field;
        let poly = vec![Scalar::random(&mut rand::thread_rng()); 4096];
        let l = 64;
        let (commit_key, _) = create_eth_commit_opening_keys();

        let h_polynomials = naive_compute_h_poly(&poly, l);
        let mut expected_comm_h_polys = h_polynomials
            .iter()
            .map(|h_poly| commit_key.commit_g1(h_poly))
            .collect::<Vec<_>>();
        // Add the identity element to h_polys to pad it to a power of two
        expected_comm_h_polys.push(bls12_381::G1Projective::identity());
        let got_comm_h_polys = semi_toeplitz_fk20_h_polys(&commit_key, &poly, l);
        assert_eq!(expected_comm_h_polys.len(), got_comm_h_polys.len());
        assert_eq!(expected_comm_h_polys, got_comm_h_polys);
    }
}
