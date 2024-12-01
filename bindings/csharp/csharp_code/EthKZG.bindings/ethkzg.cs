using EthKZG.Native;
using static EthKZG.Native.NativeMethods;
using System.Runtime.InteropServices;

namespace EthKZG;

public sealed unsafe class EthKZG : IDisposable
{
    // These constants are copied from the c-kzg csharp bindings file.
    //
    // The number of bytes in a KZG commitment.
    public const int BytesPerCommitment = 48;
    // The number of bytes in a KZG Proof
    public const int BytesPerProof = 48;
    // The number of bytes in a BLS scalar field element
    public const int BytesPerFieldElement = 32;
    // The number of bytes needed to represent a blob.
    public const int BytesPerBlob = 131_072;
    // The number of columns needed to represent an extended blob.
    public const int MaxNumColumns = 128;
    // This is the same as the MaxNumColumns, but this terminology is used
    // in the cryptography implementation, so we use it internally for readability.
    private const int CellsPerExtBlob = MaxNumColumns;
    // The number of bytes in a single cell.
    public const int BytesPerCell = 2048;

    private DASContext* _context;

    public EthKZG(bool usePrecomp = true, uint numThreads = 0)
    {
        _context = eth_kzg_das_context_new(usePrecomp, numThreads);
    }

    public void Dispose()
    {
        if (_context != null)
        {
            eth_kzg_das_context_free(_context);
            _context = null;
        }
    }

    public unsafe byte[] BlobToKzgCommitment(byte[] blob)
    {

        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length");
        }

        byte[] commitment = new byte[BytesPerCommitment];

        fixed (byte* blobPtr = blob)
        fixed (byte* commitmentPtr = commitment)
        {
            CResult result = eth_kzg_blob_to_kzg_commitment(_context, blobPtr, commitmentPtr);

            ThrowOnError(result);
        }

        return commitment;
    }

    public unsafe (byte[][], byte[][]) ComputeCellsAndKZGProofs(byte[] blob)
    {
        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length");
        }

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

            CResult result = eth_kzg_compute_cells_and_kzg_proofs(_context, blobPtr, outCellsPtrPtr, outProofsPtrPtr);
            ThrowOnError(result);
        }
        return (outCells, outProofs);
    }

    public bool VerifyCellKZGProofBatch(byte[][] commitments, ulong[] cellIndices, byte[][] cells, byte[][] proofs)
    {

        // Length checks
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

        for (int i = 0; i < commitments.Length; i++)
        {
            if (commitments[i].Length != BytesPerCommitment)
            {
                throw new ArgumentException($"commitments at index {i} has an invalid length");
            }
        }

        int numCells = cells.Length;
        int numProofs = proofs.Length;
        int numCommitments = commitments.Length;

        byte*[] commPtrs = new byte*[numCommitments];
        byte*[] cellsPtrs = new byte*[numCells];
        byte*[] proofsPtrs = new byte*[numProofs];

        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte** commitmentPtrPtr = commPtrs)
        fixed (byte** cellsPtrPtr = cellsPtrs)
        fixed (byte** proofsPtrPtr = proofsPtrs)
        fixed (ulong* cellIndicesPtr = cellIndices)
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
            for (int i = 0; i < numCommitments; i++)
            {
                fixed (byte* commPtr = commitments[i])
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

            CResult result = eth_kzg_verify_cell_kzg_proof_batch(_context, Convert.ToUInt64(commitments.Length), commitmentPtrPtr, Convert.ToUInt64(cellIndices.Length), cellIndicesPtr, Convert.ToUInt64(cells.Length), cellsPtrPtr, Convert.ToUInt64(proofs.Length), proofsPtrPtr, verifiedPtr);
            ThrowOnError(result);
        }
        return verified;
    }

    public (byte[][], byte[][]) RecoverCellsAndKZGProofs(ulong[] cellIds, byte[][] cells)
    {

        // Length checks
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

            CResult result = eth_kzg_recover_cells_and_proofs(_context, Convert.ToUInt64(numInputCells), inputCellsPtrPtr, Convert.ToUInt64(cellIds.Length), cellIdsPtr, outCellsPtrPtr, outProofsPtrPtr);
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
                    eth_kzg_free_error_message(result.error_msg);
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
                throw new ApplicationException("EthKZG returned an unexpected result variant");
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