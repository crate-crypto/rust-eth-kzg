/// Data from the test input could also be malformed,
/// So we use this type to represent that.
/// For example, although a proof should be 48 bytes, the test input
/// could give us 47.
pub type UnsafeBytes = Vec<u8>;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn collect_test_files<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_test_files_inner(dir, &mut files)?;

    // Check that the directory is not empty
    assert!(!files.is_empty());

    Ok(files)
}

fn collect_test_files_inner<P: AsRef<Path>>(dir: P, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files_inner(path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn remove_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x").map_or_else(
        || {
            panic!(
                "hex strings in ethereum are assumed to be prefixed with a 0x.
                If this is not the case, it is not a bug, however it is cause for concern,
                if there are discrepancies."
            );
        },
        |stripped| stripped,
    )
}

pub fn bytes_from_hex(bytes: &str) -> Vec<u8> {
    let bytes = remove_hex_prefix(bytes);
    hex::decode(bytes).unwrap()
}
