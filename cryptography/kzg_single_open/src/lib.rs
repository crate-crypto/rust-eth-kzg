use bls12_381::{multi_pairings, G1Point, G2Point, G2Prepared, Scalar};
use kzg_multi_open::{commit_key::CommitKey, opening_key::OpeningKey};
use polynomial::{domain::Domain, monomial::PolyCoeff};

pub struct Proof {
    pub quotient_commitment: bls12_381::G1Point,
    pub claimed_evaluation: Scalar,
}

/// Takes a polynomial in lagrange form in reverse bit order and an input point
/// and computes a proof that the polynomial is correctly evaluated at the input point.
///
// Note: The total time taken is around 40-50ms on a single thread. 2-3ms of this is
// division, fft and poly_eval. The rest of it is committing.
pub fn compute_proof(
    commit_key: &CommitKey,
    domain: &Domain,
    polynomial_lagrange: &[Scalar],
    input_point: Scalar,
) -> Proof {
    // Bit reverse the polynomial and interpolate it.
    //
    // The bit-reversal is an artifact of a feature we want to maintain
    // when we use FK20.
    let mut poly_lagrange = polynomial_lagrange.to_vec();
    reverse_bit_order(&mut poly_lagrange);
    let polynomial_coeff = domain.ifft_scalars(poly_lagrange);

    let quotient_poly = divide_by_linear(&polynomial_coeff, input_point);
    let quotient_commitment = commit_key.commit_g1(quotient_poly.as_slice());
    let claimed_evaluation = poly_eval(&polynomial_coeff, &input_point);

    Proof {
        quotient_commitment: quotient_commitment.into(),
        claimed_evaluation,
    }
}

pub(crate) fn reverse_bits(n: usize, bits: u32) -> usize {
    let mut n = n;
    let mut r = 0;
    for _ in 0..bits {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

/// Computes log2 of an integer.
///
/// Panics if the integer is not a power of two
pub(crate) fn log2(x: u32) -> u32 {
    assert!(x > 0 && x.is_power_of_two(), "x must be a power of two.");
    x.trailing_zeros()
}

// Taken and modified from: https://github.com/filecoin-project/ec-gpu/blob/bdde768d0613ae546524c5612e2ad576a646e036/ec-gpu-gen/src/fft_cpu.rs#L10C8-L10C18
pub fn reverse_bit_order<T>(a: &mut [T]) {
    let n = a.len() as u32;
    assert!(n.is_power_of_two(), "n must be a power of two");
    let log_n = log2(n);

    for k in 0..n {
        let rk = reverse_bits(k as usize, log_n) as u32;
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }
}

/// Checks that a polynomial `p` was evaluated at a point `z` and returned the value specified `y`.
/// ie. y = p(z).
pub fn verify(
    opening_key: OpeningKey,
    input_point: Scalar,
    output_point: Scalar,
    poly_comm: G1Point,
    witness_comm: G1Point,
) -> bool {
    // For scalar muls could also do precomputations
    let inner_a: G1Point = (poly_comm - (opening_key.g1s[0] * output_point)).into();
    let inner_b: G2Point = (opening_key.g2s[1] - (opening_key.g2s[0] * input_point)).into();
    let prepared_inner_b = G2Prepared::from(-inner_b);

    let g2_gen_affine: G2Point = opening_key.g2s[0].into();
    let prepared_g2_gen = G2Prepared::from(g2_gen_affine);

    multi_pairings(&[
        (&inner_a, &prepared_g2_gen),
        (&witness_comm, &prepared_inner_b),
    ])
}

pub fn poly_eval(poly: &PolyCoeff, value: &Scalar) -> Scalar {
    let mut result = Scalar::from(0u64);
    for coeff in poly.iter().rev() {
        result = result * value + coeff;
    }
    result
}

/// Division using ruffini's rule
fn divide_by_linear(poly: &[Scalar], z: Scalar) -> Vec<Scalar> {
    let mut quotient: Vec<Scalar> = Vec::with_capacity(poly.len());
    let mut k = Scalar::from(0u64);

    for coeff in poly.iter().rev() {
        let t = *coeff + k;
        quotient.push(t);
        k = z * t;
    }

    // Pop off the remainder term
    quotient.pop();

    // Reverse the results as monomial form stores coefficients starting with lowest degree
    quotient.reverse();
    quotient
}
