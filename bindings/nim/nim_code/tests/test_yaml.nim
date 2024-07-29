# This file was copied and modified from c-kzg
import
  std/[os, sequtils, strutils, streams],
  unittest2, yaml, results

import nim_eth_kzg

# Use our own fromHex implementation so that we can
# raise an error when the hex string is not the same
# size as the type we are trying to
import
  stew/byteutils except fromHex

const
  kzgPath* = currentSourcePath.rsplit(DirSep, 5)[0] & "/"
  testBase = kzgPath & "consensus_test_vectors/"
  BLOB_TO_KZG_COMMITMENT_TESTS = testBase & "blob_to_kzg_commitment"
  COMPUTE_CELLS_AND_KZG_PROOFS_TESTS = testBase & "compute_cells_and_kzg_proofs"
  VERIFY_CELL_KZG_PROOF_BATCH_TESTS = testBase & "verify_cell_kzg_proof_batch"
  RECOVER_CELLS_AND_PROOFS_TESTS = testBase & "recover_cells_and_kzg_proofs"

proc toTestName(x: string): string =
  let parts = x.split(DirSep)
  parts[^2]

proc loadYaml(filename: string): YamlNode =
  var s = newFileStream(filename)
  load(s, result)
  s.close()

proc fromHex(T: type, x: string): T =
  if (x.len - 2) div 2 > sizeof(result.bytes):
    raise newException(ValueError, "invalid hex")
  result.bytes = hexToByteArray(x, sizeof(result.bytes))

proc fromHex(T: type, x: YamlNode): T =
  T.fromHex(x.content)

proc fromHexList(T: type, xList: YamlNode): seq[T] =
  for x in xList:
    result.add(T.fromHex(x.content))

proc fromIntList(T: type, xList: YamlNode): seq[T] =
  for x in xList:
    result.add(x.content.parseInt().T)

template runTests(folder: string, body: untyped) =
  let test_files = walkDirRec(folder).toSeq()
  check test_files.len > 0
  for test_file in test_files:
    test toTestName(test_file):
      # nim template is hygienic, {.inject.} will allow body to
      # access injected symbol in current scope
      let n {.inject.} = loadYaml(test_file)
      try:
        body
      except ValueError:
        check n["output"].content == "null"

template checkRes(res, body: untyped) =
  if res.isErr:
    check n["output"].content == "null"
  else:
    body

template checkBytes48(res: untyped) =
  checkRes(res):
    let bytes = Bytes48.fromHex(n["output"])
    check bytes == res.get

template checkBool(res: untyped) =
  checkRes(res):
    check n["output"].content == $res.get

suite "yaml tests":
  var ctx: KZGCtx
  # We cannot run this in `setup` because that runs before _every_ test
  # and we only want to run this once
  #
  # This should also remove order dependency between tests; ie if we ran setup in a test
  ctx = newKZGCtx()

  runTests(BLOB_TO_KZG_COMMITMENT_TESTS):
    let
      blob = Blob.fromHex(n["input"]["blob"])
      res = ctx.blobToKZGCommitment(blob)
    checkBytes48(res)

  runTests(COMPUTE_CELLS_AND_KZG_PROOFS_TESTS):
    let
      blob = Blob.fromHex(n["input"]["blob"])
      res = ctx.computeCellsAndProofs(blob)

    checkRes(res):
      let cells = Cell.fromHexList(n["output"][0])
      check cells == res.get.cells
      let proofs = KZGProof.fromHexList(n["output"][1])
      check proofs == res.get.proofs

  runTests(RECOVER_CELLS_AND_PROOFS_TESTS):
    let
      cellIndices = uint64.fromIntList(n["input"]["cell_indices"])
      cells = Cell.fromHexList(n["input"]["cells"])
      res = ctx.recoverCellsAndProofs(cellIndices, cells)

    checkRes(res):
      let cells = Cell.fromHexList(n["output"][0])
      check cells == res.get.cells
      let proofs = KZGProof.fromHexList(n["output"][1])
      check proofs == res.get.proofs

  runTests(VERIFY_CELL_KZG_PROOF_BATCH_TESTS):
    let
      commitments = KZGCommitment.fromHexList(n["input"]["commitments"])
      cellIndices = uint64.fromIntList(n["input"]["cell_indices"])
      cells = Cell.fromHexList(n["input"]["cells"])
      proofs = KZGProof.fromHexList(n["input"]["proofs"])
      res = ctx.verifyCellKZGProofBatch(commitments, cellIndices, cells, proofs)
    checkBool(res)