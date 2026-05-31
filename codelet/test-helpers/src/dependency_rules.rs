//! Dependency-rule regression helpers.
//!
//! These helpers codify the workspace's forbidden-arrow architectural
//! invariants (e.g. `codelet-fspec → codelet-napi` is forbidden) by
//! inspecting `cargo metadata` and source files. They are intended to
//! be called from `#[test]` functions in any crate's `tests/` directory.
//!
//! The pattern is lifted from the three RPC-044 regression tests in
//! `codelet/{fspec, fspec-tui, sessions}/tests/no_napi_dependency.rs`,
//! which were nearly byte-identical (only the `from_crate` literal
//! differed). RPC-067 unifies them here and adds new wrappers for
//! `codelet-core` and `codelet-rpc-types`.
//!
//! # Assertion failure modes
//!
//! Both helpers `panic!` on failure with a message that names the
//! offending crate / file / package, so a sabotage (e.g. adding
//! `codelet-napi` to `codelet-core/Cargo.toml`) produces a loud test
//! failure that points directly at the violation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the workspace root (`codelet/`) by walking up from the test
/// crate's `CARGO_MANIFEST_DIR`.
///
/// Tests invoke the helpers from `codelet/<crate>/tests/*.rs`. Their
/// `CARGO_MANIFEST_DIR` is `codelet/<crate>/`, so the workspace root is
/// the parent of the calling crate's manifest dir.
fn workspace_root_from_caller_manifest(manifest_dir: &str) -> PathBuf {
    PathBuf::from(manifest_dir)
        .parent()
        .expect("test crate's CARGO_MANIFEST_DIR must have a parent (the workspace root)")
        .to_path_buf()
}

/// Strip Rust line (`//`) and block (`/* ... */`) comments from `src`
/// so substring assertions don't false-positive on prose inside
/// doc-comments or module-level explanation.
///
/// Lifted verbatim from the seven existing copies across the workspace
/// (RPC-044 / RPC-006 / RPC-049 / RPC-050 / RPC-056..061 tests).
pub fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
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

/// Recursively collect every `.rs` file under `root`.
fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
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

/// Walk the transitive dependency graph of `from_crate` via
/// `cargo metadata --format-version 1` and panic if `forbidden_pkg`
/// appears in the resolved set.
///
/// The workspace root is the parent of the caller's manifest directory
/// (i.e. `codelet/<test_crate>/..` resolves to `codelet/`). This works
/// for every workspace member's `tests/*.rs` because cargo passes the
/// caller's manifest dir as `CARGO_MANIFEST_DIR` at compile time, which
/// the calling test forwards via the [`assert_no_transitive_dependency`]
/// macro defined alongside this fn — see callers in
/// `codelet/{core, rpc-types, fspec, fspec-tui, sessions}/tests/no_napi_dependency.rs`.
///
/// # Panics
///
/// - If `cargo metadata` fails.
/// - If the metadata JSON cannot be parsed.
/// - If `from_crate` is not a workspace member.
/// - If `forbidden_pkg` is found anywhere in the transitive graph.
pub fn assert_no_transitive_dependency_with_manifest(
    caller_manifest_dir: &str,
    from_crate: &str,
    forbidden_pkg: &str,
) {
    let workspace = workspace_root_from_caller_manifest(caller_manifest_dir);

    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace.join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");

    assert!(
        output.status.success(),
        "cargo metadata failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("cargo metadata JSON must parse");

    let packages = json
        .get("packages")
        .and_then(|v| v.as_array())
        .expect("`packages` must be an array");
    let resolve = json.get("resolve").expect("`resolve` must exist");
    let nodes = resolve
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("`resolve.nodes` must be an array");

    let root_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(from_crate))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .unwrap_or_else(|| panic!("`{from_crate}` package must exist in workspace metadata"))
        .to_string();

    // BFS over the resolve graph rooted at `from_crate`.
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stack: Vec<String> = vec![root_id];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|i| i.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for d in deps {
                        if let Some(s) = d.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
                break;
            }
        }
    }

    // Map ids back to names.
    let mut transitive_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for id in &seen {
        if let Some(pkg) = packages
            .iter()
            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                transitive_names.insert(name.to_string());
            }
        }
    }

    assert!(
        !transitive_names.contains(forbidden_pkg),
        "{from_crate} MUST NOT transitively depend on {forbidden_pkg} (forbidden-arrow invariant). Transitive set: {transitive_names:?}"
    );
}

/// Walk every `.rs` file under `codelet/<crate_dir_name>/src/`, strip
/// comments, and panic if any file contains an import of
/// `forbidden_module` (matched by either the `use <forbidden_module>` or
/// `<forbidden_module>::` substring).
///
/// # Panics
///
/// - If `codelet/<crate_dir_name>/src/` is not a directory.
/// - If `codelet/<crate_dir_name>/src/` contains zero `.rs` files (so
///   the scan can't pass vacuously).
/// - If any source file contains a non-commented reference to
///   `forbidden_module`.
pub fn assert_no_import_in_sources_with_manifest(
    caller_manifest_dir: &str,
    crate_dir_name: &str,
    forbidden_module: &str,
) {
    let workspace = workspace_root_from_caller_manifest(caller_manifest_dir);
    let src_dir = workspace.join(crate_dir_name).join("src");
    assert!(
        src_dir.is_dir(),
        "codelet/{crate_dir_name}/src must be a directory; got {}",
        src_dir.display()
    );

    let rs_files = collect_rs_files(&src_dir);
    assert!(
        !rs_files.is_empty(),
        "codelet/{crate_dir_name}/src must contain at least one .rs file"
    );

    let use_needle = format!("use {forbidden_module}");
    let path_needle = format!("{forbidden_module}::");

    let mut offenders: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let code = strip_rust_comments(&body);
        if code.contains(&use_needle) || code.contains(&path_needle) {
            offenders.push(path.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "codelet/{crate_dir_name}/src MUST NOT import {forbidden_module} (forbidden-arrow invariant). Offending files: {offenders:?}"
    );
}

/// Convenience macro: forwards the calling test crate's
/// `CARGO_MANIFEST_DIR` (resolved at the call site, not in this crate)
/// into [`assert_no_transitive_dependency_with_manifest`].
///
/// Use this from a test like:
///
/// ```ignore
/// use codelet_test_helpers::assert_no_transitive_dependency;
/// #[test]
/// fn no_codelet_napi_in_dependency_graph() {
///     assert_no_transitive_dependency!("codelet-fspec", "codelet-napi");
/// }
/// ```
#[macro_export]
macro_rules! assert_no_transitive_dependency {
    ($from_crate:expr, $forbidden_pkg:expr) => {
        $crate::dependency_rules::assert_no_transitive_dependency_with_manifest(
            env!("CARGO_MANIFEST_DIR"),
            $from_crate,
            $forbidden_pkg,
        )
    };
}

/// Convenience macro: forwards the calling test crate's
/// `CARGO_MANIFEST_DIR` (resolved at the call site, not in this crate)
/// into [`assert_no_import_in_sources_with_manifest`].
///
/// Use this from a test like:
///
/// ```ignore
/// use codelet_test_helpers::assert_no_import_in_sources;
/// #[test]
/// fn no_codelet_napi_import_in_source() {
///     assert_no_import_in_sources!("fspec", "codelet_napi");
/// }
/// ```
#[macro_export]
macro_rules! assert_no_import_in_sources {
    ($crate_dir_name:expr, $forbidden_module:expr) => {
        $crate::dependency_rules::assert_no_import_in_sources_with_manifest(
            env!("CARGO_MANIFEST_DIR"),
            $crate_dir_name,
            $forbidden_module,
        )
    };
}
