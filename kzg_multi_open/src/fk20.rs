mod batch_toeplitz;
mod cosets;
pub mod naive;

mod toeplitz;
pub mod verify;

use bls12_381::{
    group::{prime::PrimeCurveAffine, Curve, Group},
    {G1Point, G1Projective, Scalar},
};
use polynomial::{domain::Domain, monomial::PolyCoeff};

use crate::commit_key::{CommitKey, CommitKeyLagrange};
use crate::fk20::{batch_toeplitz::BatchToeplitzMatrixVecMul, toeplitz::ToeplitzMatrix};

pub use cosets::reverse_bit_order;
/// FK20 initializes all of the components needed to compute a KZG multi point
/// proof using the FK20 method.
///
/// The FK20 method gives an efficient algorithm for opening points, where
/// the points are roots of unity. (It cannot be used to open arbitrary points)
///
/// See [Fk21](https://github.com/khovratovich/Kate/blob/master/Kate_amortized.pdf) for details
/// on the scheme.
#[derive(Debug)]
pub struct FK20 {
    batch_toeplitz: BatchToeplitzMatrixVecMul,
    /// FK20 allows you to create a proof of an opening for multiple points.
    /// Each proof will attest to the opening of `l` points.
    /// In the FK20 paper, this is also referred to as `l` (ELL).
    ///
    /// TODO(Note): This has ramifications for the number of G2 points, but it is not checked
    /// TODO: in the constructor here.
    point_set_size: usize,
    /// The total number of points that we want to open a polynomial at.
    ///
    /// Note: A proof will attest to `point_set_size` of these points at a
    /// time.
    number_of_points_to_open: usize,
    /// Domain used in FK20 to create the opening proofs
    proof_domain: Domain,
    ext_domain: Domain,
    /// Domain used for converting polynomial to monomial form.
    poly_domain: Domain,
}

impl FK20 {
    pub fn new(
        commit_key: &CommitKey,
        polynomial_bound: usize,
        point_set_size: usize,
        number_of_points_to_open: usize,
    ) -> FK20 {
        assert!(point_set_size.is_power_of_two());
        assert!(number_of_points_to_open.is_power_of_two());
        assert!(number_of_points_to_open > point_set_size);
        assert!(polynomial_bound.is_power_of_two());

        // 1. Compute the SRS vectors that we will multiply the toeplitz matrices by.
        //
        // Skip the last `l` points in the srs
        assert!(commit_key.g1s.len() > point_set_size);
        let srs_truncated: Vec<_> = commit_key
            .g1s
            .clone()
            .into_iter()
            .rev()
            .skip(point_set_size)
            .collect();
        let mut srs_vectors = take_every_nth(&srs_truncated, point_set_size);

        // TODO: We don't need to do this padding, since `BatchToeplitzMatrixVecMul` doesn't
        // TODO necessitate it.
        // Pad srs vectors by the next power of two
        for srs_vector in &mut srs_vectors {
            let pad_by = srs_vector.len().next_power_of_two();
            srs_vector.resize(pad_by, G1Projective::identity());
        }

        // Compute `l` toeplitz matrix-vector multiplications and sum them together
        let batch_toeplitz = BatchToeplitzMatrixVecMul::new(srs_vectors);

        // 2. Compute the domains needed to produce the proofs and the evaluations
        //
        // The size of the proof domain corresponds to the number of proofs that will be returned.
        let proof_domain = Domain::new(number_of_points_to_open / point_set_size);
        // The size of the extension domain corresponds to the number of points that we want to open
        let ext_domain = Domain::new(number_of_points_to_open);

        let poly_domain = Domain::new(polynomial_bound);

        FK20 {
            batch_toeplitz,
            point_set_size,
            number_of_points_to_open,
            proof_domain,
            ext_domain,
            poly_domain,
        }
    }

    /// Commit to the data that we will be creating Fk20 proofs over.
    //
    // Note: Currently this is the only place we use the lagrange form of the commitment key.
    // We could get rid of it entirely, at the cost of an IDFT.
    pub fn commit_to_data(commit_key: &CommitKeyLagrange, mut data: Vec<Scalar>) -> G1Point {
        // Reverse the order of the scalars, so that they are in bit-reversed order.
        //
        // FK20 will operate over the bit-reversed permutation of the data.
        reverse_bit_order(&mut data);

        // Commit to the bit reversed data in lagrange form using the lagrange version of the commit key
        commit_key.commit_g1(&data).into()
    }

    /// The number of proofs that will be produced.
    pub fn num_proofs(&self) -> usize {
        self.number_of_points_to_open / self.point_set_size
    }

    pub fn compute_multi_opening_proofs(&self, polynomial: PolyCoeff) -> Vec<G1Point> {
        // Compute proofs for the polynomial
        let h_poly_commitments =
            self.compute_h_poly_commitments(polynomial.clone(), self.point_set_size);
        let mut proofs = self.proof_domain.fft_g1(h_poly_commitments);
        // apply reverse bit order permutation, since fft_g1 was applied using
        // the regular order.
        // TODO: move this to eip7594 module
        // TODO: same for evaluation sets -- we could then move evaluation sets to reed solomon crate
        reverse_bit_order(&mut proofs);
        let mut proofs_affine = vec![G1Point::identity(); proofs.len()];
        // TODO: This does not seem to be using the batch affine trick
        bls12_381::G1Projective::batch_normalize(&proofs, &mut proofs_affine);

        proofs_affine
    }

    pub fn compute_multi_opening_proofs_on_data(
        &self,
        mut data: Vec<Scalar>,
    ) -> (Vec<G1Point>, Vec<Vec<Scalar>>) {
        reverse_bit_order(&mut data);
        let poly_coeff = self.poly_domain.ifft_scalars(data);
        // TODO: possibly revert change and have `compute_multi_opening_proofs` return the evaluations too
        let proofs = self.compute_multi_opening_proofs(poly_coeff.clone());
        let evaluation_sets = self.compute_evaluation_sets(poly_coeff);
        (proofs, evaluation_sets)
    }

    pub fn compute_evaluation_sets(&self, polynomial: PolyCoeff) -> Vec<Vec<Scalar>> {
        // Compute the evaluations of the polynomial on the cosets by doing an fft
        let mut evaluations = self.ext_domain.fft_scalars(polynomial);
        reverse_bit_order(&mut evaluations);
        evaluations
            .chunks_exact(self.point_set_size)
            .map(|slice| slice.to_vec())
            .collect()
    }

    fn compute_h_poly_commitments(&self, mut polynomial: PolyCoeff, l: usize) -> Vec<G1Projective> {
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

        // Reverse polynomial so highest coefficient is first.
        // See 3.1.1 of the FK20 paper, for the ordering.
        polynomial.reverse();

        // Compute the toeplitz rows for the `l` toeplitz matrices
        let toeplitz_rows = take_every_nth(&polynomial, l);

        // Compute the Toeplitz matrices
        let mut matrices = Vec::with_capacity(toeplitz_rows.len());
        // We want to do `l` toeplitz matrix multiplications
        for row in toeplitz_rows.into_iter() {
            let mut toeplitz_column = vec![Scalar::from(0u64); row.len()];
            toeplitz_column[0] = row[0];

            matrices.push(ToeplitzMatrix::new(row, toeplitz_column));
        }

        // Compute `l` toeplitz matrix-vector multiplications and sum them together
        self.batch_toeplitz.sum_matrix_vector_mul(matrices)
    }
}

/// Given a vector `k` and an integer `l`
/// Where `l` is less than |k|. We return `l-downsampled` groups.
/// Example: k = [a_0, a_1, a_3, a_4, a_5, a_6], l = 2
/// Result = [[a_0, a_3, a_5], [a_1, a_4, a_6]]
#[inline(always)]
fn take_every_nth<T: Clone + Copy>(list: &[T], n: usize) -> Vec<Vec<T>> {
    (0..n)
        .map(|i| list.iter().copied().skip(i).step_by(n).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        create_eth_commit_opening_keys,
        fk20::{naive, take_every_nth, FK20},
    };
    use bls12_381::group::Group;
    use bls12_381::Scalar;
    use polynomial::domain::Domain;

    #[test]
    fn smoke_test_downsample() {
        let k = vec![5, 4, 3, 2];
        let downsampled_lists = take_every_nth(&k, 2);
        let result = vec![vec![5, 3], vec![4, 2]];
        assert_eq!(downsampled_lists, result)
    }

    #[test]
    fn check_consistency_of_toeplitz_h_polys() {
        use bls12_381::ff::Field;
        let poly = vec![Scalar::random(&mut rand::thread_rng()); 4096];
        let l = 64;
        let (commit_key, _) = create_eth_commit_opening_keys();

        let h_polynomials = naive::compute_h_poly(&poly, l);
        let mut expected_comm_h_polys = h_polynomials
            .iter()
            .map(|h_poly| commit_key.commit_g1(h_poly))
            .collect::<Vec<_>>();
        // Add the identity element to h_polys to pad it to a power of two
        expected_comm_h_polys.push(bls12_381::G1Projective::identity());
        let fk20 = FK20::new(&commit_key, 4096, l, 2 * 4096);
        let got_comm_h_polys = fk20.compute_h_poly_commitments(poly, l);
        assert_eq!(expected_comm_h_polys.len(), got_comm_h_polys.len());
        assert_eq!(expected_comm_h_polys, got_comm_h_polys);
    }

    #[test]
    fn check_consistency_of_proofs_against_naive() {
        use bls12_381::ff::Field;
        let poly_len = 4096;
        let poly = vec![Scalar::random(&mut rand::thread_rng()); poly_len];
        let l = 64;
        let (commit_key, _) = create_eth_commit_opening_keys();
        let proof_domain = Domain::new(2 * poly_len / l);
        let ext_domain = Domain::new(2 * poly_len);

        let expected_proofs = naive::fk20_open_multi_point(&commit_key, &proof_domain, &poly, l);
        let expected_evaluations = naive::fk20_compute_evaluation_set(&poly, l, ext_domain);

        let fk20 = FK20::new(&commit_key, poly_len, l, 2 * poly_len);
        let got_proofs = fk20.compute_multi_opening_proofs(poly.clone());
        let got_evaluations = fk20.compute_evaluation_sets(poly);

        assert_eq!(got_proofs.len(), expected_proofs.len());
        assert_eq!(got_evaluations.len(), expected_evaluations.len());

        assert_eq!(got_evaluations, expected_evaluations);
        assert_eq!(got_proofs, expected_proofs);
    }
}
