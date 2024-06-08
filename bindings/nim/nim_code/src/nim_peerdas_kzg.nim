import nim_peerdas_kzg/bindings

import results, sequtils
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
    cells*: array[CELLS_PER_EXT_BLOB, KzgCell]
    proofs*: array[CELLS_PER_EXT_BLOB, KzgProof]

# Generic helper function to convert any object with a `bytes` array to a sequence of bytes
proc toByteSeq[T](obj: T): seq[byte] =
  result = obj.bytes.toSeq
# Function to flatten openArray to a sequence of bytes
proc flattenToBytes[T](arr: openArray[T]): seq[byte] =
  result = arr.mapIt(toByteSeq(it)).concat()

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

template verify_result(res: CResult, ret: untyped): untyped =
  if res.xstatus != CResultStatus.Ok:
    # TODO: get error messae then free the pointer
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

proc computeCells*(ctx: KzgCtx, blob : KzgBlob): Result[KzgCells, string] {.gcsafe.} =
  var ret: KzgCells

  let res = compute_cells(
    ctx.ctx_ptr,

    uint64(len(blob.bytes)),
    blob.bytes.getPtr, 
    
    ret.getPtr
  )
  verify_result(res, ret)

proc computeCellsAndProofs*(ctx: KzgCtx, blob : KzgBlob): Result[KzgCellsAndKzgProofs, string] {.gcsafe.} =
  var ret: KzgCellsAndKzgProofs
  
  let
    outCellsPtr = ret.cells.getPtr
    outProofsPtr = ret.proofs.getPtr
  
  let res = compute_cells_and_kzg_proofs(
    ctx.ctx_ptr,

    uint64(len(blob.bytes)),
    blob.bytes.getPtr,
    
    outCellsPtr,
    outProofsPtr
  )
  verify_result(res, ret)

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

  # TODO: I can see an argument to having pointers to an array of 
  # TODO: arrays, so that we do not need to flatten this.
  # TODO: not necessarilly important and should not afect the outwards facing API
  let rowCommitmentsFlattened = flattenToBytes(rowCommitments)
  let proofsFlattened = flattenToBytes(proofs)
  let cellsFlattened = flattenToBytes(cells)
  

  let res = verify_cell_kzg_proof_batch(
    ctx.ctx_ptr, 
    
    uint64(len(rowCommitmentsFlattened)), 
    rowCommitmentsFlattened.safeGetPtr, 
    
    uint64(len(rowIndices)),
    rowIndices.safeGetPtr, 
    
    uint64(len(columnIndices)), 
    columnIndices.safeGetPtr,
    
    uint64(len(cellsFlattened)),
    cellsFlattened.safeGetPtr, 
    
    uint64(len(proofsFlattened)),
    proofsFlattened.safeGetPtr,

    valid.getPtr
  )
  verify_result(res, valid)

proc recoverCells*(ctx: KzgCtx,
                   cellIds: openArray[uint64],
                   cells: openArray[KzgCell]): Result[KzgCells, string] {.gcsafe.} =
  
  var ret: KzgCells

  # TODO: For now, we have a check to check for empty arrays since the indexing logic would panic
  # The ideal way to handle this would be to pass a nullptr

  let cellsFlattened = flattenToBytes(cells)
  
  let res = recover_all_cells(
    ctx.ctx_ptr,

    uint64(len(cellsFlattened)),
    cellsFlattened.safeGetPtr,
    
    uint64(len(cellIds)),
    cellIds.safeGetPtr,
    
    ret.getPtr
  )

  verify_result(res, ret)