package ethereum.cryptography;
import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

public class LibEthKZG implements AutoCloseable{
    // TODO: Add equality tests
    //
    /** The number of bytes in a KZG commitment. */
    public static final int BYTES_PER_COMMITMENT = 48;
    /** The number of bytes in a KZG proof. */
    public static final int BYTES_PER_PROOF = 48;
    /** The number of bytes in a BLS scalar field element. */
    public static final int BYTES_PER_FIELD_ELEMENT = 32;
    /** The number of bytes in a blob. */
    public static final int BYTES_PER_BLOB = 131_072;
    /** The number of columns in an extended blob. */
    public static final int MAX_NUM_COLUMNS = 128;
    /** The number of bytes in a single cell. */
    public static final int BYTES_PER_CELL = 2048;

    private long contextPtr;

    private static volatile boolean libraryLoaded = false;
    private static final Object libraryLock = new Object();


    public LibEthKZG() {
        ensureLibraryLoaded();
        this.contextPtr = DASContextNew();
    }

    private static void ensureLibraryLoaded() {
        if (!libraryLoaded) {
            synchronized (libraryLock) {
                if (!libraryLoaded) {
                    loadNativeLibrary();
                    libraryLoaded = true;
                }
            }
        }
    }

    // TODO: Finalization was deprecated, we should find a method that does
    // TODO: not require a lot of code. Possibly separate the wrapper from the
    // TODO: bindings code too.
    @Override
    public void close() {
        destroy();
    }

    public void destroy() {
        if (contextPtr != 0) {
            DASContextDestroy(contextPtr);
            contextPtr = 0;
        }
    }

    // TODO: we could move this to the rust code but it requires a double pointer
    // TODO: add this code to all bindings
    private void checkContextHasNotBeenFreed() {
        if (contextPtr == 0) {
            throw new IllegalStateException("KZG context context has been destroyed");
        }
    }

    public byte[] blobToKZGCommitment(byte[] blob) {
        checkContextHasNotBeenFreed();
        return blobToKZGCommitment(contextPtr, blob);
    }

    public CellsAndProofs computeCellsAndKZGProofs(byte[] blob) {
        checkContextHasNotBeenFreed();
        CellsAndProofs cellsAndProofs = computeCellsAndKZGProofs(contextPtr, blob);
        return cellsAndProofs;
    }

    public boolean verifyCellKZGProofBatch(byte[][] commitmentsArr,  long[] cellIndices, byte[][] cellsArr,
            byte[][] proofsArr) {
                checkContextHasNotBeenFreed();
        return verifyCellKZGProofBatch(contextPtr, commitmentsArr, cellIndices, cellsArr, proofsArr);
    }

    public CellsAndProofs recoverCellsAndProofs(long[] cellIDs, byte[][] cellsArr) {
        checkContextHasNotBeenFreed();
        return recoverCellsAndProof(contextPtr, cellIDs, cellsArr);
    }

    /*
     * Below are the native methods and the code related to loading the native
     * library
     */

    private static native long DASContextNew();

    private static native void DASContextDestroy(long ctx_ptr);

    private static native CellsAndProofs computeCellsAndKZGProofs(long context_ptr, byte[] blob);

    private static native byte[] blobToKZGCommitment(long context_ptr, byte[] blob);

    private static native boolean verifyCellKZGProofBatch(
            long context_ptr, byte[][] commitments, long[] cellIndices, byte[][] cells, byte[][] proofs);

    private static native CellsAndProofs recoverCellsAndProof(long context_ptr, long[] cellIDs, byte[][] cells);

    private static final String LIBRARY_NAME = "java_eth_kzg";
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
    private static void loadNativeLibrary() {

        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = getNormalizedArchitecture();
        String libraryResourcePath = null;
        System.out.println("name: " + osName + " arch:" + osArch + " platform: " + PLATFORM_NATIVE_LIBRARY_NAME);
        if (osName.contains("win")) {
            if (osArch.contains("x86_64")) {
                libraryResourcePath = "/x86_64-pc-windows-gnu/" + PLATFORM_NATIVE_LIBRARY_NAME;
            } else if (osArch.contains("x86")) {
                // We do not support 32 bit windows
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

        InputStream libraryResource = LibEthKZG.class.getResourceAsStream(libraryResourcePath);

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
