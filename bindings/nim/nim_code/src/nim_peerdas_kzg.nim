import nim_peerdas_kzg/bindings

import results
export results

# TODO: If the underlying c library changes and we recompile the static lib
# TODO: nim will not recompile the tests. see test_yaml does not change for example
const
  FIELD_ELEMENTS_PER_BLOB = 4096
  FIELD_ELEMENTS_PER_CELL = 64
  BYTES_PER_FIELD_ELEMENT = 32
  CELLS_PER_EXT_BLOB = 128
  KzgBlobSize* = FIELD_ELEMENTS_PER_BLOB*BYTES_PER_FIELD_ELEMENT
  KzgCellSize* = FIELD_ELEMENTS_PER_CELL*BYTES_PER_FIELD_ELEMENT

# TODO: Inconsistency between the writing of kzg. Decide between `Kzg` and `KZG`
# TODO: I think KZG is the correct term since its an abbreviation
type
  KzgBytes48* = object
    bytes*: array[48, byte]

  KzgBlob* = object
    bytes*: array[KzgBlobSize, byte]

  KzgCell* = object
    bytes*: array[KzgCellSize, byte]

  KzgCommitment* = KzgBytes48
  
  KzgProof* = KzgBytes48

  KzgCells* = array[CELLS_PER_EXT_BLOB, KzgCell]

  KzgCellsAndKzgProofs* = object
    cells*: KzgCells
    proofs*: array[CELLS_PER_EXT_BLOB, KzgProof]


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
    # TODO: get error message then free the pointer
    return err($res)
  ok(ret)

type
  KzgCtx* = ref object
    ctx_ptr: ptr PeerDASContext

proc newKzgCtx*(): KzgCtx =
  var kzgCtx = KzgCtx()
  kzgCtx.ctx_ptr = peerdas_context_new()
  return kzgCtx

proc blobToKZGCommitment*(ctx: KzgCtx, blob : KzgBlob): Result[KzgCommitment, string] {.gcsafe.} =
  var ret: KzgCommitment
  
  let res = blob_to_kzg_commitment(
    ctx.ctx_ptr, 
    
    uint64(len(blob.bytes)),
    blob.bytes.getPtr, 
    
    ret.bytes.getPtr
  )
  verify_result(res, ret)


proc computeCellsAndProofs*(ctx: KzgCtx, blob : KzgBlob): Result[KzgCellsAndKzgProofs, string] {.gcsafe.} =
  var ret: KzgCellsAndKzgProofs

  let outCellsPtr = toPtrPtr(ret.cells) 
  let outProofsPtr = toPtrPtr(ret.proofs) 
  
  let res = compute_cells_and_kzg_proofs_deflattened(
    ctx.ctx_ptr,

    uint64(len(blob.bytes)),
    blob.bytes.getPtr,
    
    outCellsPtr,
    outProofsPtr
  )
  verify_result(res, ret)

proc computeCells*(ctx: KzgCtx, blob : KzgBlob): Result[KzgCells, string] {.gcsafe.} =  
  let res = ?computeCellsAndProofs(ctx, blob)
  ok(res.cells)

proc verifyCellKZGProof*(ctx: KzgCtx, commitment: KzgBytes48, cellId: uint64, cell: KzgCell, proof: KzgBytes48): Result[bool, string] =
  var valid: bool

  let res =  verify_cell_kzg_proof(
    ctx.ctx_ptr, 
    
    uint64(len(cell.bytes)),
    cell.bytes.getPtr,
    
    uint64(len(commitment.bytes)),
    commitment.bytes.getPtr,
    
    cellId,

    uint64(len(proof.bytes)),
    proof.bytes.getPtr, 
    
    valid.getPtr
  )
  verify_result(res, valid)

proc verifyCellKZGProofBatch*(ctx: KzgCtx, rowCommitments: openArray[KzgBytes48],
                   rowIndices: openArray[uint64],
                   columnIndices: openArray[uint64],
                   cells: openArray[KzgCell],
                   proofs: openArray[KzgBytes48]): Result[bool, string] {.gcsafe.} =
  var valid: bool

  let cellsPtr = toPtrPtr(cells) 
  let proofsPtr = toPtrPtr(proofs) 
  let commitmentsPtr = toPtrPtr(rowCommitments)

  let res = verify_cell_kzg_proof_batch(
    ctx.ctx_ptr, 
    
    uint64(len(rowCommitments)), 
    commitmentsPtr, 
    
    uint64(len(rowIndices)),
    rowIndices.safeGetPtr, 
    
    uint64(len(columnIndices)), 
    columnIndices.safeGetPtr,
    
    uint64(len(cells)),
    cellsPtr, 
    
    uint64(len(proofs)),
    proofsPtr,

    valid.getPtr
  )
  verify_result(res, valid)


proc recoverCellsAndProofs*(ctx: KzgCtx,
                   cellIds: openArray[uint64],
                   cells: openArray[KzgCell]): Result[KzgCellsAndKzgProofs, string] {.gcsafe.} =
  
  var ret: KzgCellsAndKzgProofs
  
  let outCellsPtr = toPtrPtr(ret.cells) 
  let outProofsPtr = toPtrPtr(ret.proofs) 
  let inputCellsPtr = toPtrPtr(cells)

  let res = recover_cells_and_proofs(
    ctx.ctx_ptr,

    uint64(len(cells)),
    inputCellsPtr,
    
    uint64(len(cellIds)),
    cellIds.safeGetPtr,
    
    outCellsPtr,
    outProofsPtr,
  )

  verify_result(res, ret)

proc recoverCells*(ctx: KzgCtx,
                   cellIds: openArray[uint64],
                   cells: openArray[KzgCell]): Result[KzgCells, string] {.gcsafe.} =
  let res = ?recoverCellsAndProofs(ctx, cellIds, cells)
  ok(res.cells)