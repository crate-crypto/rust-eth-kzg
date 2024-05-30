using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;

namespace DynamicLibraryLoader
{
    public static class RustLibrary
    {
        // When the static methods are called, .NET will look for the library in some
        // conventional locations. If it cannot find it, it will then trigger 
        // "ResolvingUnmanagedDll" event.
        // The below just says that LoadNativeLibrary will handle this event.
        //
        // The first parameter to DLLImport is the path that gets passed to the event handler.
        static RustLibrary() => AssemblyLoadContext.Default.ResolvingUnmanagedDll += LoadNativeLibrary;

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
                RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && RuntimeInformation.ProcessArchitecture == Architecture.X64 ? "x86_64-pc-windows-msvc" :
                RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "aarch64-pc-windows-msvc" :
                "";

            string extension =
                RuntimeInformation.IsOSPlatform(OSPlatform.Linux) ? "so" :
                RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "dylib" :
                RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "dll" : "";

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

        [DllImport("c_peerdas_kzg", EntryPoint = "callable_from_c", CallingConvention = CallingConvention.Cdecl)]
        public static extern long CallableFromC(long input);

        [DllImport("c_peerdas_kzg", EntryPoint = "prover_context_new", CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr ProverContextNew();

        [DllImport("c_peerdas_kzg", EntryPoint = "prover_context_free", CallingConvention = CallingConvention.Cdecl)]
        public static extern void ProverContextFree(IntPtr ctx);

        [DllImport("c_peerdas_kzg", EntryPoint = "blob_to_kzg_commitment", CallingConvention = CallingConvention.Cdecl)]
        public static extern void BlobToKzgCommitment(IntPtr ctx, byte[] blob, byte[] outCommitment);

        [DllImport("c_peerdas_kzg", EntryPoint = "compute_cells_and_kzg_proofs", CallingConvention = CallingConvention.Cdecl)]
        public static extern void ComputeCellsAndKzgProofs(IntPtr ctx, byte[] blob, byte[] outCells, byte[] outProofs);

        [DllImport("c_peerdas_kzg", EntryPoint = "verifier_context_new", CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr VerifierContextNew();

        [DllImport("c_peerdas_kzg", EntryPoint = "verifier_context_free", CallingConvention = CallingConvention.Cdecl)]
        public static extern void VerifierContextFree(IntPtr ctx);
    }

    class Program
    {
        static void Main()
        {
            long result1 = RustLibrary.CallableFromC(10);
            Console.WriteLine($"Result from callable_from_c: {result1}");
        }
    }
}
