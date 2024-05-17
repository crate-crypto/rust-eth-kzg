use std::env;
use std::path::PathBuf;

/// The directory where the generated header file will be written.
const DIR_FOR_HEADER: &str = "lib";

fn main() {
    // This will run after the c headers are built, since this has a dependency on
    // the c library
    println!("cargo:rerun-if-changed=src/");
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_dir = PathBuf::from(crate_dir);

    // Go up two directories to be at bindings parent directory
    let parent = crate_dir.parent().unwrap().parent().unwrap().to_path_buf();
    let path_to_c_crate = parent.join("c");

    let package_name = env::var("CARGO_PKG_NAME").unwrap();

    let output_file = PathBuf::from(&crate_dir)
        .join(DIR_FOR_HEADER)
        .join(format!("{}.nim", package_name))
        .display()
        .to_string();
    nbindgen::Builder::new()
        .with_crate(path_to_c_crate)
        .generate()
        .unwrap()
        .write_to_file(&output_file);
}
