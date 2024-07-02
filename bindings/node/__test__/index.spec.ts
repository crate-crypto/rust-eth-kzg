import crypto from "crypto";
import {
  ProverContextJs,
  VerifierContextJs,
} from "../index.js";

import { readFileSync } from "fs";
import { globSync } from "glob";

const yaml = require("js-yaml");

interface TestMeta<I extends Record<string, any>, O extends boolean | string | string[] | Record<string, any>> {
  input: I;
  output: O;
}

const BLOB_TO_KZG_COMMITMENT_TESTS = "../../consensus_test_vectors/blob_to_kzg_commitment/*/*/data.yaml";
const COMPUTE_CELLS_TESTS = "../../consensus_test_vectors/compute_cells/*/*/data.yaml";
const COMPUTE_CELLS_AND_KZG_PROOFS_TESTS = "../../consensus_test_vectors/compute_cells_and_kzg_proofs/*/*/data.yaml";
const VERIFY_CELL_KZG_PROOF_TESTS = "../../consensus_test_vectors/verify_cell_kzg_proof/*/*/data.yaml";
const VERIFY_CELL_KZG_PROOF_BATCH_TESTS = "../../consensus_test_vectors/verify_cell_kzg_proof_batch/*/*/data.yaml";
const RECOVER_ALL_CELLS_TESTS = "../../consensus_test_vectors/recover_all_cells/*/*/data.yaml";
const RECOVER_CELLS_AND_KZG_PROOFS_TEST = "../../consensus_test_vectors/recover_cells_and_kzg_proofs/*/*/data.yaml";

type BlobToKzgCommitmentTest = TestMeta<{ blob: string }, string>;
type ComputeCellsTest = TestMeta<{ blob: string }, string[]>;
type ComputeCellsAndKzgProofsTest = TestMeta<{ blob: string }, string[][]>;
// TODO: number here is incorrect, but it might be worthwhile to change the type in the specs instead
type VerifyCellKzgProofTest = TestMeta<{ commitment: string; cell_id: number; cell: string; proof: string }, boolean>;
type VerifyCellKzgProofBatchTest = TestMeta<
  { row_commitments: string[]; row_indices: number[]; column_indices: number[]; cells: string[]; proofs: string[] },
  boolean
>;
type RecoverAllCellsTest = TestMeta<{ cell_ids: number[]; cells: string[] }, string[]>;
type RecoverCellsAndKzgProofsTest = TestMeta<{cell_indices: number[]; cells: string[]}, string[][]>;

/**
 * Converts hex string to binary Uint8Array
 *
 * @param {string} hexString Hex string to convert
 *
 * @return {Uint8Array}
 */
function bytesFromHex(hexString: string): Uint8Array {
  if (hexString.startsWith("0x")) {
    hexString = hexString.slice(2);
  }
  return Uint8Array.from(Buffer.from(hexString, "hex"));
}

/**
 * Verifies that two Uint8Arrays are bitwise equivalent
 *
 * @param {Uint8Array} a
 * @param {Uint8Array} b
 *
 * @return {void}
 *
 * @throws {Error} If arrays are not equal length or byte values are unequal
 */
function assertBytesEqual(a: Uint8Array | Buffer, b: Uint8Array | Buffer): void {
  if (a.length !== b.length) {
    throw new Error("unequal Uint8Array lengths");
  }
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) throw new Error(`unequal Uint8Array byte at index ${i}`);
  }
}

describe("ProverContext", () => {
  const proverContext = new ProverContextJs();

  it("reference tests for blobToKzgCommitment should pass", () => {
    const tests = globSync(BLOB_TO_KZG_COMMITMENT_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: BlobToKzgCommitmentTest = yaml.load(readFileSync(testFile, "ascii"));

      let commitment: Uint8Array;
      const blob = bytesFromHex(test.input.blob);

      try {
        commitment = proverContext.blobToKzgCommitment(blob);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(test.output).not.toBeNull();
      const expectedCommitment = bytesFromHex(test.output);
      expect(assertBytesEqual(commitment, expectedCommitment));
    });
  });


  it("reference tests for computeCells should pass", () => {
    const tests = globSync(COMPUTE_CELLS_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: ComputeCellsTest = yaml.load(readFileSync(testFile, "ascii"));

      let cells;
      const blob = bytesFromHex(test.input.blob);

      try {
        cells = proverContext.computeCells(blob);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(test.output).not.toBeNull();
      const expectedCells = test.output.map(bytesFromHex);
      expect(cells.length).toBe(expectedCells.length);
      for (let i = 0; i < cells.length; i++) {
        assertBytesEqual(cells[i], expectedCells[i]);
      }
    });
  });

  it("reference tests for computeCellsAndKzgProofs should pass", () => {
    const tests = globSync(COMPUTE_CELLS_AND_KZG_PROOFS_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: ComputeCellsAndKzgProofsTest = yaml.load(readFileSync(testFile, "ascii"));

      let cells_and_proofs;
      const blob = bytesFromHex(test.input.blob);

      try {
        cells_and_proofs = proverContext.computeCellsAndKzgProofs(blob);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      let cells = cells_and_proofs.cells;
      let proofs = cells_and_proofs.proofs;

      expect(test.output).not.toBeNull();
      expect(test.output.length).toBe(2);
      const expectedCells = test.output[0].map(bytesFromHex);
      const expectedProofs = test.output[1].map(bytesFromHex);
      expect(cells.length).toBe(expectedCells.length);
      for (let i = 0; i < cells.length; i++) {
        assertBytesEqual(cells[i], expectedCells[i]);
      }
      expect(proofs.length).toBe(expectedProofs.length);
      for (let i = 0; i < proofs.length; i++) {
        assertBytesEqual(proofs[i], expectedProofs[i]);
      }
    });
  });

  it("reference tests for recoverCellsAndKzgProofs should pass", () => {
    const tests = globSync(RECOVER_CELLS_AND_KZG_PROOFS_TEST);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: RecoverCellsAndKzgProofsTest = yaml.load(readFileSync(testFile, "ascii"));

      let recoveredCellsAndProofs;
      const cellIndices = test.input.cell_indices.map((x) => BigInt(x));
      const cells = test.input.cells.map(bytesFromHex);

      try {
        recoveredCellsAndProofs = proverContext.recoverCellsAndKzgProofs(cellIndices, cells);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      let recoveredCells = recoveredCellsAndProofs.cells;
      let recoveredProofs = recoveredCellsAndProofs.proofs;

      expect(test.output).not.toBeNull();
      expect(test.output.length).toBe(2);
      const expectedCells = test.output[0].map(bytesFromHex);
      const expectedProofs = test.output[1].map(bytesFromHex);
      expect(recoveredCells.length).toBe(expectedCells.length);
      for (let i = 0; i < recoveredCells.length; i++) {
        assertBytesEqual(recoveredCells[i], expectedCells[i]);
      }
      expect(recoveredProofs.length).toBe(expectedProofs.length);
      for (let i = 0; i < recoveredProofs.length; i++) {
        assertBytesEqual(recoveredProofs[i], expectedProofs[i]);
      }
    });
  });

});

describe("VerifierContext", () => {
  const verifierContext = new VerifierContextJs();

  it("reference tests for recoverAllCells should pass", () => {
    const tests = globSync(RECOVER_ALL_CELLS_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: RecoverAllCellsTest = yaml.load(readFileSync(testFile, "ascii"));

      let recovered;
      const cellIds = test.input.cell_ids.map((x) => BigInt(x));
      const cells = test.input.cells.map(bytesFromHex);

      try {
        recovered = verifierContext.recoverAllCells(cellIds, cells);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(test.output).not.toBeNull();
      const expectedCells = test.output.map(bytesFromHex);
      expect(recovered.length).toBe(expectedCells.length);
      for (let i = 0; i < cells.length; i++) {
        assertBytesEqual(recovered[i], expectedCells[i]);
      }
    });
  });

  it("reference tests for verifyCellKzgProofBatch should pass", () => {
    const tests = globSync(VERIFY_CELL_KZG_PROOF_BATCH_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: VerifyCellKzgProofBatchTest = yaml.load(readFileSync(testFile, "ascii"));

      let valid;
      const rowCommitments = test.input.row_commitments.map(bytesFromHex);
      const rowIndices = test.input.row_indices.map((x) => BigInt(x));
      const columnIndices = test.input.column_indices.map((x) => BigInt(x));
      const cells = test.input.cells.map(bytesFromHex);
      const proofs = test.input.proofs.map(bytesFromHex);

      try {
        valid = verifierContext.verifyCellKzgProofBatch(rowCommitments, rowIndices, columnIndices, cells, proofs);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(valid).toEqual(test.output);
    });
  });

  it("reference tests for verifyCellKzgProof should pass", () => {
    const tests = globSync(VERIFY_CELL_KZG_PROOF_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {

      const test: VerifyCellKzgProofTest = yaml.load(readFileSync(testFile, "ascii"));

      let valid;
      const commitment = bytesFromHex(test.input.commitment);
      const cellId = BigInt(test.input.cell_id);
      const cell = bytesFromHex(test.input.cell);
      const proof = bytesFromHex(test.input.proof);

      try {
        valid = verifierContext.verifyCellKzgProof(commitment, cellId, cell, proof);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(valid).toEqual(test.output);
    });
  });
});
