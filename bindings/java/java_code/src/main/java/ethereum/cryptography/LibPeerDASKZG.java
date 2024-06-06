package ethereum.cryptography;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

public class LibPeerDASKZG {
    // These constants were taken from c-kzg
    //
    // TODO: We should remove the ones which are not needed or only needed for tests
    /** The number of bytes in a g1 point. */
    public static final int BYTES_PER_G1 = 48;
    /** The number of bytes in a g2 point. */
    public static final int BYTES_PER_G2 = 96;
    /** The number of bytes in a KZG commitment. */
    public static final int BYTES_PER_COMMITMENT = 48;
    /** The number of bytes in a KZG proof. */
    public static final int BYTES_PER_PROOF = 48;
    /** The number of bytes in a BLS scalar field element. */
    public static final int BYTES_PER_FIELD_ELEMENT = 32;
    /** The number of bits in a BLS scalar field element. */
    public static final int BITS_PER_FIELD_ELEMENT = 255;
    /** The number of field elements in a blob. */
    public static final int FIELD_ELEMENTS_PER_BLOB = 4096;
    /** The number of field elements in a blob. */
    public static final int BYTES_PER_BLOB = FIELD_ELEMENTS_PER_BLOB * BYTES_PER_FIELD_ELEMENT;
    /** The number of field elements in an extended blob. */
    public static final int FIELD_ELEMENTS_PER_EXT_BLOB = FIELD_ELEMENTS_PER_BLOB * 2;
    /** The number of field elements in a cell. */
    public static final int FIELD_ELEMENTS_PER_CELL = 64;
    /** The number of cells in an extended blob. */
    public static final int CELLS_PER_EXT_BLOB = FIELD_ELEMENTS_PER_EXT_BLOB / FIELD_ELEMENTS_PER_CELL;
    /** The number of bytes in a single cell. */
    public static final int BYTES_PER_CELL = BYTES_PER_FIELD_ELEMENT * FIELD_ELEMENTS_PER_CELL;

    public static native long peerDASContextNew();

    public static native void peerDASContextDestroy(long ctx_ptr);

    public static native byte[] computeCells(long context_ptr, byte[] blob);

    public static native CellsAndProofs computeCellsAndKZGProofs(long context_ptr, byte[] blob);

    public static native byte[] blobToKZGCommitment(long context_ptr, byte[] blob);

    public static native boolean verifyCellKZGProof(
            long context_ptr, byte[] commitment, long cell_id, byte[] cell, byte[] proof);

    private static final String LIBRARY_NAME = "java_peerdas_kzg";
    private static final String PLATFORM_NATIVE_LIBRARY_NAME = System.mapLibraryName(LIBRARY_NAME);

    private static String getNormalizedArchitecture() {
        String osArch = System.getProperty("os.arch").toLowerCase();
        if (osArch.equals("x86_64") || osArch.equals("amd64")) {
            return "x86_64";
        } else if (osArch.equals("aarch64") || osArch.equals("arm64")) {
            return "aarch64";
        } else {
            return osArch;
        }
    }

    /** Loads the appropriate native library based on your platform. */
    // Copied from c-kzg
    public static void loadNativeLibrary() {

        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = getNormalizedArchitecture();
        String libraryResourcePath = null;

        if (osName.contains("win")) {
            if (osArch.contains("x86_64")) {
                libraryResourcePath = "/x86_64-pc-windows-gnu/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("x86")) {
                // TODO: Remove this and just don't support 32-bit Windows
                libraryResourcePath = "/i686-pc-windows-gnu/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("aarch64")) {
                // Current version of c-kzg does not support arm windows either
                // TODO: Rust has support for msvc with arm64, but it also has:
                // aarch64-pc-windows-gnullvm -- we probably want to stick to one
                // toolchain for now.
                // If we do switch to msvc, nethermind had an issue with windows server 2022
                // that we should check works with an msvc build.
                // libraryResourcePath = "/aarch64-pc-windows-gnullvm/" + PLATFORM_NATIVE_LIBRARY_NAME;

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
