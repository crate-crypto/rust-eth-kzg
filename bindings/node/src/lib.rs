use std::sync::Arc;

use napi::{
  bindgen_prelude::{BigInt, Error, Uint8Array},
  Either, Result,
};
use napi_derive::napi;

use rust_eth_kzg::{
  constants::{self, RECOMMENDED_PRECOMP_WIDTH},
  DASContext, TrustedSetup, UsePrecomp,
};

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
pub struct DASContextJs {
  inner: Arc<DASContext>,
}

impl Default for DASContextJs {
  fn default() -> Self {
    Self::new()
  }
}

#[napi(object)]
pub struct DASContextOptions {
  pub use_precomp: bool,
}

impl Default for DASContextOptions {
  fn default() -> Self {
    Self { use_precomp: true }
  }
}

#[napi]
impl DASContextJs {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self::create(DASContextOptions::default())
  }

  #[napi(factory)]
  pub fn create(options: DASContextOptions) -> Self {
    let use_precomp = options.use_precomp;

    let precomp = if use_precomp {
      UsePrecomp::Yes {
        width: RECOMMENDED_PRECOMP_WIDTH,
      }
    } else {
      UsePrecomp::No
    };

    DASContextJs {
      inner: Arc::new(DASContext::new(&TrustedSetup::default(), precomp)),
    }
  }

  #[napi]
  pub fn blob_to_kzg_commitment(&self, blob: Uint8Array) -> Result<Uint8Array> {
    let blob = blob.as_ref();
    let ctx = &self.inner;
    let blob = slice_to_array_ref(blob, "blob")?;

    let commitment = ctx.blob_to_kzg_commitment(blob).map_err(|err| {
      Error::from_reason(format!("failed to compute blob_to_kzg_commitment: {err:?}",))
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
        "failed to compute compute_cells_and_kzg_proofs: {err:?}"
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
    let blob = blob.as_ref();
    let ctx = &self.inner;

    let blob = slice_to_array_ref(blob, "blob")?;

    let cells = ctx
      .compute_cells(blob)
      .map_err(|err| Error::from_reason(format!("failed to compute compute_cells: {err:?}")))?;

    let cells_uint8array = cells
      .into_iter()
      .map(|cell| Uint8Array::from(cell.to_vec()))
      .collect::<Vec<Uint8Array>>();

    Ok(cells_uint8array)
  }

  #[napi]
  pub async fn async_compute_cells(&self, blob: Uint8Array) -> Result<Vec<Uint8Array>> {
    self.compute_cells(blob)
  }

  #[allow(deprecated)]
  #[napi]
  pub fn recover_cells_and_kzg_proofs(
    &self,
    cell_indices: Vec<Either<u32, BigInt>>,
    cells: Vec<Uint8Array>,
  ) -> Result<CellsAndProofs> {
    let cell_indices: Vec<_> = cell_indices.into_iter().map(u32_or_bigint_to_u64).collect();
    let cells: Vec<_> = cells.iter().map(|cell| cell.as_ref()).collect();

    let ctx = &self.inner;

    let cells: Vec<_> = cells
      .iter()
      .map(|cell| slice_to_array_ref(cell, "cell"))
      .collect::<Result<_, _>>()?;

    let (cells, proofs) = ctx
      .recover_cells_and_kzg_proofs(cell_indices, cells)
      .map_err(|err| {
        Error::from_reason(format!(
          "failed to compute recover_cells_and_kzg_proofs: {err:?}"
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
    cell_indices: Vec<Either<u32, BigInt>>,
    cells: Vec<Uint8Array>,
  ) -> Result<CellsAndProofs> {
    self.recover_cells_and_kzg_proofs(cell_indices, cells)
  }

  #[napi]
  pub fn verify_cell_kzg_proof_batch(
    &self,
    commitments: Vec<Uint8Array>,
    cell_indices: Vec<Either<u32, BigInt>>,
    cells: Vec<Uint8Array>,
    proofs: Vec<Uint8Array>,
  ) -> Result<bool> {
    let cell_indices: Vec<_> = cell_indices.into_iter().map(u32_or_bigint_to_u64).collect();

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

    let valid = ctx.verify_cell_kzg_proof_batch(commitments, &cell_indices, cells, proofs);
    match valid {
      Ok(_) => Ok(true),
      Err(x) if x.is_proof_invalid() => Ok(false),
      Err(err) => Err(Error::from_reason(format!(
        "failed to compute verify_cell_kzg_proof_batch: {err:?}"
      ))),
    }
  }

  #[napi]
  pub async fn async_verify_cell_kzg_proof_batch(
    &self,
    commitments: Vec<Uint8Array>,
    cell_indices: Vec<Either<u32, BigInt>>,
    cells: Vec<Uint8Array>,
    proofs: Vec<Uint8Array>,
  ) -> Result<bool> {
    self.verify_cell_kzg_proof_batch(commitments, cell_indices, cells, proofs)
  }

  #[napi]
  pub fn compute_kzg_proof(&self, blob: Uint8Array, z: Uint8Array) -> Result<Vec<Uint8Array>> {
    let blob = blob.as_ref();
    let z = z.as_ref();
    let ctx = &self.inner;

    let blob = slice_to_array_ref(blob, "blob")?;
    let z = slice_to_array_ref(z, "z")?;

    let (proof, y) = ctx
      .compute_kzg_proof(blob, *z)
      .map_err(|err| Error::from_reason(format!("failed to compute compute_kzg_proof: {err:?}")))?;

    Ok(vec![Uint8Array::from(&proof), Uint8Array::from(&y)])
  }

  #[napi]
  pub async fn async_compute_kzg_proof(
    &self,
    blob: Uint8Array,
    z: Uint8Array,
  ) -> Result<Vec<Uint8Array>> {
    self.compute_kzg_proof(blob, z)
  }

  #[napi]
  pub fn compute_blob_kzg_proof(
    &self,
    blob: Uint8Array,
    commitment: Uint8Array,
  ) -> Result<Uint8Array> {
    let blob = blob.as_ref();
    let commitment = commitment.as_ref();
    let ctx = &self.inner;

    let blob = slice_to_array_ref(blob, "blob")?;
    let commitment = slice_to_array_ref(commitment, "commitment")?;

    let proof = ctx
      .compute_blob_kzg_proof(blob, commitment)
      .map_err(|err| {
        Error::from_reason(format!("failed to compute compute_blob_kzg_proof: {err:?}"))
      })?;

    Ok(Uint8Array::from(&proof))
  }

  #[napi]
  pub async fn async_compute_blob_kzg_proof(
    &self,
    blob: Uint8Array,
    commitment: Uint8Array,
  ) -> Result<Uint8Array> {
    self.compute_blob_kzg_proof(blob, commitment)
  }

  #[napi]
  pub fn verify_kzg_proof(
    &self,
    commitment: Uint8Array,
    z: Uint8Array,
    y: Uint8Array,
    proof: Uint8Array,
  ) -> Result<bool> {
    let commitment = commitment.as_ref();
    let z = z.as_ref();
    let y = y.as_ref();
    let proof = proof.as_ref();
    let ctx = &self.inner;

    let commitment = slice_to_array_ref(commitment, "commitment")?;
    let z = slice_to_array_ref(z, "z")?;
    let y = slice_to_array_ref(y, "y")?;
    let proof = slice_to_array_ref(proof, "proof")?;

    let valid = ctx.verify_kzg_proof(commitment, *z, *y, proof);
    match valid {
      Ok(_) => Ok(true),
      Err(x) if x.is_proof_invalid() => Ok(false),
      Err(err) => Err(Error::from_reason(format!(
        "failed to compute verify_kzg_proof: {err:?}"
      ))),
    }
  }

  #[napi]
  pub async fn async_verify_kzg_proof(
    &self,
    commitment: Uint8Array,
    z: Uint8Array,
    y: Uint8Array,
    proof: Uint8Array,
  ) -> Result<bool> {
    self.verify_kzg_proof(commitment, z, y, proof)
  }

  #[napi]
  pub fn verify_blob_kzg_proof(
    &self,
    blob: Uint8Array,
    commitment: Uint8Array,
    proof: Uint8Array,
  ) -> Result<bool> {
    let blob = blob.as_ref();
    let commitment = commitment.as_ref();
    let proof = proof.as_ref();
    let ctx = &self.inner;

    let blob = slice_to_array_ref(blob, "blob")?;
    let commitment = slice_to_array_ref(commitment, "commitment")?;
    let proof = slice_to_array_ref(proof, "proof")?;

    let valid = ctx.verify_blob_kzg_proof(blob, commitment, proof);
    match valid {
      Ok(_) => Ok(true),
      Err(x) if x.is_proof_invalid() => Ok(false),
      Err(err) => Err(Error::from_reason(format!(
        "failed to compute verify_blob_kzg_proof: {err:?}"
      ))),
    }
  }

  #[napi]
  pub async fn async_verify_blob_kzg_proof(
    &self,
    blob: Uint8Array,
    commitment: Uint8Array,
    proof: Uint8Array,
  ) -> Result<bool> {
    self.verify_blob_kzg_proof(blob, commitment, proof)
  }

  #[napi]
  pub fn verify_blob_kzg_proof_batch(
    &self,
    blobs: Vec<Uint8Array>,
    commitments: Vec<Uint8Array>,
    proofs: Vec<Uint8Array>,
  ) -> Result<bool> {
    let blobs: Vec<_> = blobs
      .iter()
      .map(|blob| slice_to_array_ref(blob, "blob"))
      .collect::<Result<_, _>>()?;
    let commitments: Vec<_> = commitments
      .iter()
      .map(|commitment| slice_to_array_ref(commitment, "commitment"))
      .collect::<Result<_, _>>()?;
    let proofs: Vec<_> = proofs
      .iter()
      .map(|proof| slice_to_array_ref(proof, "proof"))
      .collect::<Result<_, _>>()?;

    let ctx = &self.inner;

    let valid = ctx.verify_blob_kzg_proof_batch(blobs, commitments, proofs);
    match valid {
      Ok(_) => Ok(true),
      Err(x) if x.is_proof_invalid() => Ok(false),
      Err(err) => Err(Error::from_reason(format!(
        "failed to compute verify_blob_kzg_proof_batch: {err:?}"
      ))),
    }
  }

  #[napi]
  pub async fn async_verify_blob_kzg_proof_batch(
    &self,
    blobs: Vec<Uint8Array>,
    commitments: Vec<Uint8Array>,
    proofs: Vec<Uint8Array>,
  ) -> Result<bool> {
    self.verify_blob_kzg_proof_batch(blobs, commitments, proofs)
  }
}

// We use bigint because u64 cannot be used as an argument, see : https://napi.rs/docs/concepts/values.en#bigint
fn u32_or_bigint_to_u64(value: Either<u32, BigInt>) -> u64 {
  match value {
    Either::A(v) => v as u64,
    Either::B(v) => {
      let (signed, value_u128, _) = v.get_u128();
      assert!(!signed, "value should be an unsigned integer");
      value_u128 as u64
    }
  }
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
