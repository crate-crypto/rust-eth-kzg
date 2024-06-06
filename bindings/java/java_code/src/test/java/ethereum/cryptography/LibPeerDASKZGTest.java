package ethereum.cryptography;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
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
    static long ctx_ptr;

    @BeforeAll
    public static void setUp() {
        LibPeerDASKZG.loadNativeLibrary();
        ctx_ptr = LibPeerDASKZG.peerDASContextNew();
    }

    @Test
    public void testCanCallLibrary() {
        final int blobSizeInBytes = 4096 * 32;
        byte[] byteArray = new byte[blobSizeInBytes];

        java.util.Arrays.fill(byteArray, (byte) 0);

        byte[] res = LibPeerDASKZG.computeCells(ctx_ptr, byteArray);
        CellsAndProofs res2 = LibPeerDASKZG.computeCellsAndKZGProofs(ctx_ptr, byteArray);
        byte[] res3 = LibPeerDASKZG.blobToKZGCommitment(ctx_ptr, byteArray);
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getBlobToKzgCommitmentTests")
    public void blobToKzgCommitmentTests(final BlobToKzgCommitmentTest test) {
        try {
            byte[] commitment = LibPeerDASKZG.blobToKZGCommitment(ctx_ptr, test.getInput().getBlob());
            assertArrayEquals(test.getOutput(), commitment);
        } catch (IllegalArgumentException e) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getComputeCellsTests")
    public void verifyComputeCellsTests(final ComputeCellsTest test) {
        try {
            byte[] cells = LibPeerDASKZG.computeCells(ctx_ptr,test.getInput().getBlob());
            assertArrayEquals(test.getOutput(), cells);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getComputeCellsAndKzgProofsTests")
    public void verifyComputeCellsAndKzgProofsTests(final ComputeCellsAndKzgProofsTest test) {
        try {
            CellsAndProofs cellsAndProofs = LibPeerDASKZG.computeCellsAndKZGProofs(ctx_ptr, test.getInput().getBlob());
            assertArrayEquals(test.getOutput().getCells(), cellsAndProofs.getCells());
            assertArrayEquals(test.getOutput().getProofs(), cellsAndProofs.getProofs());
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getVerifyCellKzgProofTests")
    public void verifyCellKzgProofTests(final VerifyCellKzgProofTest test) {
        try {
            boolean valid = LibPeerDASKZG.verifyCellKZGProof(
                    ctx_ptr, 
                    test.getInput().getCommitment(),
                    test.getInput().getCellId(),
                    test.getInput().getCell(),
                    test.getInput().getProof());
            assertEquals(test.getOutput(), valid);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getVerifyCellKzgProofBatchTests")
    public void verifyCellKzgProofBatchTests(final VerifyCellKzgProofBatchTest test) {
        try {
            boolean valid = LibPeerDASKZG.verifyCellKZGProofBatch(
                    ctx_ptr,
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

    @ParameterizedTest
    @MethodSource("ethereum.cryptography.TestUtils#getRecoverAllCellsTests")
    public void recoverAllCellsTests(final RecoverAllCellsTest test) {
        try {
            byte[] cells = LibPeerDASKZG.recoverAllCells(ctx_ptr, test.getInput().getCellIds(), test.getInput().getCells());
            assertArrayEquals(test.getOutput(), cells);
        } catch (IllegalArgumentException ex) {
            assertNull(test.getOutput());
        }
    }

}