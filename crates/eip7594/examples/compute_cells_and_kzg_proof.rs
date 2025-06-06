//! Example: KZG cell proof generation benchmark for a single blob
//!
//! This example demonstrates how to:
//! - Construct a blob containing a degree-4095 polynomial over BLS12-381,
//! - Compute all of its cells and associated KZG proofs,
//! - Benchmark this operation in a warm-up loop.
//!
//! The functionality shown here is part of the [EIP-7594 (PeerDAS)](https://eips.ethereum.org/EIPS/eip-7594)
//! protocol, which introduces data availability sampling (DAS) using per-cell
//! KZG proofs over erasure-coded polynomial blobs.
//!
//! In PeerDAS, each blob consists of data subdivided into cells, where each cell
//! can be individually verified against a KZG commitment. This allows nodes to verify
//! partial data availability without needing the entire blob.
//!
//! This example initializes a trusted setup, builds a test blob with 4096 coefficients,
//! and repeatedly computes the cell commitments and their corresponding KZG proofs
//! using a `DASContext`.
//!
//! The final run is traced with structured logging using `tracing_forest`.

use std::time::Instant;

use bls12_381::Scalar;
use rust_eth_kzg::{constants::BYTES_PER_BLOB, DASContext, Mode, TrustedSetup};
use tracing_forest::{util::LevelFilter, ForestLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Number of polynomial coefficients in the blob (degree + 1)
const POLYNOMIAL_LEN: usize = 4096;

/// Constructs a test blob with `POLYNOMIAL_LEN` coefficients encoded as big-endian bytes.
fn dummy_blob() -> [u8; BYTES_PER_BLOB] {
    // Create an iterator over negated field elements: [-0, -1, -2, ..., -4095]
    let polynomial = (0..POLYNOMIAL_LEN).map(|i| -Scalar::from(i as u64));

    // Flatten each scalar into 32 big-endian bytes and collect into a byte buffer
    let blob: Vec<_> = polynomial
        .into_iter()
        .flat_map(|scalar| scalar.to_bytes_be())
        .collect();

    // Convert the result into a fixed-size array [u8; BYTES_PER_BLOB]
    blob.try_into().expect("blob conversion failed")
}

fn main() {
    // Load the default trusted setup parameters
    let trusted_setup = TrustedSetup::default();

    // Construct the test input blob containing a polynomial of 4096 coefficients
    let blob = dummy_blob();

    // Initialize the data availability sampling context with precomputed fixed-base MSM
    let ctx = DASContext::new(
        &trusted_setup,
        Mode::Both(bls12_381::fixed_base_msm::UsePrecomp::Yes { width: 8 }),
    );

    // Warm-up phase
    println!("Warming up for 3 seconds...");

    // Repeatedly compute cells and KZG proofs for the blob for 3 seconds
    let start = Instant::now();
    while Instant::now().duration_since(start).as_secs() < 3 {
        ctx.compute_cells_and_kzg_proofs(&blob)
            .expect("failed to compute kzg proof");
    }

    // Set up structured tracing/logging with INFO level from the environment
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    Registry::default()
        .with(env_filter)
        .with(ForestLayer::default())
        .init();

    // Perform one final proof generation with tracing enabled
    ctx.compute_cells_and_kzg_proofs(&blob)
        .expect("failed to compute kzg proof");
}
