//! Shared source-inspection helpers for RPC-005 architecture-invariants
//! tests.
//!
//! Several integration tests in this crate need to read source files
//! from sibling workspace crates and inspect them for forbidden patterns.
//! Pulling the helpers into a shared module keeps the per-scenario test
//! file under the 300-line project limit.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

/// Workspace-relative path resolution from the rpc-embedded crate root.
///
/// Cargo runs integration tests with CWD = crate root. The codelet
/// workspace root is exactly one level up.
pub fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("rpc-embedded crate must have a parent (the codelet workspace root)")
        .to_path_buf()
}

pub fn read_to_string_or_panic(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("Failed to read {}: {}", path.display(), e);
    })
}

/// Strip Rust line comments (`//...`) and block comments (`/* ... */`) so
/// that text inside prose does not register as "code". Doc comments
/// (`///`, `//!`, `/** */`, `/*! */`) start with `//` or `/*` and are
/// therefore covered.
pub fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            // Line comment - skip until newline
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            // Block comment - skip until closing */
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

pub fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}
