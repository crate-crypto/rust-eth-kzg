use std::sync::{Arc, Mutex};

use napi::{
  bindgen_prelude::{AsyncTask, BigInt, Env, Error, Uint8Array},
  Result, Task,
};
use napi_derive::napi;

use eip7594::{constants, prover::ProverContext, verifier::VerifierContext, KZGCommitment};

#[napi]
pub const BYTES_PER_CELL: u32 = constants::BYTES_PER_CELL as u32;
#[napi]
pub const BYTES_PER_COMMITMENT: u32 = constants::BYTES_PER_COMMITMENT as u32;
#[napi]
pub const BYTES_PER_FIELD_ELEMENT: u32 = constants::BYTES_PER_FIELD_ELEMENT as u32;
#[napi]
pub const FIELD_ELEMENTS_PER_BLOB: u32 = constants::FIELD_ELEMENTS_PER_BLOB as u32;
#[napi]
pub const FIELD_ELEMENTS_PER_CELL: u32 = constants::FIELD_ELEMENTS_PER_CELL as u32;

pub struct AsyncBlobToKzgCommitment {
  blob: Uint8Array,

  // TODO: make this RwLock parking_lot crate
  prover_context: Arc<Mutex<ProverContext>>,
}

#[napi]
impl Task for AsyncBlobToKzgCommitment {
  type Output = KZGCommitment;
  type JsValue = Uint8Array;

  fn compute(&mut self) -> Result<Self::Output> {
    let blob = self.blob.as_ref();
    let prover_context = self
      .prover_context
      .lock()
      .map_err(|_| napi::Error::from_reason("Failed to acquire lock"))?;
    let commitment = prover_context.blob_to_kzg_commitment(blob);
    Ok(commitment)
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(Uint8Array::from(&output))
  }
}

pub struct NativeCellsAndProofs {
  pub cells: [Vec<u8>; 128],
  pub proofs: [[u8; 48]; 128],
}

#[napi]
pub struct CellsAndProofs {
  pub cells: Vec<Uint8Array>,
  pub proofs: Vec<Uint8Array>,
}

pub struct AsyncComputeCellsAndKzgProofs {
  blob: Uint8Array,
  prover_context: Arc<Mutex<ProverContext>>,
}

#[napi]
impl Task for AsyncComputeCellsAndKzgProofs {
  type Output = NativeCellsAndProofs;
  type JsValue = CellsAndProofs;

  fn compute(&mut self) -> Result<Self::Output> {
    let blob = self.blob.as_ref();
    let prover_context = self
      .prover_context
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;
    let (cells, proofs) = prover_context.compute_cells_and_kzg_proofs(blob);

    Ok(NativeCellsAndProofs { cells, proofs })
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    let cells = output
      .cells
      .into_iter()
      .map(|cell| Uint8Array::from(cell))
      .collect::<Vec<Uint8Array>>();
    let proofs = output
      .proofs
      .into_iter()
      .map(|proof| Uint8Array::from(proof))
      .collect::<Vec<Uint8Array>>();
    Ok(CellsAndProofs { cells, proofs })
  }
}

pub struct AsyncComputeCells {
  blob: Uint8Array,
  prover_context: Arc<Mutex<ProverContext>>,
}

#[napi]
impl Task for AsyncComputeCells {
  type Output = [Vec<u8>; 128];
  type JsValue = Vec<Uint8Array>;

  fn compute(&mut self) -> Result<Self::Output> {
    let blob = self.blob.as_ref();
    let prover_context = self
      .prover_context
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;
    let cells = prover_context.compute_cells(blob);
    Ok(cells)
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    let cells = output
      .into_iter()
      .map(|cell| Uint8Array::from(cell))
      .collect::<Vec<Uint8Array>>();
    Ok(cells)
  }
}

#[napi]
pub struct ProverContextJs {
  inner: Arc<Mutex<ProverContext>>,
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
  pub fn async_compute_cells_and_kzg_proofs(
    &self,
    blob: Uint8Array,
  ) -> AsyncTask<AsyncComputeCellsAndKzgProofs> {
    AsyncTask::new(AsyncComputeCellsAndKzgProofs {
      blob,
      prover_context: Arc::clone(&self.inner),
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

  #[napi]
  pub fn async_compute_cells(&self, blob: Uint8Array) -> AsyncTask<AsyncComputeCells> {
    AsyncTask::new(AsyncComputeCells {
      blob,
      prover_context: Arc::clone(&self.inner),
    })
  }
}

pub struct AsyncVerifyCellKzgProof {
  commitment: Vec<u8>,
  cell_id: u64,
  cell: Vec<u8>,
  proof: Vec<u8>,
  verifier_context: Arc<Mutex<VerifierContext>>,
}

#[napi]
impl Task for AsyncVerifyCellKzgProof {
  type Output = bool;
  type JsValue = bool;

  fn compute(&mut self) -> Result<Self::Output> {
    let commitment = self.commitment.as_ref();
    let cell = self.cell.as_ref();
    let proof = self.proof.as_ref();
    let verifier_context = self
      .verifier_context
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;
    Ok(verifier_context.verify_cell_kzg_proof(commitment, self.cell_id, cell, proof))
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub struct VerifierContextJs {
  inner: Arc<Mutex<VerifierContext>>,
}

#[napi]
impl VerifierContextJs {
  #[napi(constructor)]
  pub fn new() -> Self {
    VerifierContextJs {
      inner: Arc::new(Mutex::new(VerifierContext::new())),
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
    let verifier_context = self
      .inner
      .lock()
      .map_err(|_| Error::from_reason("Failed to acquire lock"))?;

    let cell_id_u64 = cell_id_value as u64;
    Ok(verifier_context.verify_cell_kzg_proof(commitment, cell_id_u64, cell, proof))
  }

  #[napi]
  pub fn async_verify_cell_kzg_proof(
    &self,
    commitment: Uint8Array,
    // Note: U64 cannot be used as an argument, see : https://napi.rs/docs/concepts/values.en#bigint
    cell_id: BigInt,
    cell: Uint8Array,
    proof: Uint8Array,
  ) -> AsyncTask<AsyncVerifyCellKzgProof> {
    let commitment = commitment.as_ref();
    let cell = cell.as_ref();
    let proof = proof.as_ref();

    let (signed, cell_id_value, _) = cell_id.get_u128();
    assert!(signed == false, "cell id should be an unsigned integer");

    let cell_id_u64 = cell_id_value as u64;

    AsyncTask::new(AsyncVerifyCellKzgProof {
      commitment: commitment.to_vec(),
      cell: cell.to_vec(),
      cell_id: cell_id_u64,
      proof: proof.to_vec(),
      verifier_context: Arc::clone(&self.inner),
    })
  }
}
