using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;

namespace PeerDASKZG;

public static unsafe partial class PeerDASKZG
{
    [DllImport("c_peerdas_kzg", EntryPoint = "peerdas_context_new", CallingConvention = CallingConvention.Cdecl)]
    private static extern PeerDASContext* InternalPeerDASContextNew();

    [DllImport("c_peerdas_kzg", EntryPoint = "peerdas_context_free", CallingConvention = CallingConvention.Cdecl)]
    private static extern void InternalPeerDASContextFree(PeerDASContext* ctx);

    [DllImport("c_peerdas_kzg", EntryPoint = "blob_to_kzg_commitment", CallingConvention = CallingConvention.Cdecl)]
    private static extern Result InternalBlobToKzgCommitment(PeerDASContext* ctx, ulong blobLength, byte[] blob, byte[] outCommitment);

    [DllImport("c_peerdas_kzg", EntryPoint = "compute_cells", CallingConvention = CallingConvention.Cdecl)]
    private static extern Result InternalComputeCells(PeerDASContext* ctx, ulong blobLength, byte[] blob, byte[] outCells);

    [DllImport("c_peerdas_kzg", EntryPoint = "compute_cells_and_kzg_proofs", CallingConvention = CallingConvention.Cdecl)]
    private static extern Result InternalComputeCellsAndKzgProofs(PeerDASContext* ctx, ulong blobLength, byte[] blob, byte[] outCells, byte[] outProofs);

    [DllImport("c_peerdas_kzg", EntryPoint = "verify_cell_kzg_proof", CallingConvention = CallingConvention.Cdecl)]
    private static extern Result InternalVerifyCellKZGProof(PeerDASContext* ctx, ulong cellLength, byte[] cell, ulong commitmentLength, byte[] commitment, ulong cellId, ulong proofLength, byte[] proof, out bool verified);

    [DllImport("c_peerdas_kzg", EntryPoint = "verify_cell_kzg_proof_batch", CallingConvention = CallingConvention.Cdecl)]
    private static extern Result InternalVerifyCellKZGProofBatch(PeerDASContext* ctx, ulong rowCommitmentsLength, byte[] rowCommitments, ulong rowIndicesLength, ulong[] rowIndices, ulong columnIndicesLength, ulong[] columnIndices, ulong cellsLength, byte[] cells, ulong proofsLength, byte[] proofs, out bool verified);

    [DllImport("c_peerdas_kzg", EntryPoint = "recover_all_cells", CallingConvention = CallingConvention.Cdecl)]
    private static extern Result InternalRecoverAllCells(PeerDASContext* ctx, ulong cellsLength, byte[] cells, ulong cellIdsLength, ulong[] cellIds, byte[] outCells);

    internal enum Result : uint
    {
        Ok,
        Err,
    }

    [StructLayout(LayoutKind.Sequential)]
    public unsafe partial struct PeerDASContext
    {
    }
}

