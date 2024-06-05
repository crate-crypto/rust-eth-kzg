fn main() {
    csbindgen::Builder::default()
        .input_extern_file("../../../c/src/lib.rs")
        // .input_extern_file("src/lib.rs")
        .csharp_namespace("PeerDasKZG")
        .csharp_dll_name("c_peerdas_kzg") // TODO: pull in the package name from c crate
        .csharp_class_name("PeerDasKZG")
        .csharp_use_nint_types(false)
        .generate_csharp_file("../dotnet/NativeMethods.g.cs")
        .unwrap();
    //
}
