use std::{env, fs, path::PathBuf};

use toml::Value;

/// The path where the generated bindings file will be written, relative to the bindings folder.
const PATH_FOR_CSHARP_BINDINGS_FILE: &str = "csharp/csharp_code/PeerDASKZG.bindings/bindings.g.cs";

fn main() {
    let package_name_of_c_crate = get_package_name_of_c_crate();

    let parent = path_to_bindings_folder();
    let path_to_output_file = parent.join(PATH_FOR_CSHARP_BINDINGS_FILE);

    csbindgen::Builder::default()
        .input_extern_file("../../c/src/lib.rs")
        .csharp_namespace("PeerDAS.Native")
        .csharp_dll_name(package_name_of_c_crate)
        .csharp_class_name("NativeMethods")
        .csharp_use_nint_types(false)
        .csharp_class_accessibility("public")
        // Once we can make methods internal with the bindgen code, we make everything internal including the NativeMethods class 
        .csharp_file_header("//TODO: The class and methods in this file are currently public\n// We want to eventually make them internal.\n// This is blocked by csbindgen making all methods public.\n// See: https://github.com/Cysharp/csbindgen/pull/83")
        .generate_csharp_file(path_to_output_file)
        .unwrap();
}

fn path_to_bindings_folder() -> PathBuf {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_dir = PathBuf::from(crate_dir);
    // Go up two directories to be at bindings parent directory
    let parent = crate_dir.parent().unwrap().parent().unwrap().to_path_buf();
    parent
}
fn get_package_name_of_c_crate() -> String {
    let parent = path_to_bindings_folder();

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
