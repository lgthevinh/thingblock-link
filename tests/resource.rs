//! Coverage for `ResourceRoot` construction — the hardware-free startup
//! validation of the configured resource root (Flow 1). Lib resolution (Flow 2)
//! gets its own cases once `resolve_lib_dir` lands.

use std::fs;

use thingblock_link::resource::ResourceRoot;
use thingblock_link::utils::tempdir::TempDir;

#[test]
fn new_accepts_an_existing_dir_and_canonicalizes() {
    let dir = TempDir::new("thingblock-link-resource").expect("create temp dir");
    let root = ResourceRoot::new(dir.path()).expect("existing dir is a valid root");

    // The stored path is absolute and still points at the same directory.
    assert!(root.path().is_absolute());
    assert_eq!(root.path(), dir.path().canonicalize().unwrap());
}

#[test]
fn new_rejects_a_missing_path() {
    let dir = TempDir::new("thingblock-link-resource").expect("create temp dir");
    let missing = dir.path().join("nope");

    assert!(ResourceRoot::new(&missing).is_err());
}

#[test]
fn new_rejects_a_file() {
    let dir = TempDir::new("thingblock-link-resource").expect("create temp dir");
    let file = dir.path().join("not-a-dir");
    fs::write(&file, b"x").expect("write file");

    assert!(ResourceRoot::new(&file).is_err());
}
