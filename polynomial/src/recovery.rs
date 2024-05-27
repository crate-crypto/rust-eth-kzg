use bls12_381::{batch_inversion::batch_inverse, Scalar};

use crate::{domain::Domain, monomial::vanishing_poly};
use bls12_381::ff::Field;

// The erasures can be either indices of the polynomial
// or groups of indices
pub enum Erasures {
    Indices(Vec<usize>),
    Cells { cell_size: usize, cells: Vec<usize> },
}

pub struct ReedSolomon {
    expansion_factor: usize,
    poly_len: usize,
    domain_extended: Domain,
}

impl ReedSolomon {
    pub fn new(poly_len: usize, expansion_factor: usize) -> Self {
        let domain_extended = Domain::new(poly_len * expansion_factor);
        Self {
            poly_len,
            domain_extended,
            expansion_factor,
        }
    }

    /// We need to have at least `poly_len` evaluations
    pub fn acceptable_num_errors(&self) -> usize {
        let total_codeword_len = self.poly_len * self.expansion_factor;
        let min_num_evaluations_needed = self.poly_len;
        total_codeword_len - min_num_evaluations_needed
    }

    /// Reed solomon encodes a polynomial by evaluating it at `expansion_factor`
    /// more points than is needed.
    pub fn encode(&self, poly_coefficient_form: Vec<Scalar>) -> Vec<Scalar> {
        if poly_coefficient_form.len() > self.poly_len {
            panic!(
                "The polynomial must have a size of {}, found {}",
                self.poly_len,
                poly_coefficient_form.len()
            )
        }
        self.domain_extended.fft_scalars(poly_coefficient_form)
    }

    pub fn recover_polynomial_codeword(
        &self,
        codeword_with_errors: Vec<Scalar>,
        missing_indices: Erasures,
    ) -> Vec<Scalar> {
        recover_polynomial_evaluations_erasures(
            &self.domain_extended,
            codeword_with_errors,
            missing_indices,
        )
    }

    pub fn recover_polynomial_codeword_cells_erasures(
        &self,
        codeword_with_errors: Vec<Scalar>,
        missing_indices: Erasures,
    ) -> Vec<Scalar> {
        recover_polynomial_evaluations_erasures(
            &self.domain_extended,
            codeword_with_errors,
            missing_indices,
        )
    }
}

/// Given a set of evaluations and a list of its missing indices,
/// This method will return the polynomial in coefficient form
/// with the missing indices filled in (recovered).
fn recover_polynomial_coefficient_erasures(
    domain_extended: &Domain,
    data_eval: Vec<Scalar>,
    missing_indices: Erasures,
) -> Vec<Scalar> {
    // Compute Z(X) which is the polynomial that vanishes on all
    // of the missing points
    let z_x = construct_vanishing_poly_from_erasures(missing_indices, &domain_extended);

    // Compute Z(X)_eval which is the vanishing polynomial evaluated
    // at the missing points
    let z_x_eval = domain_extended.fft_scalars(z_x.clone());

    // Compute (D * Z)(X) or (E * Z)(X) (same polynomials)
    let ez_eval: Vec<_> = z_x_eval
        .iter()
        .zip(data_eval)
        .map(|(zx, d)| zx * d)
        .collect();

    let dz_poly = domain_extended.ifft_scalars(ez_eval);

    let mut coset_z_x_eval = domain_extended.coset_fft_scalars(z_x);
    let coset_dz_eval = domain_extended.coset_fft_scalars(dz_poly);
    batch_inverse(&mut coset_z_x_eval);
    let coset_quotient_eval: Vec<_> = coset_dz_eval
        .iter()
        .zip(coset_z_x_eval)
        .map(|(d, zx_inv)| d * zx_inv)
        .collect();

    domain_extended.coset_ifft_scalars(coset_quotient_eval)
}

fn recover_polynomial_evaluations_erasures(
    domain_extended: &Domain,
    evaluations: Vec<Scalar>,
    missing_indices: Erasures,
) -> Vec<Scalar> {
    let polynomial_coeff =
        recover_polynomial_coefficient_erasures(domain_extended, evaluations, missing_indices);

    domain_extended.fft_scalars(polynomial_coeff)
}

fn construct_vanishing_poly_from_erasures(
    erasures: Erasures,
    domain_extended: &Domain,
) -> Vec<Scalar> {
    // Encode the data by doing an fft
    const CELLS_PER_EXT_BLOB: usize = 128;
    const FIELD_ELEMENTS_PER_EXT_BLOB: usize = 8192;
    const FIELD_ELEMENTS_PER_CELL: usize = 64;

    match erasures {
        Erasures::Indices(indices) => {
            let z_x_missing_indices_roots: Vec<_> = indices
                .iter()
                .map(|index| domain_extended.roots[*index])
                .collect();

            vanishing_poly(&z_x_missing_indices_roots)
        }
        Erasures::Cells { cell_size, cells } => {
            let domain = Domain::new(CELLS_PER_EXT_BLOB);

            let z_x_missing_indices_roots: Vec<_> =
                cells.iter().map(|index| domain.roots[*index]).collect();
            let short_zero_poly = vanishing_poly(&z_x_missing_indices_roots);

            let mut z_x = vec![Scalar::ZERO; FIELD_ELEMENTS_PER_EXT_BLOB];
            for (i, coeff) in short_zero_poly.into_iter().enumerate() {
                z_x[i * FIELD_ELEMENTS_PER_CELL] = coeff;
            }
            z_x
        }
    }
}

#[test]
fn smoke_test_recovery() {
    let poly_coeff = vec![
        Scalar::from(1),
        Scalar::from(2),
        Scalar::from(4),
        Scalar::from(8),
    ];

    let domain = Domain::new(poly_coeff.len() * 2);
    let mut evaluations = domain.fft_scalars(poly_coeff.clone());

    // add errors into evaluations
    evaluations[1] = Scalar::from(0);
    evaluations[2] = Scalar::from(0);

    let _recovered_polynomial = recover_polynomial_coefficient_erasures(
        &domain,
        evaluations,
        Erasures::Indices(vec![1, 2]),
    );
}

#[test]
fn smoke_test_recovery_no_errors() {
    let rs = ReedSolomon::new(16, 2);
    let poly_coeff = vec![Scalar::random(&mut rand::thread_rng()); 15];

    let codewords = rs.encode(poly_coeff);
    assert_eq!(codewords.len(), 32);
    let got_codewords =
        rs.recover_polynomial_codeword(codewords.clone(), Erasures::Indices(vec![]));

    assert_eq!(got_codewords, codewords);
}

#[test]
fn smoke_test_recovery_upto_num_acceptable_errors() {
    let poly_len = 16;
    let expansion_factor = 2;
    let rs = ReedSolomon::new(poly_len, expansion_factor);
    let poly_coeff = (0..poly_len)
        .map(|i| Scalar::from(i as u64))
        .collect::<Vec<_>>();

    let original_codewords = rs.encode(poly_coeff);
    let acceptable_num_errors: Vec<_> = (0..rs.acceptable_num_errors()).collect();
    for num_errors in acceptable_num_errors {
        let mut codewords_with_errors = original_codewords.clone();

        // zero out `num_errors` amount of evaluations to simulate errors
        let mut missing_indices = Vec::new();
        for index in 0..num_errors {
            codewords_with_errors[index] = Scalar::from(0);
            missing_indices.push(index);
        }

        let recovered_codewords = rs
            .recover_polynomial_codeword(codewords_with_errors, Erasures::Indices(missing_indices));
        assert_eq!(recovered_codewords, original_codewords)
    }
}
