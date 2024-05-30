package ethereum.cryptography;

import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

public class LibPeerDASKZGTest {
    @BeforeAll
    public static void setUp() {
        LibPeerDASKZG.loadNativeLibrary();
    }

    @Test
    public void testCanCallLibrary() {
        final int blobSizeInBytes = 4096 * 32;
        byte[] byteArray = new byte[blobSizeInBytes];

        java.util.Arrays.fill(byteArray, (byte) 0);

        long prover_context_ptr = LibPeerDASKZG.proverContextNew();
        byte[] res = LibPeerDASKZG.computeCells(prover_context_ptr, byteArray);
        byte[] res2 = LibPeerDASKZG.computeCellsAndKZGProofs(prover_context_ptr, byteArray);
        byte[] res3 = LibPeerDASKZG.blobToKZGCommitment(prover_context_ptr, byteArray);
    }
}