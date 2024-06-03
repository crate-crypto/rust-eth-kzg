namespace PeerDASKZG;

public static partial class PeerDASKZG
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

    public static IntPtr ProverContextNew()
    {
        return InternalProverContextNew();
    }

    public static void ProverContextFree(IntPtr proverContextPtr)
    {
        InternalProverContextFree(proverContextPtr);
    }

    public static byte[] BlobToKzgCommitment(IntPtr proverContextPtr, byte[] blob)
    {
        byte[] commitment = new byte[BytesPerCommitment];

        Result result = InternalBlobToKzgCommitment(proverContextPtr, Convert.ToUInt64(blob.Length), blob, commitment);
        ThrowOnError(result);

        return commitment;
    }

    public static byte[] ComputeCells(IntPtr proverContextPtr, byte[] blob)
    {
        byte[] cells = new byte[BytesForAllCells];

        Result result = InternalComputeCells(proverContextPtr, Convert.ToUInt64(blob.Length), blob, cells);
        ThrowOnError(result);
        return cells;
    }

    public static (byte[], byte[]) ComputeCellsAndKZGProofs(IntPtr proverContextPtr, byte[] blob)
    {
        byte[] cells = new byte[BytesForAllCells];
        byte[] proofs = new byte[BytesForAllProofs];

        Result result = InternalComputeCellsAndKzgProofs(proverContextPtr, Convert.ToUInt64(blob.Length), blob, cells, proofs);
        ThrowOnError(result);

        return (cells, proofs);
    }

    public static IntPtr VerifierContextNew()
    {
        return InternalVerifierContextNew();
    }

    public static void VerifierContextFree(IntPtr verifierContextPtr)
    {
        InternalVerifierContextFree(verifierContextPtr);
    }

    public static bool VerifyCellKZGProof(IntPtr verifierContextPtr, byte[] cell, byte[] commitment, ulong cellId, byte[] proof)
    {
        Result result = InternalVerifyCellKZGProof(verifierContextPtr, Convert.ToUInt64(cell.Length), cell, Convert.ToUInt64(commitment.Length), commitment, cellId, Convert.ToUInt64(proof.Length), proof, out var verified);
        ThrowOnError(result);

        return verified;
    }


    // TODO: switch argument order to match specs closer on all functions
    public static bool VerifyCellKZGProofBatch(IntPtr verifierContextPtr, byte[] rowCommitments, ulong[] rowIndices, ulong[] columnIndices, byte[] cells, byte[] proofs)
    {
        Result result = InternalVerifyCellKZGProofBatch(verifierContextPtr, Convert.ToUInt64(rowCommitments.Length), rowCommitments, Convert.ToUInt64(rowIndices.Length), rowIndices, Convert.ToUInt64(columnIndices.Length), columnIndices, Convert.ToUInt64(cells.Length), cells, Convert.ToUInt64(proofs.Length), proofs, out var verified);
        ThrowOnError(result);

        return verified;
    }

    public static byte[] RecoverAllCells(IntPtr verifierContextPtr, ulong[] cellIds, byte[] cells)
    {
        byte[] recoveredCells = new byte[BytesForAllCells];

        Result result = InternalRecoverAllCells(verifierContextPtr, Convert.ToUInt64(cells.Length), cells, Convert.ToUInt64(cellIds.Length), cellIds, recoveredCells);
        ThrowOnError(result);

        return recoveredCells;
    }

    // Below error handling was copied from ckzg
    private static void ThrowOnError(Result result)
    {
        switch (result)
        {
            case Result.Err: throw new ArgumentException("an error occurred from the bindings");
            case Result.Ok:
                return;
            default:
                throw new ApplicationException("PeerDASKZG returned an unexpected result variant");
        }
    }
}