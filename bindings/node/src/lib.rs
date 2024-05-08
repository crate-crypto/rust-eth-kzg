use napi::{
  bindgen_prelude::{BigInt, Uint8Array},
  *,
};
use napi_derive::napi;

use eip7594::{prover::ProverContext, verifier::VerifierContext};

#[napi]
pub struct ProverContextJs {
  inner: ProverContext,
}

#[napi]
pub struct CellsAndProofs {
  pub cells: Vec<Uint8Array>,
  pub proofs: Vec<Uint8Array>,
}

#[napi]
impl ProverContextJs {
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

  #[napi]
  pub fn compute_cells_and_kzg_proofs(&self, blob: Uint8Array) -> Result<CellsAndProofs> {
    let blob = blob.as_ref();
    let (cells, proofs) = self.inner.compute_cells_and_kzg_proofs(blob);

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
    let cells = self.inner.compute_cells(blob);

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
