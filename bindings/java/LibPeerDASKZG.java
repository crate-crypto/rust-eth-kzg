import java.util.Arrays;

class LibPeerDASKZG {
    private static native long proverContextNew();
    private static native void proverContextDestroy(long prover_ctx_ptr);
    private static native byte[] computeCells(long prover_context_ptr, byte[] blob);
    private static native byte[] computeCellsAndKZGProofs(long prover_context_ptr, byte[] blob);
    private static native byte[] blobToKZGCommitment(long prover_context_ptr, byte[] blob);
    
    private static native long verifierContextNew();
    private static native void verifierContextDestroy(long verifier_context_ptr);
    private static native boolean verifyCellKZGProof(long verifier_context_ptr, byte[] commitment, long cell_id, byte[] cell, byte[] proof);
    
    static {
      System.loadLibrary("java_peerdas_kzg");
    }
    
    // This is here so I can decode hex strings, remove once we are cleaning things up
    public static byte[] hexStringToByteArray(String s) {
      if (s == null) {
        throw new IllegalArgumentException("Input string cannot be null.");
      }

      // Remove all non-hex characters (optional, depends on input format)
      s = s.replaceAll("[^0-9A-Fa-f]", "");

      int len = s.length();
      if (len % 2 != 0) {
        throw new IllegalArgumentException("Hex string has an odd length, which means it's incomplete.");
      }

      byte[] data = new byte[len / 2]; // Allocate space for the byte array
      for (int i = 0; i < len; i += 2) {
        // Parse each hex pair as a byte
        data[i / 2] = (byte) ((Character.digit(s.charAt(i), 16) << 4)
            + Character.digit(s.charAt(i + 1), 16));
      }
      return data;
    }

    public static void main(String[] args) {

        final int blobSizeInBytes = 4096 * 32;
        byte[] byteArray = new byte[blobSizeInBytes];

        java.util.Arrays.fill(byteArray, (byte) 0);
        
        long prover_context_ptr = LibPeerDASKZG.proverContextNew();
        byte[] res = LibPeerDASKZG.computeCells(prover_context_ptr, byteArray);
        byte[] res2 = LibPeerDASKZG.computeCellsAndKZGProofs(prover_context_ptr, byteArray);
        byte[] res3 = LibPeerDASKZG.blobToKZGCommitment(prover_context_ptr, byteArray);
        System.out.println("hello there " + Arrays.toString(res));
    }
}