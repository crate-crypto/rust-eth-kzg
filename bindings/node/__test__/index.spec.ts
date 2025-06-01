import {
  DasContextJs,
} from "../index.js";

import { readFileSync } from "fs";
import { globSync } from "glob";

const yaml = require("js-yaml");

interface TestMeta<I extends Record<string, any>, O extends boolean | string | string[] | Record<string, any>> {
  input: I;
  output: O;
}

const BLOB_TO_KZG_COMMITMENT_TESTS = "../../test_vectors/blob_to_kzg_commitment/*/*/data.yaml";
const COMPUTE_CELLS_AND_KZG_PROOFS_TESTS = "../../test_vectors/compute_cells_and_kzg_proofs/*/*/data.yaml";
const VERIFY_CELL_KZG_PROOF_BATCH_TESTS = "../../test_vectors/verify_cell_kzg_proof_batch/*/*/data.yaml";
const RECOVER_CELLS_AND_KZG_PROOFS_TEST = "../../test_vectors/recover_cells_and_kzg_proofs/*/*/data.yaml";
const COMPUTE_KZG_PROOF_TESTS = "../../test_vectors/compute_kzg_proof/*/*/data.yaml";
const COMPUTE_BLOB_KZG_PROOF_TESTS = "../../test_vectors/compute_blob_kzg_proof/*/*/data.yaml";
const VERIFY_KZG_PROOF_TESTS = "../../test_vectors/verify_kzg_proof/*/*/data.yaml";
const VERIFY_BLOB_KZG_PROOF_TESTS = "../../test_vectors/verify_blob_kzg_proof/*/*/data.yaml";
const VERIFY_BLOB_KZG_PROOF_BATCH_TESTS = "../../test_vectors/verify_blob_kzg_proof_batch/*/*/data.yaml";

type BlobToKzgCommitmentTest = TestMeta<{ blob: string }, string>;
type ComputeCellsAndKzgProofsTest = TestMeta<{ blob: string }, string[][]>;
type VerifyCellKzgProofBatchTest = TestMeta<
  { commitments: string[]; cell_indices: number[]; cells: string[]; proofs: string[] },
  boolean
>;
type RecoverCellsAndKzgProofsTest = TestMeta<{ cell_indices: number[]; cells: string[] }, string[][]>;
type ComputeKzgProofTest = TestMeta<{ blob: string; z: string }, string[]>;
type ComputeBlobKzgProofTest = TestMeta<{ blob: string; commitment: string }, string>;
type VerifyKzgProofTest = TestMeta<{ commitment: string; z: string; y: string; proof: string }, boolean>;
type VerifyBlobKzgProofTest = TestMeta<{ blob: string; commitment: string; proof: string }, boolean>;
type VerifyBlobKzgProofBatchTest = TestMeta<{ blobs: string[]; commitments: string[]; proofs: string[] }, boolean>;

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

describe("Spec tests", () => {
  const ctx = new DasContextJs();

  it("reference tests for blobToKzgCommitment should pass", () => {
    const tests = globSync(BLOB_TO_KZG_COMMITMENT_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: BlobToKzgCommitmentTest = yaml.load(readFileSync(testFile, "ascii"));

      let commitment: Uint8Array;
      const blob = bytesFromHex(test.input.blob);

      try {
        commitment = ctx.blobToKzgCommitment(blob);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(test.output).not.toBeNull();
      const expectedCommitment = bytesFromHex(test.output);
      expect(assertBytesEqual(commitment, expectedCommitment));
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
        cells_and_proofs = ctx.computeCellsAndKzgProofs(blob);
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

      try {
        cells = ctx.computeCells(blob);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(cells.length).toBe(expectedCells.length);
      for (let i = 0; i < cells.length; i++) {
        assertBytesEqual(cells[i], expectedCells[i]);
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
        recoveredCellsAndProofs = ctx.recoverCellsAndKzgProofs(cellIndices, cells);
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

  it("reference tests for verifyCellKzgProofBatch should pass", () => {
    const tests = globSync(VERIFY_CELL_KZG_PROOF_BATCH_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: VerifyCellKzgProofBatchTest = yaml.load(readFileSync(testFile, "ascii"));

      let valid;
      const commitments = test.input.commitments.map(bytesFromHex);
      const cellIndices = test.input.cell_indices.map((x) => BigInt(x));
      const cells = test.input.cells.map(bytesFromHex);
      const proofs = test.input.proofs.map(bytesFromHex);

      try {
        valid = ctx.verifyCellKzgProofBatch(commitments, cellIndices, cells, proofs);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(valid).toEqual(test.output);
    });
  });

  it("reference tests for computeKzgProof should pass", () => {
    const tests = globSync(COMPUTE_KZG_PROOF_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: ComputeKzgProofTest = yaml.load(readFileSync(testFile, "ascii"));

      let result: Uint8Array[];
      const blob = bytesFromHex(test.input.blob);
      const z = bytesFromHex(test.input.z);

      try {
        result = ctx.computeKzgProof(blob, z);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(test.output).not.toBeNull();
      expect(result.length).toBe(2);
      const expectedProof = bytesFromHex(test.output[0]);
      const expectedY = bytesFromHex(test.output[1]);
      assertBytesEqual(result[0], expectedProof);
      assertBytesEqual(result[1], expectedY);
    });
  });

  it("reference tests for computeBlobKzgProof should pass", () => {
    const tests = globSync(COMPUTE_BLOB_KZG_PROOF_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: ComputeBlobKzgProofTest = yaml.load(readFileSync(testFile, "ascii"));

      let proof: Uint8Array;
      const blob = bytesFromHex(test.input.blob);
      const commitment = bytesFromHex(test.input.commitment);

      try {
        proof = ctx.computeBlobKzgProof(blob, commitment);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(test.output).not.toBeNull();
      const expectedProof = bytesFromHex(test.output);
      assertBytesEqual(proof, expectedProof);
    });
  });

  it("reference tests for verifyKzgProof should pass", () => {
    const tests = globSync(VERIFY_KZG_PROOF_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: VerifyKzgProofTest = yaml.load(readFileSync(testFile, "ascii"));

      let valid: boolean;
      const commitment = bytesFromHex(test.input.commitment);
      const z = bytesFromHex(test.input.z);
      const y = bytesFromHex(test.input.y);
      const proof = bytesFromHex(test.input.proof);

      try {
        valid = ctx.verifyKzgProof(commitment, z, y, proof);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(valid).toEqual(test.output);
    });
  });

  it("reference tests for verifyBlobKzgProof should pass", () => {
    const tests = globSync(VERIFY_BLOB_KZG_PROOF_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: VerifyBlobKzgProofTest = yaml.load(readFileSync(testFile, "ascii"));

      let valid: boolean;
      const blob = bytesFromHex(test.input.blob);
      const commitment = bytesFromHex(test.input.commitment);
      const proof = bytesFromHex(test.input.proof);

      try {
        valid = ctx.verifyBlobKzgProof(blob, commitment, proof);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(valid).toEqual(test.output);
    });
  });

  it("reference tests for verifyBlobKzgProofBatch should pass", () => {
    const tests = globSync(VERIFY_BLOB_KZG_PROOF_BATCH_TESTS);
    expect(tests.length).toBeGreaterThan(0);

    tests.forEach((testFile: string) => {
      const test: VerifyBlobKzgProofBatchTest = yaml.load(readFileSync(testFile, "ascii"));

      let valid: boolean;
      const blobs = test.input.blobs.map(bytesFromHex);
      const commitments = test.input.commitments.map(bytesFromHex);
      const proofs = test.input.proofs.map(bytesFromHex);

      try {
        valid = ctx.verifyBlobKzgProofBatch(blobs, commitments, proofs);
      } catch (err) {
        expect(test.output).toBeNull();
        return;
      }

      expect(valid).toEqual(test.output);
    });
  });
});
