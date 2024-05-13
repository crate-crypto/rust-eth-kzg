use std::{env, path::PathBuf};

/// Name of the java file that we will use to generate the java bindings from
const NAME_OF_JAVA_BINDINGS_FILE: &str = "LibPeerDASKZG.java";

fn main() {
    println!("cargo:rerun-if-changed={}", NAME_OF_JAVA_BINDINGS_FILE);

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let path_to_java_bindings_file = PathBuf::from(crate_dir).join(NAME_OF_JAVA_BINDINGS_FILE);
    // Generate the header file
    std::process::Command::new("javac")
        .arg("-h")
        .arg(".")
        .arg(path_to_java_bindings_file)
        .output()
        .unwrap();
}
