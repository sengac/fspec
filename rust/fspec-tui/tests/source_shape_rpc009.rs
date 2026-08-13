//! Source-shape regression tests added in RPC-009.
//!
//! Feature: spec/features/fspec-tui-cargo-shape-rpc009.feature
//!
//! Widens the RPC-008 source-shape baseline to assert:
//!   - `rust/fspec-tui/Cargo.toml [dependencies]` now lists `tui-input`
//!     alongside the existing RPC-008 entries.
//!   - `rust/Cargo.toml [workspace.dependencies]` declares
//!     `tui-input = "0.10"`.
//!   - Production `[dependencies]` of rust/fspec-tui still excludes
//!     `codelet-napi` and `codelet-core`.
//!   - `[dev-dependencies]` of rust/fspec-tui still excludes
//!     `codelet-napi` (codelet-core stays allowed per RPC-008 Q-DEV-CORE-1).
//!   - New `src/views/*.rs` files preserve the Q9 host-supplied tokio
//!     runtime invariant (no `Runtime::new()` / `Builder` calls).
//!   - New `src/views/*.rs` files do not directly import the
//!     encapsulated transport crates (`codelet_napi`, `codelet_core`,
//!     `tarpc`, `tokio_tungstenite`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

#[test]
fn cargo_toml_dependencies_now_lists_tui_input_alongside_the_existing_entries() {
    // @step Given rust/fspec-tui/Cargo.toml exists
    let raw = common::read_to_string_or_panic(
        &common::workspace_root()
            .join("fspec-tui")
            .join("Cargo.toml"),
    );
    // @step When the test parses the manifest's `[dependencies]` table
    let deps = section_body(&raw, "[dependencies]");
    // @step Then the table contains `tui-input` (consumed via `tui-input.workspace = true`)
    assert!(
        deps.contains("tui-input"),
        "[dependencies] must list tui-input. Got:\n{deps}"
    );
    // @step And the table still contains `codelet-rpc`, `codelet-rpc-types`, `codelet-rpc-embedded`, `codelet-rpc-server`
    for required in [
        "codelet-rpc",
        "codelet-rpc-types",
        "codelet-rpc-embedded",
        "codelet-rpc-server",
    ] {
        assert!(
            deps.contains(required),
            "[dependencies] must list {required}"
        );
    }
    // @step And the table still contains `tokio`, `async-trait`, `futures`, `ratatui`, `crossterm`, `tui-popup`
    for required in [
        "tokio",
        "async-trait",
        "futures",
        "ratatui",
        "crossterm",
        "tui-popup",
    ] {
        assert!(
            deps.contains(required),
            "[dependencies] must list {required}"
        );
    }
    // @step And the table still contains `tokio-tungstenite`, `url`, `tarpc`, `anyhow`, `tracing`
    for required in ["tokio-tungstenite", "url", "tarpc", "anyhow", "tracing"] {
        assert!(
            deps.contains(required),
            "[dependencies] must list {required}"
        );
    }
}

#[test]
fn workspace_cargo_toml_declares_tui_input_0_10() {
    // @step Given rust/Cargo.toml exists
    let raw = common::read_to_string_or_panic(&common::workspace_root().join("Cargo.toml"));
    // @step When the test parses the manifest's `[workspace.dependencies]` table
    let body = section_body(&raw, "[workspace.dependencies]");
    // @step Then the table contains an entry `tui-input = "0.10"` in the `# === Terminal & UI ===` block
    assert!(
        body.contains("tui-input"),
        "[workspace.dependencies] must declare tui-input"
    );
    assert!(
        body.contains("tui-input = \"0.10\""),
        "[workspace.dependencies] tui-input must be pinned to \"0.10\""
    );
}

#[test]
fn production_dependencies_still_excludes_codelet_napi_and_codelet_core() {
    // @step Given rust/fspec-tui/Cargo.toml exists
    let raw = common::read_to_string_or_panic(
        &common::workspace_root()
            .join("fspec-tui")
            .join("Cargo.toml"),
    );
    // @step When the test parses the manifest's `[dependencies]` table
    let deps = section_body(&raw, "[dependencies]");
    // @step Then the table does NOT contain `codelet-napi`
    assert!(
        !contains_dep_key(deps, "codelet-napi"),
        "[dependencies] MUST NOT list codelet-napi"
    );
    // @step And the table does NOT contain `codelet-core`
    assert!(
        !contains_dep_key(deps, "codelet-core"),
        "[dependencies] MUST NOT list codelet-core"
    );
}

#[test]
fn dev_dependencies_still_excludes_codelet_napi() {
    // @step Given rust/fspec-tui/Cargo.toml exists
    let raw = common::read_to_string_or_panic(
        &common::workspace_root()
            .join("fspec-tui")
            .join("Cargo.toml"),
    );
    // @step When the test parses the manifest's `[dev-dependencies]` table
    let dev = section_body(&raw, "[dev-dependencies]");
    // @step Then the table does NOT contain `codelet-napi`
    assert!(
        !contains_dep_key(dev, "codelet-napi"),
        "[dev-dependencies] MUST NOT list codelet-napi"
    );
    // @step And the table MAY contain `codelet-core` (allowed for real-service fixtures per RPC-008 Q-DEV-CORE-1)
    assert!(
        contains_dep_key(dev, "codelet-core"),
        "[dev-dependencies] should list codelet-core (RPC-008 Q-DEV-CORE-1)"
    );
}

#[test]
fn new_src_views_files_preserve_the_host_supplied_tokio_runtime_invariant_q9() {
    // @step Given the source files rust/fspec-tui/src/views/work_units_list.rs, agent_repl.rs, root.rs, footer.rs, and mod.rs exist
    // RPC-012 supersession: work_units_list.rs, agent_repl.rs and root.rs
    // were removed in favour of board.rs, agent.rs and navigator.rs
    // respectively (rule [4] and architecture note [2]).
    // RPC-013 supersession: footer.rs was removed when the 1-row footer
    // moved into each view (rule [1] of rpc013-source-shape.feature).
    // The invariant is preserved across the new set of files.
    let views_dir = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("views");
    for file in ["mod.rs", "board.rs", "agent.rs", "navigator.rs"] {
        let path = views_dir.join(file);
        assert!(path.exists(), "expected {} to exist", path.display());
    }
    // @step When the test scans each file's body
    let rs_files = common::collect_rs_files(&views_dir);
    let forbidden = [
        "tokio::runtime::Builder",
        "runtime::Builder::new_multi_thread",
        "runtime::Builder::new_current_thread",
        "tokio::runtime::Runtime::new",
        "Runtime::new()",
    ];
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        for needle in forbidden {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    // @step Then NO file contains `tokio::runtime::Builder`
    // @step And NO file contains `runtime::Builder::new_multi_thread`
    // @step And NO file contains `runtime::Builder::new_current_thread`
    // @step And NO file contains `tokio::runtime::Runtime::new`
    // @step And NO file contains `Runtime::new()`
    assert!(
        violations.is_empty(),
        "src/views/*.rs MUST preserve Q9 host-supplied runtime invariant. Violations: {violations:?}"
    );
}

#[test]
fn new_src_views_files_do_not_directly_import_the_encapsulated_transport_crates() {
    // @step Given the source files rust/fspec-tui/src/views/*.rs exist
    let views_dir = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("views");
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");
    // @step When the test scans each file's `use` declarations
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        for needle in [
            "codelet_napi::",
            "codelet_core::",
            "tarpc::",
            "tokio_tungstenite::",
        ] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    // @step Then NO file directly imports `codelet_napi::`
    // @step And NO file directly imports `codelet_core::`
    // @step And NO file directly imports `tarpc::`
    // @step And NO file directly imports `tokio_tungstenite::`
    // @step And every interaction with the backend goes through `Arc<dyn FspecBackend>` (or the `FspecBackend` trait surface)
    assert!(
        violations.is_empty(),
        "src/views/*.rs MUST NOT directly import encapsulated transport crates. \
         Backend interactions must go through Arc<dyn FspecBackend>. Violations: {violations:?}"
    );
}

/// Sub-section extractor: returns the substring of `raw` from after
/// `header` up to (but not including) the next `[section]` header.
fn section_body<'a>(raw: &'a str, header: &str) -> &'a str {
    let Some(start) = raw.find(header) else {
        panic!("expected section header `{header}` in Cargo.toml. Got:\n{raw}");
    };
    let after_header = &raw[start + header.len()..];
    if let Some(next) = after_header.find("\n[") {
        &after_header[..next]
    } else {
        after_header
    }
}

/// Returns true iff `section` contains a key line matching `name`,
/// excluding comments and string-literal false positives.
fn contains_dep_key(section: &str, name: &str) -> bool {
    section.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            return false;
        }
        let prefix = trimmed.split('=').next().unwrap_or("").trim();
        let prefix_no_dot = prefix.split('.').next().unwrap_or("").trim();
        prefix_no_dot == name
    })
}
