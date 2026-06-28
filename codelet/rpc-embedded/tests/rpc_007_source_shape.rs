//! RPC-007 source-shape integration tests.
//!
//! Feature: spec/features/session-rpcs-streamchunk-logevent-push-channels-repl-backend.feature
//!
//! - Scenario: Source-shape regression: rpc → napi remains forbidden and the five new types are defined exactly once
//!
//! Codifies the post-RPC-007 source-shape invariants by inspecting source files
//! statically. Does not exercise any runtime path. Restates the RPC-006
//! invariant (rpc → napi forbidden) and adds the RPC-007 type-uniqueness
//! invariant for SessionId, SessionInfo, SessionStatus, StreamChunk, LogRecord.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod source_helpers;

use source_helpers::{
    collect_rs_files, read_to_string_or_panic, strip_rust_comments, workspace_root,
};

#[test]
fn scenario_rpc_007_source_shape_invariants() {
    // @step Given the workspace contains codelet/rpc, codelet/rpc-types, codelet/rpc-server, codelet/rpc-embedded, codelet/napi, and codelet/core
    let root = workspace_root();
    for crate_name in [
        "rpc",
        "rpc-types",
        "rpc-server",
        "rpc-embedded",
        "napi",
        "core",
    ] {
        assert!(
            root.join(crate_name).join("Cargo.toml").exists(),
            "expected codelet/{crate_name}/Cargo.toml to exist",
        );
    }

    // @step When cargo metadata is queried for codelet/rpc/Cargo.toml dependencies
    let cargo_path = root.join("rpc").join("Cargo.toml");
    let cargo = read_to_string_or_panic(&cargo_path);
    let has_codelet_napi_dep = cargo.contains("codelet-napi");

    // Source-level use statements are also forbidden.
    let src_dir = root.join("rpc").join("src");
    let mut napi_imports: Vec<String> = Vec::new();
    for path in collect_rs_files(&src_dir) {
        let body = read_to_string_or_panic(&path);
        let code = strip_rust_comments(&body);
        if code.contains("use codelet_napi") || code.contains("codelet_napi::") {
            napi_imports.push(path.display().to_string());
        }
    }

    // @step Then no dependency named codelet-napi is present
    assert!(
        !has_codelet_napi_dep,
        "codelet/rpc/Cargo.toml MUST NOT depend on codelet-napi (RPC-007 source-shape preserves the RPC-006 invariant)",
    );
    assert!(
        napi_imports.is_empty(),
        "codelet/rpc/src must not import codelet_napi. Found: {napi_imports:?}",
    );

    // @step When ast-grep searches the workspace for definitions of StreamChunk, SessionInfo, SessionStatus, SessionId, and LogRecord
    let lifted_types = [
        "StreamChunk",
        "SessionInfo",
        "SessionStatus",
        "SessionId",
        "LogRecord",
    ];
    let mut definition_sites: std::collections::BTreeMap<&'static str, Vec<String>> =
        std::collections::BTreeMap::new();
    for ty in lifted_types {
        definition_sites.insert(ty, Vec::new());
    }
    for crate_name in [
        "rpc-types",
        "rpc",
        "rpc-server",
        "rpc-embedded",
        "napi",
        "core",
    ] {
        let src = root.join(crate_name).join("src");
        if !src.exists() {
            continue;
        }
        for path in collect_rs_files(&src) {
            let body = read_to_string_or_panic(&path);
            let code = strip_rust_comments(&body);
            for ty in lifted_types {
                // Match `pub struct {ty}` and `pub enum {ty}` only when the
                // identifier is followed by a word boundary so that
                // `SessionInfoJs` does not match `SessionInfo` (and similarly
                // for `LogRecordLayer` vs `LogRecord`). This is the
                // word-boundary fix landed alongside the RPC-007 lift; the
                // RPC-006 test used a looser substring match because none of
                // its lifted types had collision-prone prefixes.
                let needle_struct = format!("pub struct {ty}");
                let needle_enum = format!("pub enum {ty}");
                let next_char_is_boundary = |after: &str| {
                    after
                        .chars()
                        .next()
                        .map(|c| !c.is_alphanumeric() && c != '_')
                        .unwrap_or(true)
                };
                let mut found = false;
                for needle in [&needle_struct, &needle_enum] {
                    let mut start = 0;
                    while let Some(idx) = code[start..].find(needle.as_str()) {
                        let abs = start + idx;
                        let rest = &code[abs + needle.len()..];
                        if next_char_is_boundary(rest) {
                            found = true;
                            break;
                        }
                        start = abs + needle.len();
                    }
                    if found {
                        break;
                    }
                }
                if found {
                    definition_sites
                        .get_mut(ty)
                        .unwrap()
                        .push(path.display().to_string());
                }
            }
        }
    }

    // @step Then each type has exactly one definition site, located in codelet/rpc-types
    for (ty, sites) in &definition_sites {
        assert_eq!(
            sites.len(),
            1,
            "type {ty} must be defined exactly once. Found definition sites: {sites:?}",
        );
        assert!(
            sites[0].contains("rpc-types"),
            "type {ty} must be defined in codelet/rpc-types. Found: {}",
            sites[0],
        );
    }

    // @step And codelet/napi re-exports each type via the existing #[cfg_attr(feature = "napi", napi(...))] pattern
    let napi_lib = read_to_string_or_panic(&root.join("napi").join("src").join("lib.rs"));
    let napi_types = read_to_string_or_panic(&root.join("napi").join("src").join("types.rs"));
    let napi_session_manager =
        read_to_string_or_panic(&root.join("napi").join("src").join("session_manager.rs"));
    for ty in lifted_types {
        let appears_via_use = napi_lib.contains(&format!("codelet_rpc_types::{ty}"))
            || napi_types.contains(&format!("codelet_rpc_types::{ty}"))
            || napi_session_manager.contains(&format!("codelet_rpc_types::{ty}"))
            || napi_lib.contains("pub use codelet_rpc_types::{")
            || napi_types.contains("pub use codelet_rpc_types::{");
        assert!(
            appears_via_use,
            "codelet/napi must re-export {ty} from codelet_rpc_types (look for `pub use codelet_rpc_types::{ty}` or grouped re-export)",
        );
    }
}
