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

public class LibPeerDASKZGTest {
    static LibPeerDASKZG context;

    @BeforeAll
    public static void setUp() {
        context = new LibPeerDASKZG();
    }

    @Test
    void testMultipleInstanceCreation() {
        LibPeerDASKZG instance1 = null;
        LibPeerDASKZG instance2 = null;
        try {
            instance1 = new LibPeerDASKZG();
            instance2 = new LibPeerDASKZG();

            assertNotNull(instance1);
            assertNotNull(instance2);
            assertNotEquals(instance1, instance2);

            // Test a simple operation to ensure both instances are functional
            byte[] dummyBlob = new byte[LibPeerDASKZG.BYTES_PER_BLOB];
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
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getRecoverCellsAndKzgProofsTests")
    public void recoverCellsAndKzgProofsTests(final RecoverCellsAndKzgProofsTest test) {
      try {
        final CellsAndProofs recoveredCellsAndProofs =
            context.recoverCellsAndProofs(
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
                    test.getInput().getRowCommitments(),
                    test.getInput().getRowIndices(),
                    test.getInput().getColumnIndices(),
                    test.getInput().getCells(),
                    test.getInput().getProofs());
            assertEquals(test.getOutput(), valid);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

}