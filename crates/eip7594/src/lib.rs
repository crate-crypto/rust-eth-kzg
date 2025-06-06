mod eip4844_methods;
mod errors;
mod prover;
mod recovery;
mod trusted_setup;
mod verifier;

// Exported types
//
pub use bls12_381::fixed_base_msm::UsePrecomp;
pub use errors::Error;
pub use serialization::{constants, types::*};
/// TrustedSetup contains the Structured Reference String(SRS)
/// needed to make and verify proofs.
pub use trusted_setup::TrustedSetup;

/// `CellIndex` is reference to the coset/set of points that were used to create that Cell,
/// on a particular polynomial, f(x).
///
/// Note: Since the verifier and prover both know what cosets will be used
/// to evaluate the polynomials being used in opening proofs, the protocol
/// only requires an index to reference them.
pub type CellIndex = kzg_multi_open::CosetIndex;

use prover::ProverContext;
use verifier::VerifierContext;

/// An enum to specify whether we want to prove and verify or just verify
pub enum Mode {
    /// Initialize both the prover and verifier
    Both(UsePrecomp),
    /// Only initialize the verifier. Methods like blob_to_kzg_commitment will not be available
    VerifierOnly,
}

/// DASContext manages the shared environment for creating and
/// verifying KZG cell proofs used in PeerDAS (EIP-7594).
///
/// It holds:
/// - The EIP-7594 prover context (for generating proofs)
/// - The EIP-7594 verifier context (for checking proofs)
/// - The EIP-4844 context (for basic KZG operations). This is re-exported for convenience.
///
/// All initialized from the same trusted setup (SRS).
///
/// The EIP-7594 context is required for sampling and validating data
/// availability across blobs and cells without downloading all of the data.
#[derive(Debug)]
pub struct DASContext {
    /// Prover-side context:
    /// prepares and generates KZG cell proofs for blobs and cells.
    pub prover_ctx: Option<ProverContext>,

    /// Verifier-side context:
    /// verifies KZG cell proofs and ensures data integrity in PeerDAS.
    pub verifier_ctx: VerifierContext,

    /// EIP-4844 context:
    /// provides core KZG commitment operations for blob verification (proto-danksharding variant)
    eip4844_ctx: eip4844::Context,
}

impl Default for DASContext {
    fn default() -> Self {
        Self::new(&TrustedSetup::default(), Mode::Both(UsePrecomp::No))
    }
}

impl DASContext {
    /// Creates a new DASContext with both prover and verifier
    /// initialized from the given trusted setup (SRS).
    ///
    /// This context is used for generating and verifying KZG cell
    /// proofs as part of PeerDAS (EIP-7594), which enables
    /// data availability sampling without downloading all blob data.
    ///
    /// The `use_precomp` parameter controls whether prover-side
    /// precomputations are enabled. Enabling precomputations
    /// (typically with width 8) increases memory use but improves
    /// proof generation speed, making it suitable for performance-sensitive
    /// environments.
    ///
    /// # Arguments
    /// * `trusted_setup` — The shared structured reference string (SRS)
    ///   used to configure both prover and verifier contexts.
    /// * `use_precomp` — Whether to enable prover-side precomputations
    ///   for faster proof creation at the cost of extra memory. The cost in
    ///   memory is exponential in the `width`.
    pub fn new(trusted_setup: &TrustedSetup, mode: Mode) -> Self {
        let (prover_ctx, eip4844_mode) = match mode {
            Mode::Both(use_precomp) => (
                Some(ProverContext::new(trusted_setup, use_precomp)),
                eip4844::Mode::Both,
            ),
            Mode::VerifierOnly => (None, eip4844::Mode::VerifierOnly),
        };

        Self {
            prover_ctx,
            verifier_ctx: VerifierContext::new(trusted_setup),
            eip4844_ctx: eip4844::Context::new(trusted_setup, eip4844_mode),
        }
    }
}
