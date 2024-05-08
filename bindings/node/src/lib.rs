
use napi_derive::napi;
use napi::{bindgen_prelude::Uint8Array, *};

use eip7594::{prover::ProverContext, verifier::VerifierContext};

#[napi]
pub struct ProverContextJs {
  inner: ProverContext,
}

#[napi]
impl ProverContextJs{
  #[napi(constructor)]
  pub fn new() -> Self {
    ProverContextJs {
      inner: ProverContext::new(),
    }
  }

  #[napi]
  pub fn blob_to_kzg_commitment(&self, blob: Uint8Array) -> Result<Uint8Array> {
    let blob = blob.as_ref();
    let commitment = self.inner.blob_to_kzg_commitment(blob);
    Ok(Uint8Array::from(&commitment))
  }

}

#[napi]
pub struct VerifierContextJs {
  inner: VerifierContext,
}
