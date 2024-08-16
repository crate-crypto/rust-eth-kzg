// <auto-generated>
// This code is generated by csbindgen.
// DON'T CHANGE THIS DIRECTLY.
// </auto-generated>
#pragma warning disable CS8500
#pragma warning disable CS8981
using System;
using System.Runtime.InteropServices;


namespace EthKZG.Native
{
    internal static unsafe partial class NativeMethods
    {
        const string __DllName = "c_eth_kzg";



        /// <summary>
        ///  Create a new DASContext and return a pointer to it.
        ///
        ///  `num_threads`: set to `0`` to indicate that the library should pick a sensible default.
        ///
        ///  # Memory faults
        ///
        ///  To avoid memory leaks, one should ensure that the pointer is freed after use
        ///  by calling `eth_kzg_das_context_free`.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_das_context_new", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern DASContext* eth_kzg_das_context_new([MarshalAs(UnmanagedType.U1)] bool use_precomp, System.UIntPtr num_threads);

        /// <summary>
        ///  # Safety
        ///
        ///  - The caller must ensure that the pointer is valid. If the pointer is null, this method will return early.
        ///  - The caller should also avoid a double-free by setting the pointer to null after calling this method.
        ///
        ///  # Memory faults
        ///
        ///  - If this method is called twice on the same pointer, it will result in a double-free.
        ///
        ///  # Undefined behavior
        ///
        ///  - Since the `ctx` is created in Rust, we can only get undefined behavior, if the caller passes in
        ///  a pointer that was not created by `eth_kzg_das_context_new`.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_das_context_free", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void eth_kzg_das_context_free(DASContext* ctx);

        /// <summary>
        ///  Free the memory allocated for the error message.
        ///
        ///  # Safety
        ///
        ///  - The caller must ensure that the pointer is valid. If the pointer is null, this method will return early.
        ///  - The caller should also avoid a double-free by setting the pointer to null after calling this method.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_free_error_message", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void eth_kzg_free_error_message(byte* c_message);

        /// <summary>
        ///  Compute a commitment from a Blob
        ///
        ///  # Safety
        ///
        ///  - The caller must ensure that the pointers are valid.
        ///  - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
        ///  - The caller must ensure that `out` points to a region of memory that is at least `BYTES_PER_COMMITMENT` bytes.
        ///
        ///  # Undefined behavior
        ///
        ///  - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
        ///    If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_blob_to_kzg_commitment", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern CResult eth_kzg_blob_to_kzg_commitment(DASContext* ctx, byte* blob, byte* @out);

        /// <summary>
        ///  Computes the cells and KZG proofs for a given blob.
        ///
        ///  # Safety
        ///
        ///  - The caller must ensure that the pointers are valid. If pointers are null.
        ///  - The caller must ensure that `blob` points to a region of memory that is at least `BYTES_PER_BLOB` bytes.
        ///  - The caller must ensure that `out_cells` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` elements
        ///    and that each element is at least `BYTES_PER_CELL` bytes.
        ///  - The caller must ensure that `out_proofs` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` elements
        ///    and that each element is at least `BYTES_PER_COMMITMENT` bytes.
        ///
        ///  # Undefined behavior
        ///
        ///  - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
        ///    If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_compute_cells_and_kzg_proofs", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern CResult eth_kzg_compute_cells_and_kzg_proofs(DASContext* ctx, byte* blob, byte** out_cells, byte** out_proofs);

        /// <summary>
        ///  Verifies a batch of cells and their KZG proofs.
        ///
        ///  # Safety
        ///
        ///  - If the length parameter for a pointer is set to zero, then this implementation will not check if its pointer is
        ///    null. This is because the caller might have passed in a null pointer, if the length is zero. Instead an empty slice
        ///    will be created.
        ///
        ///  - The caller must ensure that the pointers are valid.
        ///  - The caller must ensure that `commitments` points to a region of memory that is at least `commitments_length` commitments
        ///    and that each commitment is at least `BYTES_PER_COMMITMENT` bytes.
        ///  - The caller must ensure that `row_indices` points to a region of memory that is at least `num_cells` elements
        ///    and that each element is 8 bytes.
        ///  - The caller must ensure that `cell_indices` points to a region of memory that is at least `num_cells` elements
        ///    and that each element is 8 bytes.
        ///  - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` proof and
        ///    that each cell is at least `BYTES_PER_CELL` bytes
        ///  - The caller must ensure that `proofs` points to a region of memory that is at least `proofs_length` proofs
        ///     and that each proof is at least `BYTES_PER_COMMITMENT` bytes.
        ///  - The caller must ensure that `verified` points to a region of memory that is at least 1 byte.
        ///
        ///  # Undefined behavior
        ///
        ///  - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
        ///    If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_verify_cell_kzg_proof_batch", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern CResult eth_kzg_verify_cell_kzg_proof_batch(DASContext* ctx, ulong commitments_length, byte** commitments, ulong cell_indices_length, ulong* cell_indices, ulong cells_length, byte** cells, ulong proofs_length, byte** proofs, bool* verified);

        /// <summary>
        ///  Recovers all cells and their KZG proofs from the given cell indices and cells
        ///
        ///  # Safety
        ///
        ///   - If the length parameter for a pointer is set to zero, then this implementation will not check if its pointer is
        ///    null. This is because the caller might have passed in a null pointer, if the length is zero. Instead an empty slice
        ///    will be created.
        ///
        ///  - The caller must ensure that the pointers are valid.
        ///  - The caller must ensure that `cells` points to a region of memory that is at least `cells_length` cells
        ///    and that each cell is at least `BYTES_PER_CELL` bytes.
        ///  - The caller must ensure that `cell_indices` points to a region of memory that is at least `cell_indices_length` cell indices
        ///    and that each cell id is 8 bytes.
        ///  - The caller must ensure that `out_cells` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` cells
        ///    and that each cell is at least `BYTES_PER_CELL` bytes.
        ///  - The caller must ensure that `out_proofs` points to a region of memory that is at least `CELLS_PER_EXT_BLOB` proofs
        ///    and that each proof is at least `BYTES_PER_COMMITMENT` bytes.
        ///
        ///  # Undefined behavior
        ///
        ///  - This implementation will check if the ctx pointer is null, but it will not check if the other arguments are null.
        ///    If the other arguments are null, this method will dereference a null pointer and result in undefined behavior.
        /// </summary>
        [DllImport(__DllName, EntryPoint = "eth_kzg_recover_cells_and_proofs", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern CResult eth_kzg_recover_cells_and_proofs(DASContext* ctx, ulong cells_length, byte** cells, ulong cell_indices_length, ulong* cell_indices, byte** out_cells, byte** out_proofs);

        [DllImport(__DllName, EntryPoint = "eth_kzg_constant_bytes_per_cell", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ulong eth_kzg_constant_bytes_per_cell();

        [DllImport(__DllName, EntryPoint = "eth_kzg_constant_bytes_per_proof", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ulong eth_kzg_constant_bytes_per_proof();

        [DllImport(__DllName, EntryPoint = "eth_kzg_constant_cells_per_ext_blob", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ulong eth_kzg_constant_cells_per_ext_blob();


    }

    [StructLayout(LayoutKind.Sequential)]
    internal unsafe partial struct DASContext
    {
    }

    [StructLayout(LayoutKind.Sequential)]
    internal unsafe partial struct CResult
    {
        public CResultStatus status;
        public byte* error_msg;
    }


    internal enum CResultStatus : uint
    {
        Ok,
        Err,
    }


}
