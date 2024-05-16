import {ProverContextJs, VerifierContextJs} from "../index.js";

const proverContext = new ProverContextJs();
const verifierContext = new VerifierContextJs();

describe("ProverContext", () => {
  describe("blobToKzgCommitment", () => {
    test("function exists", () => {
      expect(proverContext.blobToKzgCommitment).toBeDefined();
    });
  });
  describe("asyncBlobToKzgCommitment", () => {
    test("function exists", () => {
      expect(proverContext.asyncBlobToKzgCommitment).toBeDefined();
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
