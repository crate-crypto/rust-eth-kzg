use std::time::Instant;

use bls12_381::{traits::*, Scalar};
use eip4844::{
    constants::{BYTES_PER_BLOB, FIELD_ELEMENTS_PER_BLOB},
    Context, TrustedSetup,
};
use tracing_forest::{util::LevelFilter, ForestLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

fn dummy_blob() -> [u8; BYTES_PER_BLOB] {
    let polynomial = (0..FIELD_ELEMENTS_PER_BLOB).map(|i| -Scalar::from(i as u64));
    let blob: Vec<_> = polynomial
        .into_iter()
        .flat_map(|scalar| scalar.to_bytes_be())
        .collect();
    blob.try_into().expect("blob conversion failed")
}

fn main() {
    let trusted_setup = TrustedSetup::default();
    let blob = dummy_blob();
    let z = Scalar::random(rand::thread_rng()).to_bytes_be();

    let ctx = Context::new(&trusted_setup, eip4844::Mode::Both);

    println!("Warming up for 3 seconds...");

    let start = Instant::now();
    while Instant::now().duration_since(start).as_secs() < 3 {
        ctx.compute_kzg_proof(&blob, z)
            .expect("failed to compute kzg proof");
    }

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    Registry::default()
        .with(env_filter)
        .with(ForestLayer::default())
        .init();

    ctx.compute_kzg_proof(&blob, z)
        .expect("failed to compute kzg proof (z out of domain)");
    ctx.compute_kzg_proof(&blob, (-Scalar::ONE).to_bytes_be())
        .expect("failed to compute kzg proof (z within domain)");
}
