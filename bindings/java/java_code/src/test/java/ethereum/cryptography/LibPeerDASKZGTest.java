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

        long ctx_ptr = LibPeerDASKZG.peerDASContextNew();
        byte[] res = LibPeerDASKZG.computeCells(ctx_ptr, byteArray);
        byte[] res2 = LibPeerDASKZG.computeCellsAndKZGProofs(ctx_ptr, byteArray);
        byte[] res3 = LibPeerDASKZG.blobToKZGCommitment(ctx_ptr, byteArray);
    }
}