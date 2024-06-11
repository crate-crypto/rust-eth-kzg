using PeerDAS.Native;
using static PeerDAS.Native.NativeMethods;
using System.Runtime.InteropServices;

namespace PeerDASKZG;

public sealed unsafe class PeerDASKZG : IDisposable
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
    public const int NumColumns = CellsPerExtBlob;
    private const int CellsPerExtBlob = FieldElementsPerExtBlob / FieldElementsPerCell;

    private PeerDASContext* _context;

    public PeerDASKZG()
    {
        _context = peerdas_context_new();
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

        (byte[][] cells, _) = ComputeCellsAndKZGProofs(blob);
        return cells;
    }

    public unsafe (byte[][], byte[][]) ComputeCellsAndKZGProofs(byte[] blob)
    {
        int numProofs = CellsPerExtBlob;
        int numCells = CellsPerExtBlob;

        byte[][] outCells = InitializeJaggedArray(numCells, BytesPerCell);
        byte[][] outProofs = InitializeJaggedArray(numCells, BytesPerProof);

        // Allocate an array of pointers for cells and proofs
        byte*[] outCellsPtrs = new byte*[numCells];
        byte*[] outProofsPtrs = new byte*[numProofs];

        fixed (byte* blobPtr = blob)
        fixed (byte** outCellsPtrPtr = outCellsPtrs)
        fixed (byte** outProofsPtrPtr = outProofsPtrs)
        {

            // Get the pointer for each cell
            for (int i = 0; i < numCells; i++)
            {
                fixed (byte* cellPtr = outCells[i])
                {
                    outCellsPtrPtr[i] = cellPtr;
                }
            }

            // Get the pointer for each proof
            for (int i = 0; i < numCells; i++)
            {
                fixed (byte* proofPtr = outProofs[i])
                {
                    outProofsPtrPtr[i] = proofPtr;
                }
            }

            CResult result = compute_cells_and_kzg_proofs(_context, Convert.ToUInt64(blob.Length), blobPtr, outCellsPtrPtr, outProofsPtrPtr);
            ThrowOnError(result);
        }
        return (outCells, outProofs);
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

        // The native code assumes that all double vectors have the same length.
        for (int i = 0; i < cells.Length; i++)
        {
            if (cells[i].Length != BytesPerCell)
            {
                throw new ArgumentException($"cell at index {i} has an invalid length");
            }
        }

        for (int i = 0; i < proofs.Length; i++)
        {
            if (proofs[i].Length != BytesPerCommitment)
            {
                throw new ArgumentException($"proof at index {i} has an invalid length");
            }
        }

        for (int i = 0; i < rowCommitments.Length; i++)
        {
            if (rowCommitments[i].Length != BytesPerCommitment)
            {
                throw new ArgumentException($"commitments at index {i} has an invalid length");
            }
        }

        int numCells = cells.Length;
        int numProofs = proofs.Length;
        int numRowCommitments = rowCommitments.Length;

        byte*[] commPtrs = new byte*[numRowCommitments];
        byte*[] cellsPtrs = new byte*[numCells];
        byte*[] proofsPtrs = new byte*[numProofs];

        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte** commitmentPtrPtr = commPtrs)
        fixed (byte** cellsPtrPtr = cellsPtrs)
        fixed (byte** proofsPtrPtr = proofsPtrs)
        fixed (ulong* rowIndicesPtr = rowIndices)
        fixed (ulong* columnIndicesPtr = columnIndices)
        {
            // Get the pointer for each cell
            for (int i = 0; i < numCells; i++)
            {
                fixed (byte* cellPtr = cells[i])
                {
                    cellsPtrPtr[i] = cellPtr;
                }
            }

            // Get the pointer for each commitment
            for (int i = 0; i < numRowCommitments; i++)
            {
                fixed (byte* commPtr = rowCommitments[i])
                {
                    commitmentPtrPtr[i] = commPtr;
                }
            }

            // Get the pointer for each commitment
            for (int i = 0; i < numProofs; i++)
            {
                fixed (byte* proofPtr = proofs[i])
                {
                    proofsPtrPtr[i] = proofPtr;
                }
            }

            CResult result = verify_cell_kzg_proof_batch(_context, Convert.ToUInt64(rowCommitments.Length), commitmentPtrPtr, Convert.ToUInt64(rowIndices.Length), rowIndicesPtr, Convert.ToUInt64(columnIndices.Length), columnIndicesPtr, Convert.ToUInt64(cells.Length), cellsPtrPtr, Convert.ToUInt64(proofs.Length), proofsPtrPtr, verifiedPtr);
            ThrowOnError(result);
        }
        return verified;
    }

    public byte[][] RecoverAllCells(ulong[] cellIds, byte[][] cells)
    {
        (byte[][] recoveredCells, _) = RecoverCellsAndKZGProofs(cellIds, cells);
        return recoveredCells;
    }

    public (byte[][], byte[][]) RecoverCellsAndKZGProofs(ulong[] cellIds, byte[][] cells)
    {

        // The native code assumes that all cells have the same length.
        for (int i = 0; i < cells.Length; i++)
        {
            if (cells[i].Length != BytesPerCell)
            {
                throw new ArgumentException($"cell at index {i} has an invalid length");
            }
        }

        int numProofs = CellsPerExtBlob;
        int numOutCells = CellsPerExtBlob;
        int numInputCells = cells.Length;

        byte[][] outCells = InitializeJaggedArray(numOutCells, BytesPerCell);
        byte[][] outProofs = InitializeJaggedArray(numProofs, BytesPerProof);

        // Allocate an array of pointers for inputCells, outputCells and proofs
        byte*[] inputCellsPtrs = new byte*[numInputCells];
        byte*[] outCellsPtrs = new byte*[numOutCells];
        byte*[] outProofsPtrs = new byte*[numProofs];

        fixed (ulong* cellIdsPtr = cellIds)
        fixed (byte** inputCellsPtrPtr = inputCellsPtrs)
        fixed (byte** outCellsPtrPtr = outCellsPtrs)
        fixed (byte** outProofsPtrPtr = outProofsPtrs)
        {
            // Get the pointer for each input cell
            for (int i = 0; i < numInputCells; i++)
            {
                fixed (byte* cellPtr = cells[i])
                {
                    inputCellsPtrPtr[i] = cellPtr;
                }
            }

            // Get the pointer for each output cell
            for (int i = 0; i < numOutCells; i++)
            {
                fixed (byte* cellPtr = outCells[i])
                {
                    outCellsPtrPtr[i] = cellPtr;
                }
            }

            // Get the pointer for each proof
            for (int i = 0; i < numProofs; i++)
            {
                fixed (byte* proofPtr = outProofs[i])
                {
                    outProofsPtrPtr[i] = proofPtr;
                }
            }

            CResult result = recover_cells_and_proofs(_context, Convert.ToUInt64(numInputCells), inputCellsPtrPtr, Convert.ToUInt64(cellIds.Length), cellIdsPtr, outCellsPtrPtr, outProofsPtrPtr);
            ThrowOnError(result);
        }

        return (outCells, outProofs);
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

    static byte[][] InitializeJaggedArray(int outerLen, int innerLen)
    {
        // Create and initialize the jagged array
        byte[][] jaggedArray = new byte[outerLen][];
        for (int i = 0; i < outerLen; i++)
        {
            jaggedArray[i] = new byte[innerLen];
        }
        return jaggedArray;
    }
}