use bls12_381::{lincomb::g1_lincomb, traits::*, G1Point, Scalar};
use polynomial::domain::Domain;

/// The key that is used to commit to polynomials in monomial form.
pub struct CommitKey {
    pub g1s: Vec<G1Point>,
}

impl CommitKey {
    pub const fn new(g1s: Vec<G1Point>) -> Self {
        Self { g1s }
    }
}

pub struct Prover {
    /// Domain used to create the opening proofs.
    pub domain: Domain,
    /// Commitment key used for committing to the polynomial
    /// in monomial form
    pub commit_key: CommitKey,
}

impl Prover {
    pub fn new(domain_size: usize, commit_key: CommitKey) -> Self {
        Self {
            domain: Domain::new(domain_size),
            commit_key,
        }
    }

    pub fn compute_kzg_proof(&self, polynomial: &[Scalar], z: Scalar) -> (G1Point, Scalar) {
        // Compute evaluation and quotient at point `z`.
        // The quotient is in "normal order"
        let (quotient, y) = divide_by_linear(polynomial, z);

        // Compute KZG opening proof.
        let proof = g1_lincomb(&self.commit_key.g1s[..quotient.len()], &quotient)
            .expect("commit_key.g1s[..quotient.len()].len() == quotient.len()")
            .to_affine();

        (proof, y)
    }
}

/// Divides poly by X-Z using ruffini's rule, and returns quotient and reminder.
fn divide_by_linear(poly: &[Scalar], z: Scalar) -> (Vec<Scalar>, Scalar) {
    let mut quotient: Vec<Scalar> = Vec::with_capacity(poly.len());
    let mut k = Scalar::from(0u64);

    for coeff in poly.iter().rev() {
        let t = *coeff + k;
        quotient.push(t);
        k = z * t;
    }

    // Pop off the remainder term
    let remainder = quotient.pop().expect("!quotient.is_empty()");

    // Reverse the results as monomial form stores coefficients starting with lowest degree
    quotient.reverse();

    (quotient, remainder)
}
