use std::{env, fs, path::PathBuf};

use toml::Value;

fn main() {
    let package_name_of_c_crate = get_package_name_of_c_crate();

    csbindgen::Builder::default()
        .input_extern_file("../../c/src/lib.rs")
        .csharp_namespace("PeerDasKZG")
        .csharp_dll_name(package_name_of_c_crate)
        .csharp_class_name("PeerDasKZG")
        .csharp_use_nint_types(false)
        .generate_csharp_file("../dotnet/NativeMethods.g.cs")
        .unwrap();
}

fn get_package_name_of_c_crate() -> String {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_dir = PathBuf::from(crate_dir);

    // Go up two directories to be at bindings parent directory
    let parent = crate_dir.parent().unwrap().parent().unwrap().to_path_buf();
    let path_to_c_crate = parent.join("c");
    let path_to_c_crate_cargo_toml = path_to_c_crate.join("Cargo.toml");

    // Read the Cargo.toml of the other crate
    let cargo_toml =
        fs::read_to_string(path_to_c_crate_cargo_toml).expect("Failed to read Cargo.toml");

    // Parse the Cargo.toml content
    let cargo_toml: Value = cargo_toml.parse().expect("Failed to parse Cargo.toml");

    // Access the package name from the parsed Cargo.toml
    let package_name = cargo_toml["package"]["name"]
        .as_str()
        .expect("Failed to get package name");

    package_name.to_string()
}
