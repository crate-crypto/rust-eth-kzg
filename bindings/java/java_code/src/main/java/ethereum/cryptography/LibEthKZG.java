package ethereum.cryptography;
import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

/**
 * This class handles the loading of native libraries and provides methods for
 * Ethereum's DAS related cryptography.
 */
public class LibEthKZG implements AutoCloseable{
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

    /**
     * Constructs a LibEthKZG instance with default parameters.
     * Uses pre-computation and a single thread.
     */
    public LibEthKZG() {
        ensureLibraryLoaded();
        boolean usePrecomp = true;
        this.contextPtr = DASContextNew(usePrecomp);
    }

    /**
     * Constructs a LibEthKZG instance with specified parameters.
     *
     * @param usePrecomp Whether to use pre-computation.
     */
    public LibEthKZG(boolean usePrecomp) {
        ensureLibraryLoaded();
        this.contextPtr = DASContextNew(usePrecomp);
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

    @Override
    public void close() {
        destroy();
    }

    /**
     * Destroys the KZG context and frees associated resources.
     * This method should be called when the LibEthKZG instance is no longer needed.
     */
    public void destroy() {
        if (contextPtr != 0) {
            DASContextDestroy(contextPtr);
            contextPtr = 0;
        }
    }

    private void checkContextHasNotBeenFreed() {
        if (contextPtr == 0) {
            throw new IllegalStateException("KZG context context has been destroyed");
        }
    }

    /**
     * Computes the KZG commitment for a given blob.
     *
     * @param blob The input blob.
     * @return The KZG commitment as a byte array.
     */
    public byte[] blobToKZGCommitment(byte[] blob) {
        checkContextHasNotBeenFreed();
        return blobToKZGCommitment(contextPtr, blob);
    }

    /**
     * Computes cells and KZG proofs for a given blob.
     *
     * @param blob The input blob.
     * @return CellsAndProofs object containing the computed cells and proofs.
     */
    public CellsAndProofs computeCellsAndKZGProofs(byte[] blob) {
        checkContextHasNotBeenFreed();
        CellsAndProofs cellsAndProofs = computeCellsAndKZGProofs(contextPtr, blob);
        return cellsAndProofs;
    }

    /**
     * Computes cells for a given blob.
     *
     * @param blob The input blob.
     * @return Cells object containing the computed cells.
     */
    public Cells computeCells(byte[] blob) {
        checkContextHasNotBeenFreed();
        Cells cells = computeCells(contextPtr, blob);
        return cells;
    }

    /**
     * Verifies a batch of cell KZG proofs.
     *
     * @param commitmentsArr Array of commitments.
     * @param cellIndices    Array of cell indices.
     * @param cellsArr       Array of cells.
     * @param proofsArr      Array of proofs.
     * @return true if the batch verification succeeds, false otherwise.
     */
    public boolean verifyCellKZGProofBatch(byte[][] commitmentsArr,  long[] cellIndices, byte[][] cellsArr,
            byte[][] proofsArr) {
                checkContextHasNotBeenFreed();
        return verifyCellKZGProofBatch(contextPtr, commitmentsArr, cellIndices, cellsArr, proofsArr);
    }

    /**
     * Recovers cells and computes KZG proofs from given cell IDs and cells.
     *
     * @param cellIDs  Array of cell IDs.
     * @param cellsArr Array of cells.
     * @return CellsAndProofs object containing the recovered cells and proofs.
     */
    public CellsAndProofs recoverCellsAndKZGProofs(long[] cellIDs, byte[][] cellsArr) {
        checkContextHasNotBeenFreed();
        return recoverCellsAndKZGProofs(contextPtr, cellIDs, cellsArr);
    }

    /**
     * Computes the KZG proof given a blob and a point.
     *
     * @param blob The input blob.
     * @param z    The evaluation point.
     * @return A two-element array where the first element is the KZG proof and the second is the evaluation result.
     */
    public byte[][] computeKzgProof(byte[] blob, byte[] z) {
        checkContextHasNotBeenFreed();
        return computeKzgProof(contextPtr, blob, z);
    }

    /**
     * Computes the KZG proof given a blob and its corresponding commitment.
     *
     * @param blob       The input blob.
     * @param commitment The KZG commitment.
     * @return The KZG proof as a byte array.
     */
    public byte[] computeBlobKzgProof(byte[] blob, byte[] commitment) {
        checkContextHasNotBeenFreed();
        return computeBlobKzgProof(contextPtr, blob, commitment);
    }

    /**
     * Verifies the KZG proof to the commitment.
     *
     * @param commitment The KZG commitment.
     * @param z          The evaluation point.
     * @param y          The evaluation result.
     * @param proof      The KZG proof.
     * @return true if the proof is valid, false otherwise.
     */
    public boolean verifyKzgProof(byte[] commitment, byte[] z, byte[] y, byte[] proof) {
        checkContextHasNotBeenFreed();
        return verifyKzgProof(contextPtr, commitment, z, y, proof);
    }

    /**
     * Verifies the KZG proof to the commitment of a blob.
     *
     * @param blob       The input blob.
     * @param commitment The KZG commitment.
     * @param proof      The KZG proof.
     * @return true if the proof is valid, false otherwise.
     */
    public boolean verifyBlobKzgProof(byte[] blob, byte[] commitment, byte[] proof) {
        checkContextHasNotBeenFreed();
        return verifyBlobKzgProof(contextPtr, blob, commitment, proof);
    }

    /**
     * Verifies a batch of KZG proofs to the commitments of blobs.
     *
     * @param blobs       Array of blobs.
     * @param commitments Array of commitments.
     * @param proofs      Array of proofs.
     * @return true if all proofs are valid, false otherwise.
     */
    public boolean verifyBlobKzgProofBatch(byte[][] blobs, byte[][] commitments, byte[][] proofs) {
        checkContextHasNotBeenFreed();
        return verifyBlobKzgProofBatch(contextPtr, blobs, commitments, proofs);
    }

    /*
     * Below are the native methods and the code related to loading the native
     * library
     */

    private static native long DASContextNew(boolean usePrecomp);

    private static native void DASContextDestroy(long ctx_ptr);

    private static native CellsAndProofs computeCellsAndKZGProofs(long context_ptr, byte[] blob);
    
    private static native Cells computeCells(long context_ptr, byte[] blob);

    private static native byte[] blobToKZGCommitment(long context_ptr, byte[] blob);

    private static native boolean verifyCellKZGProofBatch(
            long context_ptr, byte[][] commitments, long[] cellIndices, byte[][] cells, byte[][] proofs);

    private static native CellsAndProofs recoverCellsAndKZGProofs(long context_ptr, long[] cellIDs, byte[][] cells);

    private static native byte[][] computeKzgProof(long context_ptr, byte[] blob, byte[] z);

    private static native byte[] computeBlobKzgProof(long context_ptr, byte[] blob, byte[] commitment);

    private static native boolean verifyKzgProof(long context_ptr, byte[] commitment, byte[] z, byte[] y, byte[] proof);

    private static native boolean verifyBlobKzgProof(long context_ptr, byte[] blob, byte[] commitment, byte[] proof);

    private static native boolean verifyBlobKzgProofBatch(long context_ptr, byte[][] blobs, byte[][] commitments, byte[][] proofs);

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
                // We currently do not support arm on windows
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
