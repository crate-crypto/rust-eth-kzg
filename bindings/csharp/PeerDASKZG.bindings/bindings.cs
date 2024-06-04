using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;

namespace PeerDASKZG;

public static unsafe partial class PeerDASKZG
{
    // When the static methods are called, .NET will look for the library in some
    // conventional locations. If it cannot find it, it will then trigger 
    // "ResolvingUnmanagedDll" event.
    // The below just says that LoadNativeLibrary will handle this event.
    //
    // The first parameter to DLLImport is the path that gets passed to the event handler.
    static PeerDASKZG() => AssemblyLoadContext.Default.ResolvingUnmanagedDll += LoadNativeLibrary;

    private static IntPtr LoadNativeLibrary(Assembly _, string path)
    {
        // This checks whether the requested library is the one we're interested in
        // ie this class can only be used to load a dynamic library with the name "c_peerdas_kzg"
        if (!path.Equals("c_peerdas_kzg", StringComparison.OrdinalIgnoreCase))
        {
            return IntPtr.Zero;
        }

        string target =
            RuntimeInformation.IsOSPlatform(OSPlatform.Linux) && RuntimeInformation.ProcessArchitecture == Architecture.X64 ? "x86_64-unknown-linux-gnu" :
            RuntimeInformation.IsOSPlatform(OSPlatform.Linux) && RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "aarch64-unknown-linux-gnu" :
            RuntimeInformation.IsOSPlatform(OSPlatform.OSX) && RuntimeInformation.ProcessArchitecture == Architecture.X64 ? "x86_64-apple-darwin" :
            RuntimeInformation.IsOSPlatform(OSPlatform.OSX) && RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "aarch64-apple-darwin" :
            RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && RuntimeInformation.ProcessArchitecture == Architecture.X64 ? "x86_64-pc-windows-gnu" :
            // Windows on ARM doesn't seem to be massively supported in nethermind. Check the secp256k1 bindings for example.
            // We can add support for it later if needed.
            // RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "aarch64-pc-windows-msvc" :
            "";

        string extension =
            RuntimeInformation.IsOSPlatform(OSPlatform.Linux) ? "so" :
            RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "dylib" :
            RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "dll" : "";

        // All platforms should have an extension, an unknown extension is unexpected and an error
        if (extension == "")
        {
            return IntPtr.Zero;
        }

        // Windows doesn't have a lib prefix
        string prefix =
           RuntimeInformation.IsOSPlatform(OSPlatform.Linux) || RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "lib" : "";

        string baseDirectory = AppContext.BaseDirectory;

        string libraryPath = Path.Combine(baseDirectory, $"runtimes/{target}/{prefix}{path}.{extension}");

        if (File.Exists(libraryPath))
        {
            return NativeLibrary.Load(libraryPath);
        }

        return IntPtr.Zero;
    }

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

