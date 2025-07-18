package ethereum.cryptography;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import java.util.stream.IntStream;
import java.util.stream.LongStream;
import java.util.stream.Stream;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;
import org.junit.jupiter.params.provider.MethodSource;

import ethereum.cryptography.test_formats.*;

public class LibEthKZGTest {
    static LibEthKZG context;

    @BeforeAll
    public static void setUp() {
        context = new LibEthKZG();
    }

    @Test
    void testMultipleInstanceCreation() {
        LibEthKZG instance1 = null;
        LibEthKZG instance2 = null;
        try {
            instance1 = new LibEthKZG();
            instance2 = new LibEthKZG();

            assertNotNull(instance1);
            assertNotNull(instance2);
            assertNotEquals(instance1, instance2);

            // Test a simple operation to ensure both instances are functional
            byte[] dummyBlob = new byte[LibEthKZG.BYTES_PER_BLOB];
            byte[] commitment1 = instance1.blobToKZGCommitment(dummyBlob);
            byte[] commitment2 = instance2.blobToKZGCommitment(dummyBlob);

            assertNotNull(commitment1);
            assertNotNull(commitment2);
            assertArrayEquals(commitment1, commitment2);
        } finally {
            if (instance1 != null) instance1.close();
            if (instance2 != null) instance2.close();
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getBlobToKzgCommitmentTests")
    public void blobToKzgCommitmentTests(final BlobToKzgCommitmentTest test) {
        try {
            byte[] commitment = context.blobToKZGCommitment(test.getInput().getBlob());
            assertArrayEquals(test.getOutput(), commitment);
        } catch (IllegalArgumentException e) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getComputeCellsAndKzgProofsTests")
    public void verifyComputeCellsAndKzgProofsTests(final ComputeCellsAndKzgProofsTest test) {
        try {
            CellsAndProofs cellsAndProofs = context.computeCellsAndKZGProofs(test.getInput().getBlob());
            assertArrayEquals(test.getOutput().getCells(), cellsAndProofs.getCells());
            assertArrayEquals(test.getOutput().getProofs(), cellsAndProofs.getProofs());

            Cells cells = context.computeCells(test.getInput().getBlob());
            assertArrayEquals(test.getOutput().getCells(), cells.getCells());
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getRecoverCellsAndKzgProofsTests")
    public void recoverCellsAndKzgProofsTests(final RecoverCellsAndKzgProofsTest test) {
      try {
        final CellsAndProofs recoveredCellsAndProofs =
            context.recoverCellsAndKZGProofs(
                test.getInput().getCellIndices(), test.getInput().getCells());
        assertArrayEquals(test.getOutput().getCells(), recoveredCellsAndProofs.getCells());
        assertArrayEquals(test.getOutput().getProofs(), recoveredCellsAndProofs.getProofs());
      } catch (IllegalArgumentException ex) {
        assertNull(test.getOutput());
      }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getVerifyCellKzgProofBatchTests")
    public void verifyCellKzgProofBatchTests(final VerifyCellKzgProofBatchTest test) {
        try {
            boolean valid = context.verifyCellKZGProofBatch(
                    test.getInput().getCommitments(),
                    test.getInput().getCellIndices(),
                    test.getInput().getCells(),
                    test.getInput().getProofs());
            assertEquals(test.getOutput(), valid);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getComputeKzgProofTests")
    public void computeKzgProofTests(final ComputeKzgProofTest test) {
        try {
            byte[][] result = context.computeKzgProof(test.getInput().getBlob(), test.getInput().getZ());
            assertArrayEquals(test.getOutput()[0], result[0]); // proof
            assertArrayEquals(test.getOutput()[1], result[1]); // y
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getComputeBlobKzgProofTests")
    public void computeBlobKzgProofTests(final ComputeBlobKzgProofTest test) {
        try {
            byte[] proof = context.computeBlobKzgProof(test.getInput().getBlob(), test.getInput().getCommitment());
            assertArrayEquals(test.getOutput(), proof);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getVerifyKzgProofTests")
    public void verifyKzgProofTests(final VerifyKzgProofTest test) {
        try {
            boolean valid = context.verifyKzgProof(
                test.getInput().getCommitment(),
                test.getInput().getZ(),
                test.getInput().getY(),
                test.getInput().getProof());
            assertEquals(test.getOutput(), valid);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getVerifyBlobKzgProofTests")
    public void verifyBlobKzgProofTests(final VerifyBlobKzgProofTest test) {
        try {
            boolean valid = context.verifyBlobKzgProof(
                test.getInput().getBlob(),
                test.getInput().getCommitment(),
                test.getInput().getProof());
            assertEquals(test.getOutput(), valid);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getVerifyBlobKzgProofBatchTests")
    public void verifyBlobKzgProofBatchTests(final VerifyBlobKzgProofBatchTest test) {
        try {
            boolean valid = context.verifyBlobKzgProofBatch(
                test.getInput().getBlobs(),
                test.getInput().getCommitments(),
                test.getInput().getProofs());
            assertEquals(test.getOutput(), valid);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

}