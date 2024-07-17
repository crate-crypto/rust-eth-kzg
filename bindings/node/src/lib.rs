use std::sync::Arc;

use napi::{
  bindgen_prelude::{BigInt, Error, Uint8Array},
  Result,
};
use napi_derive::napi;

use rust_eth_kzg::{constants, DASContext};

#[napi]
pub const BYTES_PER_COMMITMENT: u32 = constants::BYTES_PER_COMMITMENT as u32;
#[napi]
pub const BYTES_PER_PROOF: u32 = constants::BYTES_PER_COMMITMENT as u32;
#[napi]
pub const BYTES_PER_FIELD_ELEMENT: u32 = constants::BYTES_PER_FIELD_ELEMENT as u32;
#[napi]
pub const BYTES_PER_BLOB: u32 = constants::BYTES_PER_BLOB as u32;
#[napi]
pub const MAX_NUM_COLUMNS: u32 = constants::CELLS_PER_EXT_BLOB as u32;
#[napi]
pub const BYTES_PER_CELL: u32 = constants::BYTES_PER_CELL as u32;

#[napi]
pub struct CellsAndProofs {
  pub cells: Vec<Uint8Array>,
  pub proofs: Vec<Uint8Array>,
}

#[napi]
pub struct PeerDASContextJs {
  inner: Arc<DASContext>,
}

impl Default for PeerDASContextJs {
  fn default() -> Self {
    Self::new()
  }
}

#[napi]
impl PeerDASContextJs {
  #[napi(constructor)]
  pub fn new() -> Self {
    PeerDASContextJs {
      inner: Arc::new(DASContext::default()),
    }
  }

  #[napi]
  pub fn blob_to_kzg_commitment(&self, blob: Uint8Array) -> Result<Uint8Array> {
    let blob = blob.as_ref();
    let ctx = &self.inner;
    let blob = slice_to_array_ref(blob, "blob")?;

    let commitment = ctx.blob_to_kzg_commitment(blob).map_err(|err| {
      Error::from_reason(format!(
        "failed to compute blob_to_kzg_commitment: {:?}",
        err
      ))
    })?;
    Ok(Uint8Array::from(&commitment))
  }

  #[napi]
  pub async fn async_blob_to_kzg_commitment(&self, blob: Uint8Array) -> Result<Uint8Array> {
    self.blob_to_kzg_commitment(blob)
  }

  #[napi]
  pub fn compute_cells_and_kzg_proofs(&self, blob: Uint8Array) -> Result<CellsAndProofs> {
    let blob = blob.as_ref();
    let ctx = &self.inner;

    let blob = slice_to_array_ref(blob, "blob")?;

    let (cells, proofs) = ctx.compute_cells_and_kzg_proofs(blob).map_err(|err| {
      Error::from_reason(format!(
        "failed to compute compute_cells_and_kzg_proofs: {:?}",
        err
      ))
    })?;

    let cells_uint8array = cells
      .into_iter()
      .map(|cell| Uint8Array::from(cell.to_vec()))
      .collect::<Vec<Uint8Array>>();
    let proofs_uint8array = proofs
      .into_iter()
      .map(Uint8Array::from)
      .collect::<Vec<Uint8Array>>();

    Ok(CellsAndProofs {
      cells: cells_uint8array,
      proofs: proofs_uint8array,
    })
  }

  #[napi]
  pub async fn async_compute_cells_and_kzg_proofs(
    &self,
    blob: Uint8Array,
  ) -> Result<CellsAndProofs> {
    self.compute_cells_and_kzg_proofs(blob)
  }

  #[napi]
  pub fn compute_cells(&self, blob: Uint8Array) -> Result<Vec<Uint8Array>> {
    self
      .compute_cells_and_kzg_proofs(blob)
      .map(|cells_and_proofs| cells_and_proofs.cells)
  }

  #[napi]
  pub async fn async_compute_cells(&self, blob: Uint8Array) -> Result<Vec<Uint8Array>> {
    self.compute_cells(blob)
  }

  #[allow(deprecated)]
  #[napi]
  pub fn recover_cells_and_kzg_proofs(
    &self,
    cell_indices: Vec<BigInt>,
    cells: Vec<Uint8Array>,
  ) -> Result<CellsAndProofs> {
    let cell_indices: Vec<_> = cell_indices.into_iter().map(bigint_to_u64).collect();
    let cells: Vec<_> = cells.iter().map(|cell| cell.as_ref()).collect();

    let ctx = &self.inner;

    let cells: Vec<_> = cells
      .iter()
      .map(|cell| slice_to_array_ref(cell, "cell"))
      .collect::<Result<_, _>>()?;

    let (cells, proofs) = ctx
      .recover_cells_and_proofs(cell_indices, cells)
      .map_err(|err| {
        Error::from_reason(format!(
          "failed to compute recover_cells_and_kzg_proofs: {:?}",
          err
        ))
      })?;

    let cells_uint8array = cells
      .into_iter()
      .map(|cell| Uint8Array::from(cell.to_vec()))
      .collect::<Vec<Uint8Array>>();
    let proofs_uint8array = proofs
      .into_iter()
      .map(Uint8Array::from)
      .collect::<Vec<Uint8Array>>();

    Ok(CellsAndProofs {
      cells: cells_uint8array,
      proofs: proofs_uint8array,
    })
  }

  #[napi]
  pub async fn async_recover_cells_and_kzg_proofs(
    &self,
    cell_indices: Vec<BigInt>,
    cells: Vec<Uint8Array>,
  ) -> Result<CellsAndProofs> {
    self.recover_cells_and_kzg_proofs(cell_indices, cells)
  }

  #[napi]
  pub fn verify_cell_kzg_proof_batch(
    &self,
    commitments: Vec<Uint8Array>,
    row_indices: Vec<BigInt>,
    column_indices: Vec<BigInt>,
    cells: Vec<Uint8Array>,
    proofs: Vec<Uint8Array>,
  ) -> Result<bool> {
    let row_indices: Vec<_> = row_indices.into_iter().map(bigint_to_u64).collect();
    let column_indices: Vec<_> = column_indices.into_iter().map(bigint_to_u64).collect();

    let commitments: Vec<_> = commitments
      .iter()
      .map(|commitment| slice_to_array_ref(commitment, "commitment"))
      .collect::<Result<_, _>>()?;
    let cells: Vec<_> = cells
      .iter()
      .map(|cell| slice_to_array_ref(cell, "cell"))
      .collect::<Result<_, _>>()?;
    let proofs: Vec<_> = proofs
      .iter()
      .map(|proof| slice_to_array_ref(proof, "proof"))
      .collect::<Result<_, _>>()?;

    let ctx = &self.inner;

    let valid =
      ctx.verify_cell_kzg_proof_batch(commitments, row_indices, column_indices, cells, proofs);
    match valid {
      Ok(_) => Ok(true),
      Err(x) if x.invalid_proof() => Ok(false),
      Err(err) => Err(Error::from_reason(format!(
        "failed to compute verify_cell_kzg_proof_batch: {:?}",
        err
      ))),
    }
  }

  #[napi]
  pub async fn async_verify_cell_kzg_proof_batch(
    &self,
    commitments: Vec<Uint8Array>,
    row_indices: Vec<BigInt>,
    column_indices: Vec<BigInt>,
    cells: Vec<Uint8Array>,
    proofs: Vec<Uint8Array>,
  ) -> Result<bool> {
    self.verify_cell_kzg_proof_batch(commitments, row_indices, column_indices, cells, proofs)
  }
}

// We use bigint because u64 cannot be used as an argument, see : https://napi.rs/docs/concepts/values.en#bigint
fn bigint_to_u64(value: BigInt) -> u64 {
  let (signed, value_u128, _) = value.get_u128();
  assert!(!signed, "value should be an unsigned integer");
  value_u128 as u64
}

/// Convert a slice into a reference to an array
///
/// This is needed as the API for rust library does
/// not accept slices.
fn slice_to_array_ref<'a, const N: usize>(
  slice: &'a [u8],
  name: &'static str,
) -> Result<&'a [u8; N]> {
  slice.try_into().map_err(|err| {
    Error::from_reason(format!(
      "{name} must have size {N}, found size {}\n err:{}",
      slice.len(),
      err
    ))
  })
}
