
namespace PeerDASKZG;

public static unsafe partial class PeerDASKZG
{
    // These constants are copied from the c-kzg csharp bindings file.
    // TODO: This is not ideal, since we want the Rust code to be the single source of truth
    // TODO: Generally, we want the c code to return functions that define these constants
    // TODO: At the very least, we can have code to sanity check these constants
    private const int BytesPerFieldElement = 32;
    private const int FieldElementsPerBlob = 4096;
    private const int FieldElementsPerExtBlob = 2 * FieldElementsPerBlob;
    public const int BytesPerBlob = BytesPerFieldElement * FieldElementsPerBlob;
    public const int BytesForAllCells = BytesPerFieldElement * FieldElementsPerExtBlob;
    public const int BytesForAllProofs = CellsPerExtBlob * BytesPerProof;
    public const int BytesPerCommitment = 48;
    public const int BytesPerProof = 48;
    private const int FieldElementsPerCell = 64;
    public const int BytesPerCell = BytesPerFieldElement * FieldElementsPerCell;
    private const int CellsPerExtBlob = FieldElementsPerExtBlob / FieldElementsPerCell;

    public static unsafe byte[] BlobToKzgCommitment(IntPtr proverContextPtr, byte[] blob)
    {
        byte[] commitment = new byte[BytesPerCommitment];

        fixed (byte* blobPtr = blob)
        fixed (byte* commitmentPtr = commitment)
        {

            CResult result = blob_to_kzg_commitment(IntPtrToPeerDASContext(proverContextPtr), Convert.ToUInt64(blob.Length), blobPtr, commitmentPtr);
            ThrowOnError(result);

        }

        return commitment;
    }

    public static unsafe byte[] ComputeCells(IntPtr proverContextPtr, byte[] blob)
    {
        byte[] cells = new byte[BytesForAllCells];

        fixed (byte* blobPtr = blob)
        fixed (byte* cellsPtr = cells)
        {
            CResult result = compute_cells(IntPtrToPeerDASContext(proverContextPtr), Convert.ToUInt64(blob.Length), blobPtr, cellsPtr);
            ThrowOnError(result);
        }
        return cells;
    }

    public static unsafe (byte[], byte[]) ComputeCellsAndKZGProofs(IntPtr proverContextPtr, byte[] blob)
    {
        byte[] cells = new byte[BytesForAllCells];
        byte[] proofs = new byte[BytesForAllProofs];

        fixed (byte* blobPtr = blob)
        fixed (byte* cellsPtr = cells)
        fixed (byte* proofsPtr = proofs)
        {

            CResult result = compute_cells_and_kzg_proofs(IntPtrToPeerDASContext(proverContextPtr), Convert.ToUInt64(blob.Length), blobPtr, cellsPtr, proofsPtr);
            ThrowOnError(result);
        }
        return (cells, proofs);
    }

    public static unsafe IntPtr PeerDASContextNew()
    {
        return PeerDASContextToIntPtr(peerdas_context_new());
    }

    public static unsafe void PeerDASContextFree(IntPtr peerDASContextPtr)
    {
        peerdas_context_free(IntPtrToPeerDASContext(peerDASContextPtr));
    }

    public static unsafe bool VerifyCellKZGProof(IntPtr verifierContextPtr, byte[] cell, byte[] commitment, ulong cellId, byte[] proof)
    {
        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte* cellPtr = cell)
        fixed (byte* commitmentPtr = commitment)
        fixed (byte* proofPtr = proof)
        {
            CResult result = verify_cell_kzg_proof(IntPtrToPeerDASContext(verifierContextPtr), Convert.ToUInt64(cell.Length), cellPtr, Convert.ToUInt64(commitment.Length), commitmentPtr, cellId, Convert.ToUInt64(proof.Length), proofPtr, verifiedPtr);
            ThrowOnError(result);
        }

        return verified;
    }

    // TODO: switch argument order to match specs closer on all functions
    public static bool VerifyCellKZGProofBatch(IntPtr verifierContextPtr, byte[] rowCommitments, ReadOnlySpan<ulong> rowIndices, ulong[] columnIndices, byte[] cells, byte[] proofs)
    {
        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte* rowCommitmentsPtr = rowCommitments)
        fixed (ulong* rowIndicesPtr = rowIndices)
        fixed (ulong* columnIndicesPtr = columnIndices)
        fixed (byte* cellsPtr = cells)
        fixed (byte* proofsPtr = proofs)
        {
            CResult result = verify_cell_kzg_proof_batch(IntPtrToPeerDASContext(verifierContextPtr), Convert.ToUInt64(rowCommitments.Length), rowCommitmentsPtr, Convert.ToUInt64(rowIndices.Length), rowIndicesPtr, Convert.ToUInt64(columnIndices.Length), columnIndicesPtr, Convert.ToUInt64(cells.Length), cellsPtr, Convert.ToUInt64(proofs.Length), proofsPtr, verifiedPtr);
            ThrowOnError(result);
        }
        return verified;
    }

    public static byte[] RecoverAllCells(IntPtr verifierContextPtr, ulong[] cellIds, byte[] cells)
    {
        byte[] recoveredCells = new byte[BytesForAllCells];
        fixed (byte* cellsPtr = cells)
        fixed (ulong* cellIdsPtr = cellIds)
        fixed (byte* recoveredCellsPtr = recoveredCells)
        {
            CResult result = recover_all_cells(IntPtrToPeerDASContext(verifierContextPtr), Convert.ToUInt64(cells.Length), cellsPtr, Convert.ToUInt64(cellIds.Length), cellIdsPtr, recoveredCellsPtr);
            ThrowOnError(result);
        }

        return recoveredCells;
    }

    // Below error handling was copied from ckzg
    private static void ThrowOnError(CResult result)
    {
        switch (result)
        {
            case CResult.Err: throw new ArgumentException("an error occurred from the bindings");
            case CResult.Ok:
                return;
            default:
                throw new ApplicationException("PeerDASKZG returned an unexpected result variant");
        }
    }

    // TODO: Ideally, these methods are not needed and we use PeerDASContext* directly
    // instead of IntPtr.
    // The csharp code, in particular tests, seems to not be able to handle pointers directly
    public static IntPtr PeerDASContextToIntPtr(PeerDASContext* context)
    {
        return new IntPtr(context);
    }
    public static PeerDASContext* IntPtrToPeerDASContext(IntPtr ptr)
    {
        return (PeerDASContext*)ptr.ToPointer();
    }
}