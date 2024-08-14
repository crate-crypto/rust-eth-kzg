import bindings

import results
export results


# Note: there are no length checks in the nim code before calling the rust library's c api
# because the types are are sized at compile time.

const
  BYTES_PER_FIELD_ELEMENT* = 32
  CELLS_PER_EXT_BLOB = 128
  MAX_NUM_COLUMNS* = CELLS_PER_EXT_BLOB
  BYTES_PER_BLOB* = 131_072
  BYTES_PER_CELL* = 2048

type
  Bytes48* = object
    bytes*: array[48, byte]

  Blob* = object
    bytes*: array[BYTES_PER_BLOB, byte]

  Cell* = object
    bytes*: array[BYTES_PER_CELL, byte]

  KZGCommitment* = Bytes48

  KZGProof* = Bytes48

  Cells* = array[CELLS_PER_EXT_BLOB, Cell]

  CellsAndProofs* = object
    cells*: Cells
    proofs*: array[CELLS_PER_EXT_BLOB, KZGProof]


template getPtr(x: untyped): auto =
  when (NimMajor, NimMinor) <= (1,6):
    unsafeAddr(x)
  else:
    addr(x)

# Function to safely get a pointer to the first element of a sequence or openArray
template safeGetPtr[T](arr: openArray[T]): pointer =
  if arr.len > 0:
    arr[0].getPtr
  else:
    # Return a null pointer if the array is empty
    nil

# Convert an openArray of untyped to a pointer to a pointer
# ie convert a 2d array to a double pointer
template toPtrPtr(cells: openArray[untyped]): ptr pointer =
  # Create a seq of pointers to pointers
  var ptrSeq: seq[ptr pointer]
  ptrSeq.setLen(cells.len)

  # For each item in the openArray, get its pointer and assign it to the seq
  for i in 0..<cells.len:
    ptrSeq[i] = cast[ptr pointer](cells[i].bytes.getPtr)

  # Return the pointer to the seq of pointers
  cast[ptr pointer](ptrSeq.safeGetPtr)

template verify_result(res: CResult, ret: untyped): untyped =
  if res.xstatus != CResultStatus.Ok:
    return err($res)
  ok(ret)


type
  KZGCtx* = ref object
    ctx_ptr: ptr DASContext

# Define custom destructor
# Nim2 does not allow us to take in a var T
# for the custom destructor so it must ensure that
# this is not called twice.
# https://forum.nim-lang.org/t/11229
proc `=destroy`(x: typeof KZGCtx()[]) =
  if x.ctx_ptr != nil:
    eth_kzg_das_context_free(x.ctx_ptr)

proc newKZGCtx*(): KZGCtx =
  var kzgCtx = KZGCtx()
  kzgCtx.ctx_ptr = eth_kzg_das_context_new()
  return kzgCtx


proc blobToKZGCommitment*(ctx: KZGCtx, blob : Blob): Result[KZGCommitment, string] {.gcsafe.} =
  var ret: KZGCommitment

  let res = eth_kzg_blob_to_kzg_commitment(
    ctx.ctx_ptr,

    blob.bytes.getPtr,

    ret.bytes.getPtr
  )
  verify_result(res, ret)

proc computeCellsAndProofs*(ctx: KZGCtx, blob : Blob): Result[CellsAndProofs, string] {.gcsafe.} =
  var ret: CellsAndProofs

  let outCellsPtr = toPtrPtr(ret.cells)
  let outProofsPtr = toPtrPtr(ret.proofs)

  let res = eth_kzg_compute_cells_and_kzg_proofs(
    ctx.ctx_ptr,

    blob.bytes.getPtr,

    outCellsPtr,
    outProofsPtr
  )
  verify_result(res, ret)

proc verifyCellKZGProofBatch*(ctx: KZGCtx, commitments: openArray[Bytes48],
                   cellIndices: openArray[uint64],
                   cells: openArray[Cell],
                   proofs: openArray[Bytes48]): Result[bool, string] {.gcsafe.} =
  var valid: bool

  let cellsPtr = toPtrPtr(cells)
  let proofsPtr = toPtrPtr(proofs)
  let commitmentsPtr = toPtrPtr(commitments)

  let res = eth_kzg_verify_cell_kzg_proof_batch(
    ctx.ctx_ptr,

    uint64(len(commitments)),
    commitmentsPtr,

    uint64(len(cellIndices)),
    cellIndices.safeGetPtr,

    uint64(len(cells)),
    cellsPtr,

    uint64(len(proofs)),
    proofsPtr,

    valid.getPtr
  )
  verify_result(res, valid)

proc recoverCellsAndProofs*(ctx: KZGCtx,
                   cellIds: openArray[uint64],
                   cells: openArray[Cell]): Result[CellsAndProofs, string] {.gcsafe.} =

  var ret: CellsAndProofs

  let outCellsPtr = toPtrPtr(ret.cells)
  let outProofsPtr = toPtrPtr(ret.proofs)
  let inputCellsPtr = toPtrPtr(cells)

  let res = eth_kzg_recover_cells_and_proofs(
    ctx.ctx_ptr,

    uint64(len(cells)),
    inputCellsPtr,

    uint64(len(cellIds)),
    cellIds.safeGetPtr,

    outCellsPtr,
    outProofsPtr,
  )

  verify_result(res, ret)