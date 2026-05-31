//! RPC-076: Source-shape regression tests pinning the
//! `clippy::unused_imports` fix in
//! `codelet/core/src/session_manager_handle.rs`.
//!
//! Feature: spec/features/session-manager-handle-imports-clippy-compliance.feature
//!
//! Pattern mirrors `codelet/sessions/tests/rpc075_scheduler_format_args_shape.rs`
//! (sibling card RPC-075 — same workspace-lint failure mode, different file).
//!
//! Before the RPC-076 fix, `codelet/core/src/session_manager_handle.rs`
//! contained an unused `NotificationSeverity` symbol in its
//! `use codelet_rpc_types::{...}` block. The symbol had been added by
//! WIP that intended to broadcast a `UserNotification` chunk for /clear
//! but the chunk path was subsequently removed to match the TypeScript
//! reference (see git diff: "The previous `UserNotification { message:
//! ... }` broadcast for /clear was a Rust-side invention with no
//! counterpart in the TypeScript reference… and has been removed").
//! The stray import triggered
//! `error: unused import: \`NotificationSeverity\`` under the
//! workspace-wide `-D warnings`, which in turn made
//! `codelet/sessions/tests/skeleton_invariants.rs::scenario_workspace_lints_are_inherited_and_clippy_passes`
//! fail.
//!
//! After the fix, the symbol is no longer in the use-block (or, if a
//! future card legitimately re-introduces it, it has at least one
//! matching use-site in the same file). The regression test pins
//! both invariants so subsequent WIP can't silently re-create the
//! same defect.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Path / read helpers — mirror of rpc075_scheduler_format_args_shape.rs
// =============================================================================

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Strip `//` line comments and `/* ... */` block comments from
/// Rust source so that substring scans don't get fooled by needle
/// references inside doc comments.
fn strip_rust_comments(src: &str) -> String {
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

fn session_manager_handle_path() -> PathBuf {
    workspace_root()
        .join("core")
        .join("src")
        .join("session_manager_handle.rs")
}

fn workspace_cargo_toml() -> PathBuf {
    workspace_root().join("Cargo.toml")
}

/// Extract the body of the `use codelet_rpc_types::{...}` block from
/// the comment-stripped source. Returns the substring between the
/// opening `{` and its matching closing `}` (exclusive on both ends).
/// Returns `None` if the block is absent.
fn extract_codelet_rpc_types_use_block(stripped_src: &str) -> Option<String> {
    let needle = "use codelet_rpc_types::{";
    let start = stripped_src.find(needle)?;
    let after_open = start + needle.len();
    let rest = &stripped_src[after_open..];
    let mut depth = 1usize;
    for (i, ch) in rest.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[..i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Tokenise a use-block body into a flat Vec of identifiers, skipping
/// whitespace, commas, leading-keyword `pub`, and nested-group braces.
/// Good enough for "does the bare identifier `NotificationSeverity`
/// appear in this list" — we don't have to be a full Rust parser.
fn use_block_identifiers(body: &str) -> Vec<String> {
    body.split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "{" && *s != "}" && *s != "pub")
        .map(std::string::ToString::to_string)
        .collect()
}

// =============================================================================
// Scenario: session_manager_handle.rs use-block does not import NotificationSeverity
// =============================================================================

#[test]
fn session_manager_handle_use_block_does_not_import_notification_severity() {
    // @step Given the file `codelet/core/src/session_manager_handle.rs` exists in the workspace
    let path = session_manager_handle_path();
    assert!(
        path.exists(),
        "expected session_manager_handle.rs at {} — has codelet-core relocated?",
        path.display()
    );
    let body = read(&path);
    let stripped = strip_rust_comments(&body);

    // @step When I scan the `use codelet_rpc_types::{...}` import block at the top of that file
    let use_body = extract_codelet_rpc_types_use_block(&stripped).unwrap_or_else(|| {
        panic!(
            "expected a `use codelet_rpc_types::{{...}}` block in {} — \
             has the import shape changed?",
            path.display()
        )
    });
    let identifiers = use_block_identifiers(&use_body);

    // @step Then the symbol `NotificationSeverity` does not appear in the import list
    assert!(
        !identifiers.iter().any(|id| id == "NotificationSeverity"),
        "session_manager_handle.rs `use codelet_rpc_types::{{...}}` block still \
         imports `NotificationSeverity` but the symbol is not used anywhere in the \
         file — RPC-076 requires removing the unused import. Imports observed:\n  \
         {}",
        identifiers.join(", ")
    );

    // @step Then the symbols `PauseState` and `ProviderInfo` continue to appear in the import list
    assert!(
        identifiers.iter().any(|id| id == "PauseState"),
        "session_manager_handle.rs `use codelet_rpc_types::{{...}}` block must \
         continue to import `PauseState` (the import surface was widened in WIP \
         to cover the RPC-037 trait expansion). Imports observed:\n  {}",
        identifiers.join(", ")
    );
    assert!(
        identifiers.iter().any(|id| id == "ProviderInfo"),
        "session_manager_handle.rs `use codelet_rpc_types::{{...}}` block must \
         continue to import `ProviderInfo`. Imports observed:\n  {}",
        identifiers.join(", ")
    );
}

// =============================================================================
// Scenario: Source-shape regression test pins the absence of unused NotificationSeverity import
// =============================================================================
//
// This is the more *permissive* invariant: a future card may legitimately
// bring `NotificationSeverity` back in — but only if it also adds at
// least one use-site for it in the same file. We assert that the symbol
// either does not appear at all, or its occurrences in the use-block
// are matched by additional occurrences elsewhere in the file.

#[test]
fn notification_severity_is_never_unused_in_session_manager_handle() {
    // @step Given the file `codelet/core/src/session_manager_handle.rs` exists in the workspace
    let path = session_manager_handle_path();
    assert!(path.exists(), "{} must exist", path.display());
    let body = read(&path);
    let stripped = strip_rust_comments(&body);

    // @step When I scan the file for occurrences of the identifier `NotificationSeverity`
    let total_occurrences = stripped.matches("NotificationSeverity").count();
    let in_use_block: usize = extract_codelet_rpc_types_use_block(&stripped)
        .map(|body| {
            use_block_identifiers(&body)
                .into_iter()
                .filter(|id| id == "NotificationSeverity")
                .count()
        })
        .unwrap_or(0);

    // @step Then either the identifier does not appear at all, or every occurrence inside the `use codelet_rpc_types::{...}` block is matched by at least one use-site elsewhere in the same file
    assert!(
        total_occurrences == 0 || total_occurrences > in_use_block,
        "session_manager_handle.rs references `NotificationSeverity` {total_occurrences} \
         time(s), all of which are inside the `use codelet_rpc_types::{{...}}` \
         block ({in_use_block}). That means the symbol is imported but never used \
         — `cargo clippy -D warnings` will reject it. Either drop the import or \
         add a use-site."
    );
}

// =============================================================================
// Scenario: cargo clippy on codelet-core emits no unused_imports diagnostic against session_manager_handle.rs
// =============================================================================
//
// We invoke clippy directly on `codelet-core` and then filter the
// diagnostic stream for `unused_imports` errors that point at
// session_manager_handle.rs. This scopes the assertion to the fix area
// without making the test failures of *other* clippy lints elsewhere
// in `codelet-core` cause this test to fail. Same pattern as the
// sibling RPC-075 test scoped to scheduler files.

#[test]
fn cargo_clippy_on_codelet_core_emits_no_unused_imports_against_session_manager_handle() {
    // @step Given the workspace lint set denies `unused_imports` (implied by `-D warnings`)
    let workspace = workspace_cargo_toml();
    assert!(
        workspace.exists(),
        "expected workspace Cargo.toml at {} — has the workspace layout moved?",
        workspace.display()
    );

    // @step When I run `cargo clippy -p codelet-core --all-targets -- -D warnings`
    let output = Command::new(env!("CARGO"))
        .args([
            "clippy",
            "-p",
            "codelet-core",
            "--all-targets",
            "--manifest-path",
        ])
        .arg(&workspace)
        .args(["--", "-D", "warnings"])
        .output()
        .expect("cargo clippy must run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // @step Then no `unused_imports` diagnostic is emitted against `core/src/session_manager_handle.rs`
    assert!(
        !stderr_has_unused_imports_violation(&stderr, "session_manager_handle.rs"),
        "cargo clippy reported unused_imports against session_manager_handle.rs:\n{stderr}"
    );
}

/// Returns `true` iff the clippy stderr stream contains an
/// `unused_imports` / `unused-imports` diagnostic whose source-pointer
/// (the `-->` line) names the given file under `core/src/<file>`.
fn stderr_has_unused_imports_violation(stderr: &str, file: &str) -> bool {
    let needle_lint_a = "unused_imports";
    let needle_lint_b = "unused import";
    let needle_path = format!("core/src/{file}");
    let lines: Vec<&str> = stderr.lines().collect();
    let mentions_lint = |line: &str| line.contains(needle_lint_a) || line.contains(needle_lint_b);

    for (i, line) in lines.iter().enumerate() {
        if mentions_lint(line) {
            let window_end = (i + 6).min(lines.len());
            for follow in &lines[i..window_end] {
                if follow.contains(&needle_path) {
                    return true;
                }
            }
        }
        if line.contains(&needle_path) {
            let window_start = i.saturating_sub(6);
            for prior in &lines[window_start..=i] {
                if mentions_lint(prior) {
                    return true;
                }
            }
        }
    }
    false
}

// =============================================================================
// Scenario: codelet-core skeleton-invariants workspace-lint precondition: codelet-core itself passes -D warnings
// =============================================================================
//
// The `skeleton_invariants::scenario_workspace_lints_are_inherited_and_clippy_passes`
// test in codelet/sessions/tests/ invokes
// `cargo clippy -p codelet-sessions --all-targets -- -D warnings`. Before
// RPC-076, the *codelet-core* portion of that build (codelet-sessions
// depends on codelet-core) failed with `error: unused import:
// NotificationSeverity`. RPC-076's deliverable is to clean that
// upstream-dependency violation up so the codelet-core layer of the
// skeleton-invariants build is green.
//
// Other clippy violations in codelet-sessions itself (e.g. the current
// `redundant_clone` errors in `sessions/src/handle_impl.rs`) are scoped
// to follow-up cards per the "no scope creep" rule that RPC-075 used to
// hand RPC-076 off as its own card. We assert the *codelet-core*
// invariant here directly; the skeleton-invariants test will go green
// once the remaining follow-up cards land.

#[test]
fn codelet_core_workspace_lints_pass_with_session_manager_handle_clean() {
    // @step Given the codelet/Cargo.toml workspace lints declaration is inherited by codelet-core
    let core_manifest = workspace_root().join("core").join("Cargo.toml");
    assert!(
        core_manifest.exists(),
        "expected codelet/core/Cargo.toml at {}",
        core_manifest.display()
    );
    let manifest_body = read(&core_manifest);
    assert!(
        manifest_body.contains("[lints]"),
        "codelet/core/Cargo.toml must declare a [lints] section"
    );
    let has_workspace_lints = manifest_body.lines().any(|l| {
        let t = l.trim();
        t.starts_with("workspace") && t.contains("= true")
    });
    assert!(
        has_workspace_lints,
        "codelet/core/Cargo.toml [lints] must inherit `workspace = true` \
         (otherwise the unused_imports deny doesn't reach codelet-core)"
    );

    // @step Given session_manager_handle.rs no longer imports `NotificationSeverity`
    let handle_body = read(&session_manager_handle_path());
    let handle_stripped = strip_rust_comments(&handle_body);
    let use_body = extract_codelet_rpc_types_use_block(&handle_stripped)
        .expect("session_manager_handle.rs must contain a `use codelet_rpc_types::{...}` block");
    let identifiers = use_block_identifiers(&use_body);
    assert!(
        !identifiers.iter().any(|id| id == "NotificationSeverity"),
        "precondition failed: session_manager_handle.rs still imports `NotificationSeverity`"
    );

    // @step When I run `cargo clippy -p codelet-core --all-targets -- -D warnings`
    let output = Command::new(env!("CARGO"))
        .args([
            "clippy",
            "-p",
            "codelet-core",
            "--all-targets",
            "--manifest-path",
        ])
        .arg(workspace_cargo_toml())
        .args(["--", "-D", "warnings"])
        .output()
        .expect("cargo clippy on codelet-core must run");

    // @step Then the command exits 0 with no errors
    assert!(
        output.status.success(),
        "cargo clippy -p codelet-core failed (RPC-076 invariant: the \
         codelet-core layer of the skeleton-invariants clippy build must \
         be clean even if sibling crates still have follow-up clippy \
         work):\nstderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
