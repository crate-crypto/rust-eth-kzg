use eip7594::Blob;
use eip7594::{Bytes48, Cell};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn collect_test_files<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    _collect_test_files(dir, &mut files)?;

    // Check that the directory is not empty
    assert!(!files.is_empty());

    Ok(files)
}
fn _collect_test_files<P: AsRef<Path>>(dir: P, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            _collect_test_files(path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn remove_hex_prefix(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        panic!(
            "hex strings in ethereum are assumed to be prefixed with a 0x. 
                If this is not the case, it is not a bug, however it is cause for concern, 
                if there are discrepancies."
        );
    }
}

pub fn blob_from_hex(blob: &str) -> Blob {
    let blob = remove_hex_prefix(&blob);
    hex::decode(blob).unwrap()
}

pub fn bytes48_from_hex(bytes48: &str) -> Bytes48 {
    let bytes48 = remove_hex_prefix(&bytes48);
    hex::decode(bytes48).unwrap().try_into().unwrap()
}

pub fn cell_from_hex(cell: &str) -> Cell {
    let cell = remove_hex_prefix(&cell);
    hex::decode(cell).unwrap().try_into().unwrap()
}
