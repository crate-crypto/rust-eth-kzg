package ethereum.cryptography;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

public class LibPeerDASKZG {
    public static native long proverContextNew();

    public static native void proverContextDestroy(long prover_ctx_ptr);

    public static native byte[] computeCells(long prover_context_ptr, byte[] blob);

    public static native byte[] computeCellsAndKZGProofs(long prover_context_ptr, byte[] blob);

    public static native byte[] blobToKZGCommitment(long prover_context_ptr, byte[] blob);

    public static native long verifierContextNew();

    public static native void verifierContextDestroy(long verifier_context_ptr);

    public static native boolean verifyCellKZGProof(
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
}
