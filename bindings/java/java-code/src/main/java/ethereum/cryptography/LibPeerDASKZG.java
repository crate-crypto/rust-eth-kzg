package ethereum.cryptography;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

public class LibPeerDASKZG {
    private static native long proverContextNew();

    private static native void proverContextDestroy(long prover_ctx_ptr);

    private static native byte[] computeCells(long prover_context_ptr, byte[] blob);

    private static native byte[] computeCellsAndKZGProofs(long prover_context_ptr, byte[] blob);

    private static native byte[] blobToKZGCommitment(long prover_context_ptr, byte[] blob);

    private static native long verifierContextNew();

    private static native void verifierContextDestroy(long verifier_context_ptr);

    private static native boolean verifyCellKZGProof(
            long verifier_context_ptr, byte[] commitment, long cell_id, byte[] cell, byte[] proof);

    private static final String LIBRARY_NAME = "java_peerdas_kzg";
    private static final String PLATFORM_NATIVE_LIBRARY_NAME = System.mapLibraryName(LIBRARY_NAME);

    /** Loads the appropriate native library based on your platform. */
    // Copied from c-kzg
    public static void loadNativeLibrary() {

        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = System.getProperty("os.arch").toLowerCase();
        String libraryResourcePath = null;

        if (osName.contains("win")) {
            if (osArch.contains("amd64") || osArch.contains("x86_64")) {
                libraryResourcePath = "/x86_64-windows/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("x86")) {
                libraryResourcePath = "/x86-windows/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("arm64")) {
                libraryResourcePath = "/arm64-windows/" + PLATFORM_NATIVE_LIBRARY_NAME;
            }
        } else if (osName.contains("mac")) {
            if (osArch.contains("x86_64")) {
                libraryResourcePath = "/x86_64-apple-darwin/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("aarch64")) {
                libraryResourcePath = "/aarch64-apple-darwin/" + PLATFORM_NATIVE_LIBRARY_NAME;
            }
        } else if (osName.contains("linux")) {
            if (osArch.contains("x86_64")) {
                libraryResourcePath = "/x86_64-unknown-linux-gnu/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("aarch64")) {
                libraryResourcePath = "/aarch64-unknown-linux-gnu/" + PLATFORM_NATIVE_LIBRARY_NAME;
            }
        }

        if (libraryResourcePath == null) {
            throw new UnsupportedOperationException("Unsupported OS or architecture: " + osName + ", " + osArch);
        }

        InputStream libraryResource = LibPeerDASKZG.class.getResourceAsStream(libraryResourcePath);

        if (libraryResource == null) {
            try {
                System.loadLibrary(LIBRARY_NAME);
            } catch (UnsatisfiedLinkError __) {
                String exceptionMessage =
                        String.format(
                                "Couldn't load native library (%s). It wasn't available at %s or the library path.",
                                LIBRARY_NAME, libraryResourcePath);
                throw new RuntimeException(exceptionMessage);
            }
        } else {
            try {
                Path tempDir = Files.createTempDirectory(LIBRARY_NAME + "@");
                tempDir.toFile().deleteOnExit();
                Path tempDll = tempDir.resolve(PLATFORM_NATIVE_LIBRARY_NAME);
                tempDll.toFile().deleteOnExit();
                Files.copy(libraryResource, tempDll, StandardCopyOption.REPLACE_EXISTING);
                libraryResource.close();
                System.load(tempDll.toString());
            } catch (IOException ex) {
                throw new UncheckedIOException(ex);
            }
        }
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
            throw new IllegalArgumentException(
                    "Hex string has an odd length, which means it's incomplete.");
        }

        byte[] data = new byte[len / 2]; // Allocate space for the byte array
        for (int i = 0; i < len; i += 2) {
            // Parse each hex pair as a byte
            data[i / 2] =
                    (byte) ((Character.digit(s.charAt(i), 16) << 4) + Character.digit(s.charAt(i + 1), 16));
        }
        return data;
    }

    public static void main(String[] args) {
        loadNativeLibrary();

        final int blobSizeInBytes = 4096 * 32;
        byte[] byteArray = new byte[blobSizeInBytes];

        java.util.Arrays.fill(byteArray, (byte) 0);

        long prover_context_ptr = LibPeerDASKZG.proverContextNew();
        byte[] res = LibPeerDASKZG.computeCells(prover_context_ptr, byteArray);
        byte[] res2 = LibPeerDASKZG.computeCellsAndKZGProofs(prover_context_ptr, byteArray);
        byte[] res3 = LibPeerDASKZG.blobToKZGCommitment(prover_context_ptr, byteArray);
        // System.out.println("hello there " + Arrays.toString(res));
        System.out.println("hello there ");
    }
}
