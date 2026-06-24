//! RPC-075: Source-shape + behavioural-parity regression tests pinning
//! the `clippy::uninlined_format_args` fix in
//! `codelet/core/src/scheduler/{agent_job,shell_job}.rs`.
//!
//! Feature: spec/features/scheduler-format-args-clippy-compliance.feature
//!
//! Pattern mirrors the source-shape helpers from
//! `codelet/sessions/tests/mcp_injection_source_shape.rs` (RPC-062) and
//! `codelet/sessions/tests/rpc073_list_providers_wiring.rs` (RPC-073).
//!
//! Before the RPC-075 fix, both files contained legacy positional
//! `format!("...{}...", var)` / `anyhow!("...{}...", var)` calls that
//! triggered `error: variables can be used directly in the format!
//! string` against the workspace-wide
//! `-D clippy::uninlined_format_args` lint level, which in turn made
//! `codelet/sessions/tests/skeleton_invariants.rs::scenario_workspace_lints_are_inherited_and_clippy_passes`
//! fail.
//!
//! After the fix, every `format!` / `anyhow!` invocation in the two
//! scheduler files uses inline-capture form (`{name}`, `{timestamp}`,
//! `{e}`, etc.). The output strings are byte-identical to the legacy
//! positional form — Rust's `Display` impl produces the same bytes
//! for both spellings.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// This test file *intentionally* contains a legacy positional
// `format!("...{}...", var, var)` call inside
// `agent_job_session_name_inline_capture_matches_legacy_positional_form`
// in order to assert byte-equivalence between the two spellings. We
// therefore opt-out of `uninlined_format_args` at the file level — it
// is the production code in `codelet-core/src/scheduler/` that this
// card is enforcing the lint against, not this regression test.
#![allow(clippy::uninlined_format_args)]

use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Path / read helpers — sibling of mcp_injection_source_shape.rs's set
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

/// Return a Vec of "offender" lines: the comment-stripped source
/// lines that contain a `format!(` or `anyhow!(` invocation AND
/// contain a positional `{}` placeholder. After the RPC-075 fix
/// this Vec must be empty for both scheduler source files.
fn legacy_positional_format_args(src: &str) -> Vec<String> {
    let stripped = strip_rust_comments(src);
    stripped
        .lines()
        .filter(|line| {
            (line.contains("format!(") || line.contains("anyhow!(")) && line.contains("{}")
        })
        .map(|line| line.trim().to_string())
        .collect()
}

fn agent_job_path() -> PathBuf {
    workspace_root()
        .join("core")
        .join("src")
        .join("scheduler")
        .join("agent_job.rs")
}

fn shell_job_path() -> PathBuf {
    workspace_root()
        .join("core")
        .join("src")
        .join("scheduler")
        .join("shell_job.rs")
}

// =============================================================================
// Scenario: scheduler/agent_job.rs uses inline-capture format args exclusively
// =============================================================================

#[test]
fn scheduler_agent_job_uses_inline_format_args_exclusively() {
    // @step Given the file `codelet/core/src/scheduler/agent_job.rs` exists in the workspace
    let path = agent_job_path();
    assert!(
        path.exists(),
        "expected agent_job.rs at {} — has the scheduler module moved?",
        path.display()
    );
    let body = read(&path);

    // @step When I scan every `format!(` and `anyhow!(` invocation in that file
    let offenders = legacy_positional_format_args(&body);

    // @step Then no invocation contains a positional `{}` placeholder followed by a comma-separated argument list
    // @step And every formatted variable appears inline inside the format string (e.g. `{name}`, `{timestamp}`, `{e}`)
    assert!(
        offenders.is_empty(),
        "agent_job.rs still contains legacy positional `{{}}` format args — \
         RPC-075 requires inline-capture form ({{name}} / {{timestamp}} / {{e}}). \
         Offenders:\n  {}",
        offenders.join("\n  ")
    );
}

// =============================================================================
// Scenario: scheduler/shell_job.rs uses inline-capture format args exclusively
// =============================================================================

#[test]
fn scheduler_shell_job_uses_inline_format_args_exclusively() {
    // @step Given the file `codelet/core/src/scheduler/shell_job.rs` exists in the workspace
    let path = shell_job_path();
    assert!(
        path.exists(),
        "expected shell_job.rs at {} — has the scheduler module moved?",
        path.display()
    );
    let body = read(&path);

    // @step When I scan every `format!(` and `anyhow!(` invocation in that file
    let offenders = legacy_positional_format_args(&body);

    // @step Then no invocation contains a positional `{}` placeholder followed by a comma-separated argument list
    // @step And every formatted variable appears inline inside the format string (e.g. `{name}`, `{command}`, `{e}`)
    assert!(
        offenders.is_empty(),
        "shell_job.rs still contains legacy positional `{{}}` format args — \
         RPC-075 requires inline-capture form ({{name}} / {{command}} / {{e}}). \
         Offenders:\n  {}",
        offenders.join("\n  ")
    );
}

// =============================================================================
// Scenario: cargo clippy on codelet-core passes with -D warnings for the scheduler module
// =============================================================================
//
// We invoke clippy directly on `codelet-core` and then filter the
// diagnostic stream for `uninlined_format_args` errors that point at
// either of the two scheduler files. This scopes the assertion to the
// fix area without making the test failures of *other* clippy lints
// elsewhere in `codelet-core` (e.g. an unused import in a different
// module that is out-of-scope for RPC-075) cause this test to fail.

fn workspace_cargo_toml() -> PathBuf {
    workspace_root().join("Cargo.toml")
}

#[test]
fn cargo_clippy_on_codelet_core_emits_no_uninlined_format_args_for_scheduler_files() {
    // @step Given the workspace lint set denies `clippy::uninlined_format_args`
    let workspace = workspace_cargo_toml();
    assert!(
        workspace.exists(),
        "expected workspace Cargo.toml at {} — has the workspace layout moved?",
        workspace.display()
    );
    let manifest = read(&workspace);
    assert!(
        manifest.contains("uninlined_format_args"),
        "workspace Cargo.toml must declare a clippy lint level for \
         `uninlined_format_args` (RPC-075 invariant). Found manifest:\n{manifest}"
    );

    // @step And the scheduler module uses inline-capture format args
    let agent = read(&agent_job_path());
    let shell = read(&shell_job_path());
    assert!(
        legacy_positional_format_args(&agent).is_empty(),
        "precondition failed: agent_job.rs still has legacy positional format args"
    );
    assert!(
        legacy_positional_format_args(&shell).is_empty(),
        "precondition failed: shell_job.rs still has legacy positional format args"
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

    // @step Then no `uninlined_format_args` diagnostic is emitted against `core/src/scheduler/agent_job.rs`
    assert!(
        !stderr_has_scheduler_format_args_violation(&stderr, "agent_job.rs"),
        "cargo clippy reported uninlined_format_args against agent_job.rs:\n{stderr}"
    );

    // @step And no `uninlined_format_args` diagnostic is emitted against `core/src/scheduler/shell_job.rs`
    assert!(
        !stderr_has_scheduler_format_args_violation(&stderr, "shell_job.rs"),
        "cargo clippy reported uninlined_format_args against shell_job.rs:\n{stderr}"
    );
}

/// Returns `true` iff the clippy stderr stream contains a
/// `uninlined_format_args` diagnostic whose source-pointer (the `-->`
/// line) names the given scheduler file under
/// `core/src/scheduler/<file>`.
fn stderr_has_scheduler_format_args_violation(stderr: &str, file: &str) -> bool {
    // Walk the stream looking for an `uninlined_format_args` mention
    // followed (within a small window of lines) by a `--> core/src/scheduler/<file>` pointer.
    let needle_lint = "uninlined_format_args";
    let needle_path = format!("core/src/scheduler/{file}");
    let lines: Vec<&str> = stderr.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(needle_lint) {
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
                if prior.contains(needle_lint) {
                    return true;
                }
            }
        }
    }
    false
}

// =============================================================================
// Scenario: format! output strings are byte-identical to the legacy positional form
// =============================================================================

#[test]
fn agent_job_session_name_inline_capture_matches_legacy_positional_form() {
    // @step Given a schedule name of "nightly" and a timestamp of "2026-05-28T00:00:00Z"
    let name = "nightly";
    let timestamp = "2026-05-28T00:00:00Z";

    // @step When the agent_job inline-capture form `format!("[scheduled] {name} — {timestamp}")` is evaluated
    let inline = format!("[scheduled] {name} — {timestamp}");

    // @step Then the resulting string equals `[scheduled] nightly — 2026-05-28T00:00:00Z`
    assert_eq!(inline, "[scheduled] nightly — 2026-05-28T00:00:00Z");

    // @step And the string is byte-identical to the legacy positional form `format!("[scheduled] {} — {}", "nightly", "2026-05-28T00:00:00Z")`
    let positional = format!("[scheduled] {} — {}", name, timestamp);
    assert_eq!(
        inline.as_bytes(),
        positional.as_bytes(),
        "inline-capture and positional `format!` produce divergent bytes — \
         Rust's Display impl invariant has been violated (this should be impossible)"
    );
}
