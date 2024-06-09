using PeerDAS.Native;
using static PeerDAS.Native.NativeMethods;
using System.Runtime.InteropServices;

namespace PeerDASKZG;

public sealed unsafe partial class PeerDASKZG : IDisposable
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

    private PeerDASContext* _context;

    public PeerDASKZG()
    {
        _context = peerdas_context_new();
    }

    public PeerDASKZG(ContextSetting setting = ContextSetting.Both)
    {
        _context = peerdas_context_new_with_setting((CContextSetting)setting);
    }

    public void Dispose()
    {
        if (_context != null)
        {
            peerdas_context_free(_context);
            _context = null;
        }
    }

    public unsafe byte[] BlobToKzgCommitment(byte[] blob)
    {
        byte[] commitment = new byte[BytesPerCommitment];

        fixed (byte* blobPtr = blob)
        fixed (byte* commitmentPtr = commitment)
        {
            CResult result = blob_to_kzg_commitment(_context, Convert.ToUInt64(blob.Length), blobPtr, commitmentPtr);

            ThrowOnError(result);
        }

        return commitment;
    }

    public unsafe byte[][] ComputeCells(byte[] blob)
    {
        byte[] cells = new byte[BytesForAllCells];

        fixed (byte* blobPtr = blob)
        fixed (byte* cellsPtr = cells)
        {
            CResult result = compute_cells(_context, Convert.ToUInt64(blob.Length), blobPtr, cellsPtr);
            ThrowOnError(result);
        }
        return DeflattenArray(cells, BytesPerCell);
    }

    public unsafe (byte[][], byte[][]) ComputeCellsAndKZGProofs(byte[] blob)
    {
        byte[] cells = new byte[BytesForAllCells];
        byte[] proofs = new byte[BytesForAllProofs];

        fixed (byte* blobPtr = blob)
        fixed (byte* cellsPtr = cells)
        fixed (byte* proofsPtr = proofs)
        {
            CResult result = compute_cells_and_kzg_proofs(_context, Convert.ToUInt64(blob.Length), blobPtr, cellsPtr, proofsPtr);
            ThrowOnError(result);
        }
        return (DeflattenArray(cells, BytesPerCell), DeflattenArray(proofs, BytesPerCommitment));
    }

    public unsafe bool VerifyCellKZGProof(byte[] cell, byte[] commitment, ulong cellId, byte[] proof)
    {
        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte* cellPtr = cell)
        fixed (byte* commitmentPtr = commitment)
        fixed (byte* proofPtr = proof)
        {
            CResult result = verify_cell_kzg_proof(_context, Convert.ToUInt64(cell.Length), cellPtr, Convert.ToUInt64(commitment.Length), commitmentPtr, cellId, Convert.ToUInt64(proof.Length), proofPtr, verifiedPtr);
            ThrowOnError(result);
        }

        return verified;
    }

    // TODO: switch argument order to match specs closer on all functions
    public bool VerifyCellKZGProofBatch(byte[][] rowCommitments, ulong[] rowIndices, ulong[] columnIndices, byte[][] cells, byte[][] proofs)
    {

        byte[] rowCommitmentsFlattened = FlattenArray(rowCommitments);
        byte[] cellsFlattened = FlattenArray(cells);
        byte[] proofsFlattened = FlattenArray(proofs);

        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte* rowCommitmentsPtr = rowCommitmentsFlattened)
        fixed (ulong* rowIndicesPtr = rowIndices)
        fixed (ulong* columnIndicesPtr = columnIndices)
        fixed (byte* cellsPtr = cellsFlattened)
        fixed (byte* proofsPtr = proofsFlattened)
        {
            CResult result = verify_cell_kzg_proof_batch(_context, Convert.ToUInt64(rowCommitmentsFlattened.Length), rowCommitmentsPtr, Convert.ToUInt64(rowIndices.Length), rowIndicesPtr, Convert.ToUInt64(columnIndices.Length), columnIndicesPtr, Convert.ToUInt64(cellsFlattened.Length), cellsPtr, Convert.ToUInt64(proofsFlattened.Length), proofsPtr, verifiedPtr);
            ThrowOnError(result);
        }
        return verified;
    }

    public byte[][] RecoverAllCells(ulong[] cellIds, byte[][] cells)
    {
        byte[] cellsFlattened = FlattenArray(cells);

        byte[] recoveredCells = new byte[BytesForAllCells];
        fixed (byte* cellsPtr = cellsFlattened)
        fixed (ulong* cellIdsPtr = cellIds)
        fixed (byte* recoveredCellsPtr = recoveredCells)
        {
            CResult result = recover_all_cells(_context, Convert.ToUInt64(cellsFlattened.Length), cellsPtr, Convert.ToUInt64(cellIds.Length), cellIdsPtr, recoveredCellsPtr);
            ThrowOnError(result);
        }

        return DeflattenArray(recoveredCells, BytesPerCell);
    }

    private static void ThrowOnError(CResult result)
    {
        switch (result.status)
        {
            case CResultStatus.Err:
                string? errorMessage = Marshal.PtrToStringAnsi((IntPtr)result.error_msg);

                if (errorMessage != null)
                {
                    // Free the error message that we allocated on the rust side
                    free_error_message(result.error_msg);
                    throw new ArgumentException($"an error occurred from the bindings: {errorMessage}");
                }
                else
                {
                    // This branch should not be hit, ie when the native library returns
                    // and error, the error_message should always be set.
                    throw new ArgumentException("an error occurred from the bindings: unknown error");
                }
            case CResultStatus.Ok:
                return;
            default:
                throw new ApplicationException("PeerDASKZG returned an unexpected result variant");
        }
    }

    private static byte[] FlattenArray(byte[][] jaggedArray)
    {
        int totalLength = 0;

        // Calculate the total length of the flattened array
        foreach (byte[] subArray in jaggedArray)
        {
            totalLength += subArray.Length;
        }

        // Create a new array to hold the flattened result
        byte[] flattenedArray = new byte[totalLength];

        int currentIndex = 0;

        // Copy elements from the jagged array to the flattened array
        foreach (byte[] subArray in jaggedArray)
        {
            Array.Copy(subArray, 0, flattenedArray, currentIndex, subArray.Length);
            currentIndex += subArray.Length;
        }

        return flattenedArray;
    }

    private static byte[][] DeflattenArray(byte[] flattenedArray, int length)
    {
        int numArrays = flattenedArray.Length / length;
        byte[][] jaggedArray = new byte[numArrays][];

        for (int i = 0; i < numArrays; i++)
        {
            jaggedArray[i] = new byte[length];
            Array.Copy(flattenedArray, i * length, jaggedArray[i], 0, length);
        }

        return jaggedArray;
    }

    public enum ContextSetting
    {
        ProvingOnly,
        VerifyOnly,
        Both
    }
}