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

    public EthKZG(bool usePrecomp = true)
    {
        _context = eth_kzg_das_context_new(usePrecomp);
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

    public unsafe (Memory<byte>[], Memory<byte>[]) ComputeCellsAndKZGProofs(byte[] blob)
    {
        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length");
        }

        int numProofs = CellsPerExtBlob;
        int numCells = CellsPerExtBlob;

        byte[] outCells = new byte[numCells * BytesPerCell];
        byte[] outProofs = new byte[numCells * BytesPerProof];

        // Allocate an array of pointers for cells and proofs
        byte*[] outCellsPtrs = new byte*[numCells];
        byte*[] outProofsPtrs = new byte*[numProofs];

        fixed (byte* blobPtr = blob)
        fixed (byte* outCellsPtr = outCells)
        fixed (byte* outProofsPtr = outProofs)
        fixed (byte** outCellsPtrPtr = outCellsPtrs)
        fixed (byte** outProofsPtrPtr = outProofsPtrs)
        {

            // Get the pointer for each cell
            for (int i = 0; i < numCells; i++)
            {
                outCellsPtrPtr[i] = outCellsPtr + i * BytesPerCell;
            }

            // Get the pointer for each proof
            for (int i = 0; i < numCells; i++)
            {
                outProofsPtrPtr[i] = outProofsPtr + i * BytesPerProof;
            }

            CResult result = eth_kzg_compute_cells_and_kzg_proofs(_context, blobPtr, outCellsPtrPtr, outProofsPtrPtr);
            ThrowOnError(result);
        }

        return (Segment(outCells, BytesPerCell), Segment(outProofs, BytesPerProof));
    }

    public unsafe Memory<byte>[] ComputeCells(byte[] blob)
    {
        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length");
        }

        int numCells = CellsPerExtBlob;

        byte[] outCells = new byte[numCells * BytesPerCell];

        // Allocate an array of pointers for cells and proofs
        byte*[] outCellsPtrs = new byte*[numCells];

        fixed (byte* blobPtr = blob)
        fixed (byte* outCellsPtr = outCells)
        fixed (byte** outCellsPtrPtr = outCellsPtrs)
        {

            // Get the pointer for each cell
            for (int i = 0; i < numCells; i++)
            {
                outCellsPtrPtr[i] = outCellsPtr + i * BytesPerCell;
            }

            CResult result = eth_kzg_compute_cells(_context, blobPtr, outCellsPtrPtr);
            ThrowOnError(result);
        }

        return Segment(outCells, BytesPerCell);
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

        // Flatten arrays to avoid nested fixed blocks
        byte[] flattenedCells = FlattenArray(cells);
        byte[] flattenedCommitments = FlattenArray(commitments);
        byte[] flattenedProofs = FlattenArray(proofs);

        byte*[] cellsPtrs = new byte*[numCells];
        byte*[] commPtrs = new byte*[numCommitments];
        byte*[] proofsPtrs = new byte*[numProofs];

        bool verified = false;
        bool* verifiedPtr = &verified;

        fixed (byte* flatCellsPtr = flattenedCells)
        fixed (byte* flatCommPtr = flattenedCommitments)
        fixed (byte* flatProofsPtr = flattenedProofs)
        fixed (byte** cellsPtrPtr = cellsPtrs)
        fixed (byte** commitmentPtrPtr = commPtrs)
        fixed (byte** proofsPtrPtr = proofsPtrs)
        fixed (ulong* cellIndicesPtr = cellIndices)
        {
            // Set pointers to flattened data
            for (int i = 0; i < numCells; i++)
            {
                cellsPtrs[i] = flatCellsPtr + i * BytesPerCell;
            }

            for (int i = 0; i < numCommitments; i++)
            {
                commPtrs[i] = flatCommPtr + i * BytesPerCommitment;
            }

            for (int i = 0; i < numProofs; i++)
            {
                proofsPtrs[i] = flatProofsPtr + i * BytesPerProof;
            }

            CResult result = eth_kzg_verify_cell_kzg_proof_batch(_context, Convert.ToUInt64(commitments.Length), commitmentPtrPtr, Convert.ToUInt64(cellIndices.Length), cellIndicesPtr, Convert.ToUInt64(cells.Length), cellsPtrPtr, Convert.ToUInt64(proofs.Length), proofsPtrPtr, verifiedPtr);
            ThrowOnError(result);
        }
        return verified;
    }

    public (Memory<byte>[], Memory<byte>[]) RecoverCellsAndKZGProofs(ulong[] cellIds, byte[][] cells)
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

        byte[] outCells = new byte[numOutCells * BytesPerCell];
        byte[] outProofs = new byte[numProofs * BytesPerProof];

        // Allocate an array of pointers for inputCells, outputCells and proofs
        byte*[] inputCellsPtrs = new byte*[numInputCells];
        byte*[] outCellsPtrs = new byte*[numOutCells];
        byte*[] outProofsPtrs = new byte*[numProofs];

        // Flatten input cells to avoid nested fixed blocks
        byte[] flattenedInputCells = FlattenArray(cells);

        fixed (ulong* cellIdsPtr = cellIds)
        fixed (byte* flatInputCellsPtr = flattenedInputCells)
        fixed (byte* outCellsPtr = outCells)
        fixed (byte* outProofsPtr = outProofs)
        fixed (byte** inputCellsPtrPtr = inputCellsPtrs)
        fixed (byte** outCellsPtrPtr = outCellsPtrs)
        fixed (byte** outProofsPtrPtr = outProofsPtrs)
        {
            // Set pointers to flattened input cells
            for (int i = 0; i < numInputCells; i++)
            {
                inputCellsPtrs[i] = flatInputCellsPtr + i * BytesPerCell;
            }

            // Get the pointer for each output cell
            for (int i = 0; i < numOutCells; i++)
            {
                outCellsPtrPtr[i] = outCellsPtr + i * BytesPerCell;
            }

            // Get the pointer for each proof
            for (int i = 0; i < numProofs; i++)
            {
                outProofsPtrPtr[i] = outProofsPtr + i * BytesPerProof;
            }

            CResult result = eth_kzg_recover_cells_and_proofs(_context, Convert.ToUInt64(numInputCells), inputCellsPtrPtr, Convert.ToUInt64(cellIds.Length), cellIdsPtr, outCellsPtrPtr, outProofsPtrPtr);
            ThrowOnError(result);
        }

        return (Segment(outCells, BytesPerCell), Segment(outProofs, BytesPerProof));
    }

    // EIP-4844 methods

    public unsafe (byte[], byte[]) ComputeKzgProof(byte[] blob, byte[] z)
    {
        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length. Expected {BytesPerBlob}, got {blob.Length}");
        }

        if (z.Length != BytesPerFieldElement)
        {
            throw new ArgumentException($"z has an invalid length. Expected {BytesPerFieldElement}, got {z.Length}");
        }

        byte[] proof = new byte[BytesPerProof];
        byte[] y = new byte[BytesPerFieldElement];

        fixed (byte* blobPtr = blob)
        fixed (byte* zPtr = z)
        fixed (byte* proofPtr = proof)
        fixed (byte* yPtr = y)
        {
            CResult result = eth_kzg_compute_kzg_proof(_context, blobPtr, zPtr, proofPtr, yPtr);
            ThrowOnError(result);
        }

        return (proof, y);
    }

    public unsafe byte[] ComputeBlobKzgProof(byte[] blob, byte[] commitment)
    {
        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length. Expected {BytesPerBlob}, got {blob.Length}");
        }

        if (commitment.Length != BytesPerCommitment)
        {
            throw new ArgumentException($"commitment has an invalid length. Expected {BytesPerCommitment}, got {commitment.Length}");
        }

        byte[] proof = new byte[BytesPerProof];

        fixed (byte* blobPtr = blob)
        fixed (byte* commitmentPtr = commitment)
        fixed (byte* proofPtr = proof)
        {
            CResult result = eth_kzg_compute_blob_kzg_proof(_context, blobPtr, commitmentPtr, proofPtr);
            ThrowOnError(result);
        }

        return proof;
    }

    public unsafe bool VerifyKzgProof(byte[] commitment, byte[] z, byte[] y, byte[] proof)
    {
        // Length checks
        if (commitment.Length != BytesPerCommitment)
        {
            throw new ArgumentException($"commitment has an invalid length. Expected {BytesPerCommitment}, got {commitment.Length}");
        }

        if (z.Length != BytesPerFieldElement)
        {
            throw new ArgumentException($"z has an invalid length. Expected {BytesPerFieldElement}, got {z.Length}");
        }

        if (y.Length != BytesPerFieldElement)
        {
            throw new ArgumentException($"y has an invalid length. Expected {BytesPerFieldElement}, got {y.Length}");
        }

        if (proof.Length != BytesPerProof)
        {
            throw new ArgumentException($"proof has an invalid length. Expected {BytesPerProof}, got {proof.Length}");
        }

        bool verified = false;

        fixed (byte* commitmentPtr = commitment)
        fixed (byte* zPtr = z)
        fixed (byte* yPtr = y)
        fixed (byte* proofPtr = proof)
        {
            CResult result = eth_kzg_verify_kzg_proof(_context, commitmentPtr, zPtr, yPtr, proofPtr, &verified);
            ThrowOnError(result);
        }

        return verified;
    }

    public unsafe bool VerifyBlobKzgProof(byte[] blob, byte[] commitment, byte[] proof)
    {
        // Length checks
        if (blob.Length != BytesPerBlob)
        {
            throw new ArgumentException($"blob has an invalid length. Expected {BytesPerBlob}, got {blob.Length}");
        }

        if (commitment.Length != BytesPerCommitment)
        {
            throw new ArgumentException($"commitment has an invalid length. Expected {BytesPerCommitment}, got {commitment.Length}");
        }

        if (proof.Length != BytesPerProof)
        {
            throw new ArgumentException($"proof has an invalid length. Expected {BytesPerProof}, got {proof.Length}");
        }

        bool verified = false;

        fixed (byte* blobPtr = blob)
        fixed (byte* commitmentPtr = commitment)
        fixed (byte* proofPtr = proof)
        {
            CResult result = eth_kzg_verify_blob_kzg_proof(_context, blobPtr, commitmentPtr, proofPtr, &verified);
            ThrowOnError(result);
        }

        return verified;
    }

    public unsafe bool VerifyBlobKzgProofBatch(byte[][] blobs, byte[][] commitments, byte[][] proofs)
    {
        // Length checks
        if (blobs.Length != commitments.Length || blobs.Length != proofs.Length)
        {
            throw new ArgumentException($"blobs, commitments, and proofs must have the same length");
        }

        for (int i = 0; i < blobs.Length; i++)
        {
            if (blobs[i].Length != BytesPerBlob)
            {
                throw new ArgumentException($"blob at index {i} has an invalid length. Expected {BytesPerBlob}, got {blobs[i].Length}");
            }
        }

        for (int i = 0; i < commitments.Length; i++)
        {
            if (commitments[i].Length != BytesPerCommitment)
            {
                throw new ArgumentException($"commitment at index {i} has an invalid length. Expected {BytesPerCommitment}, got {commitments[i].Length}");
            }
        }

        for (int i = 0; i < proofs.Length; i++)
        {
            if (proofs[i].Length != BytesPerProof)
            {
                throw new ArgumentException($"proof at index {i} has an invalid length. Expected {BytesPerProof}, got {proofs[i].Length}");
            }
        }

        int numBlobs = blobs.Length;
        int numCommitments = commitments.Length;
        int numProofs = proofs.Length;

        // Flatten arrays to avoid nested fixed blocks
        byte[] flattenedBlobs = FlattenArray(blobs);
        byte[] flattenedCommitments = FlattenArray(commitments);
        byte[] flattenedProofs = FlattenArray(proofs);

        byte*[] blobPtrs = new byte*[numBlobs];
        byte*[] commitmentPtrs = new byte*[numCommitments];
        byte*[] proofPtrs = new byte*[numProofs];

        bool verified = false;

        fixed (byte* flatBlobsPtr = flattenedBlobs)
        fixed (byte* flatCommPtr = flattenedCommitments)
        fixed (byte* flatProofsPtr = flattenedProofs)
        fixed (byte** blobPtrPtr = blobPtrs)
        fixed (byte** commitmentPtrPtr = commitmentPtrs)
        fixed (byte** proofPtrPtr = proofPtrs)
        {
            // Set pointers to flattened data
            for (int i = 0; i < numBlobs; i++)
            {
                blobPtrs[i] = flatBlobsPtr + i * BytesPerBlob;
            }

            for (int i = 0; i < numCommitments; i++)
            {
                commitmentPtrs[i] = flatCommPtr + i * BytesPerCommitment;
            }

            for (int i = 0; i < numProofs; i++)
            {
                proofPtrs[i] = flatProofsPtr + i * BytesPerProof;
            }

            CResult result = eth_kzg_verify_blob_kzg_proof_batch(_context,
                Convert.ToUInt64(numBlobs), blobPtrPtr,
                Convert.ToUInt64(numCommitments), commitmentPtrPtr,
                Convert.ToUInt64(numProofs), proofPtrPtr,
                &verified);
            ThrowOnError(result);
        }

        return verified;
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

    private static Memory<byte>[] Segment(byte[] arr, int itemLength)
    {
        Memory<byte>[] result = new Memory<byte>[arr.Length / itemLength];

        for (int i = 0; i < result.Length; i++)
        {
            result[i] = new Memory<byte>(arr, i * itemLength, itemLength);
        }

        return result;
    }
}