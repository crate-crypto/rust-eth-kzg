use bls12_381::G1Point;
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
}
