use std::{env, path::PathBuf};

/// Path to the java file that we will use to generate the java bindings from
///
/// Relative to the bindings folder.
const PATH_TO_JAVA_BINDINGS_FILE: &str =
    "java-code/src/main/java/ethereum/cryptography/LibPeerDASKZG.java";

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let path_to_bindings_dir = PathBuf::from(crate_dir).parent().unwrap().to_path_buf();
    let path_to_java_bindings_file = path_to_bindings_dir.join(PATH_TO_JAVA_BINDINGS_FILE);

    println!(
        "cargo:rerun-if-changed={}",
        path_to_java_bindings_file.as_os_str().to_str().unwrap()
    );

    // Generate the header file
    std::process::Command::new("javac")
        .arg("-h")
        .arg(".")
        .arg(path_to_java_bindings_file)
        .output()
        .unwrap();
}
