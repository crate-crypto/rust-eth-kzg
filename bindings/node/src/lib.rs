use std::sync::{Arc, Mutex};

use napi::{
  bindgen_prelude::{AsyncTask, BigInt, Env, Error, Uint8Array},
  Result, Task, *,
};
use napi_derive::napi;

use eip7594::{prover::ProverContext, verifier::VerifierContext};

#[napi]
pub struct ProverContextJs {
  inner: Arc<Mutex<ProverContext>>,
}

#[napi]
pub struct CellsAndProofs {
  pub cells: Vec<Uint8Array>,
  pub proofs: Vec<Uint8Array>,
}

pub struct AsyncBlobToKzgCommitment {
  blob: Uint8Array,
  prover_context: Arc<Mutex<ProverContext>>,
}

#[napi]
impl Task for AsyncBlobToKzgCommitment {
  type Output = Uint8Array;
  type JsValue = Uint8Array;

  fn compute(&mut self) -> Result<Uint8Array> {
    let blob = self.blob.as_ref();
    let prover_context = self
      .prover_context
      .lock()
      .map_err(|_| napi::Error::from_reason("Failed to acquire lock"))?;
    let commitment = prover_context.blob_to_kzg_commitment(blob);
    Ok(Uint8Array::from(&commitment))
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
impl ProverContextJs {
  #[napi(constructor)]
  pub fn new() -> Self {
    ProverContextJs {
      inner: Arc::new(Mutex::new(ProverContext::new())),
    }
  }

  #[napi]
  pub fn blob_to_kzg_commitment(&self, blob: Uint8Array) -> Result<Uint8Array> {
    let blob = blob.as_ref();
    let prover_context = self
      .inner
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;
    let commitment = prover_context.blob_to_kzg_commitment(blob);
    Ok(Uint8Array::from(&commitment))
  }

  #[napi]
  pub fn async_blob_to_kzg_commitment(
    &self,
    blob: Uint8Array,
  ) -> AsyncTask<AsyncBlobToKzgCommitment> {
    AsyncTask::new(AsyncBlobToKzgCommitment {
      blob,
      prover_context: Arc::clone(&self.inner),
    })
  }

  #[napi]
  pub fn compute_cells_and_kzg_proofs(&self, blob: Uint8Array) -> Result<CellsAndProofs> {
    let blob = blob.as_ref();
    let prover_context = self
      .inner
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;
    let (cells, proofs) = prover_context.compute_cells_and_kzg_proofs(blob);

    let cells_uint8array = cells
      .into_iter()
      .map(|cell| Uint8Array::from(cell))
      .collect::<Vec<Uint8Array>>();
    let proofs_uint8array = proofs
      .into_iter()
      .map(|proof| Uint8Array::from(proof))
      .collect::<Vec<Uint8Array>>();

    Ok(CellsAndProofs {
      cells: cells_uint8array,
      proofs: proofs_uint8array,
    })
  }

  #[napi]
  pub fn compute_cells(&self, blob: Uint8Array) -> Result<Vec<Uint8Array>> {
    let blob = blob.as_ref();
    let prover_context = self
      .inner
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;
    let cells = prover_context.compute_cells(blob);

    let cells_uint8array = cells
      .into_iter()
      .map(|cell| Uint8Array::from(cell))
      .collect::<Vec<Uint8Array>>();

    Ok(cells_uint8array)
  }
}

#[napi]
pub struct VerifierContextJs {
  inner: VerifierContext,
}

#[napi]
impl VerifierContextJs {
  #[napi(constructor)]
  pub fn new() -> Self {
    VerifierContextJs {
      inner: VerifierContext::new(),
    }
  }

  #[napi]
  pub fn verify_cell_kzg_proof(
    &self,
    commitment: Uint8Array,
    // Note: U64 cannot be used as an argument, see : https://napi.rs/docs/concepts/values.en#bigint
    cell_id: BigInt,
    cell: Uint8Array,
    proof: Uint8Array,
  ) -> Result<bool> {
    let commitment = commitment.as_ref();
    let cell = cell.as_ref();
    let proof = proof.as_ref();

    let (signed, cell_id_value, _) = cell_id.get_u128();
    assert!(signed == false, "cell id should be an unsigned integer");

    let cell_id_u64 = cell_id_value as u64;
    Ok(
      self
        .inner
        .verify_cell_kzg_proof(commitment, cell_id_u64, cell, proof),
    )
  }
}
