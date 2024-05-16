import crypto from "crypto";
import {
  ProverContextJs,
  VerifierContextJs,
  BYTES_PER_COMMITMENT,
  BYTES_PER_BLOB,
  BYTES_PER_FIELD_ELEMENT,
} from "../index.js";

const MAX_TOP_BYTE = 114;

/**
 * Generates a random blob of the correct length for the KZG library
 *
 * @return {Uint8Array}
 */
function generateRandomBlob(): Uint8Array {
  return new Uint8Array(
    crypto.randomBytes(BYTES_PER_BLOB).map((x, i) => {
      // Set the top byte to be low enough that the field element doesn't overflow the BLS modulus
      if (x > MAX_TOP_BYTE && i % BYTES_PER_FIELD_ELEMENT == 0) {
        return Math.floor(Math.random() * MAX_TOP_BYTE);
      }
      return x;
    })
  );
}

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
  describe("blobToKzgCommitment", () => {
    test("function exists", () => {
      expect(proverContext.blobToKzgCommitment).toBeDefined();
    });
    test("creates a commitment", () => {
      const blob = generateRandomBlob();
      const commitment = proverContext.blobToKzgCommitment(blob);
      expect(commitment).toBeInstanceOf(Uint8Array);
      expect(commitment.length).toEqual(BYTES_PER_COMMITMENT);
    });
  });
  describe("asyncBlobToKzgCommitment", () => {
    test("function exists", () => {
      expect(proverContext.asyncBlobToKzgCommitment).toBeDefined();
    });
    test("creates a commitment", async () => {
      const blob = generateRandomBlob();
      const commitment = await proverContext.asyncBlobToKzgCommitment(blob);
      expect(commitment).toBeInstanceOf(Uint8Array);
      expect(commitment.length).toEqual(BYTES_PER_COMMITMENT);
    });
  });
  describe("computeCellsAndKzgProofs", () => {
    test("function exists", () => {
      expect(proverContext.computeCellsAndKzgProofs).toBeDefined();
    });
  });
  describe("asyncComputeCellsAndKzgProofs", () => {
    test("function exists", () => {
      expect(proverContext.asyncComputeCellsAndKzgProofs).toBeDefined();
    });
  });
  describe("computeCells", () => {
    test("function exists", () => {
      expect(proverContext.computeCells).toBeDefined();
    });
  });
  describe("asyncComputeCells", () => {
    test("function exists", () => {
      expect(proverContext.asyncComputeCells).toBeDefined();
    });
  });
});

describe("VerifierContext", () => {
  const verifierContext = new VerifierContextJs();
  describe("verifyCellKzgProof", () => {
    test("function exists", () => {
      expect(verifierContext.verifyCellKzgProof).toBeDefined();
    });
  });
  describe("asyncVerifyCellKzgProof", () => {
    test("function exists", () => {
      expect(verifierContext.asyncVerifyCellKzgProof).toBeDefined();
    });
  });
});
