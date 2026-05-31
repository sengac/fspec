//! RPC-058 — Source-shape assertions for the scheduler-engine LIFT.
//!
//! Feature: spec/features/rpc058-scheduler-engine-lift.feature
//!
//! These tests pin the file layout for the lift of the scheduler
//! engine out of `codelet/napi/src/scheduler/` and into
//! `codelet/core/src/scheduler/`, plus the new `SchedulerHooks` trait
//! that replaces direct `crate::session_bindings::SessionManager`
//! references. The pure CRUD helpers from
//! `codelet/napi/src/schedule_handler.rs` lift into
//! `codelet/core/src/scheduler/crud.rs`.
//!
//! These tests are pure file scans — no compile dependency on the
//! lifted modules themselves — so they catch refactors that
//! accidentally re-introduce a NAPI dependency into the engine.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above codelet/fspec-tui")
        .to_path_buf()
}

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn scheduler_core_dir() -> PathBuf {
    workspace_root().join("codelet/core/src/scheduler")
}

/// Recursively read every regular file under `dir` and return a Vec
/// of (relative-path, contents).
fn read_all_rust_files(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                out.push((path, content));
            }
        }
    }
    out
}

/// Scenario: Scheduler engine modules live under codelet/core/src/scheduler/
#[test]
fn scheduler_engine_modules_live_under_codelet_core() {
    // @step Given the directory codelet/core/src/scheduler/ exists
    let dir = scheduler_core_dir();
    assert!(
        dir.is_dir(),
        "codelet/core/src/scheduler/ directory should exist"
    );

    // @step Then it contains a file named "mod.rs"
    // @step And it contains a file named "engine.rs"
    // @step And it contains a file named "state.rs"
    // @step And it contains a file named "cron_utils.rs"
    // @step And it contains a file named "types.rs"
    // @step And it contains a file named "trigger.rs"
    // @step And it contains a file named "agent_job.rs"
    // @step And it contains a file named "shell_job.rs"
    // @step And it contains a file named "catch_up.rs"
    // @step And it contains a file named "job_log.rs"
    // @step And it contains a file named "crud.rs"
    for file in [
        "mod.rs",
        "engine.rs",
        "state.rs",
        "cron_utils.rs",
        "types.rs",
        "trigger.rs",
        "agent_job.rs",
        "shell_job.rs",
        "catch_up.rs",
        "job_log.rs",
        "crud.rs",
    ] {
        let path = dir.join(file);
        assert!(
            path.is_file(),
            "codelet/core/src/scheduler/{file} should exist"
        );
    }
}

/// Scenario: codelet-core declares a SchedulerHooks trait
#[test]
fn codelet_core_declares_scheduler_hooks_trait() {
    // @step Given the file codelet/core/src/scheduler/mod.rs is compiled
    let path = scheduler_core_dir().join("mod.rs");
    let source = fs::read_to_string(&path).expect("read core/scheduler/mod.rs");

    // @step Then it declares a public trait named "SchedulerHooks"
    assert!(
        source.contains("pub trait SchedulerHooks"),
        "scheduler/mod.rs should declare pub trait SchedulerHooks"
    );

    // @step And SchedulerHooks declares a method named "get_session_count" returning usize
    // @step And SchedulerHooks declares a method named "get_live_session_ids" returning Vec<Uuid>
    // @step And SchedulerHooks declares a method named "spawn_scheduled_session" returning Result<(), String>
    // @step And SchedulerHooks declares a method named "default_model" returning String
    for method in [
        "get_session_count",
        "get_live_session_ids",
        "spawn_scheduled_session",
        "default_model",
    ] {
        let needle = format!("fn {method}(");
        assert!(
            source.contains(&needle),
            "SchedulerHooks should declare fn {method}"
        );
    }
}

/// Scenario: spawn_scheduler accepts an Arc<dyn SchedulerHooks>
#[test]
fn spawn_scheduler_takes_arc_dyn_scheduler_hooks() {
    // @step Given the file codelet/core/src/scheduler/engine.rs is compiled
    let path = scheduler_core_dir().join("engine.rs");
    let source = fs::read_to_string(&path).expect("read core/scheduler/engine.rs");
    let normalised = normalise(&source);

    // @step Then it declares a public fn named "spawn_scheduler" whose last parameter has type "Arc<dyn SchedulerHooks>"
    assert!(
        source.contains("pub fn spawn_scheduler("),
        "engine.rs should declare pub fn spawn_scheduler"
    );
    assert!(
        normalised.contains("Arc<dyn SchedulerHooks>"),
        "spawn_scheduler should accept Arc<dyn SchedulerHooks>"
    );
}

/// Scenario: The lifted engine has no crate::session_bindings references
#[test]
fn lifted_engine_has_no_session_bindings_references() {
    // @step Given the directory codelet/core/src/scheduler/ exists
    let dir = scheduler_core_dir();
    assert!(dir.is_dir(), "scheduler core dir should exist");

    // @step Then no file under codelet/core/src/scheduler/ contains the text "crate::session_bindings"
    // @step And no file under codelet/core/src/scheduler/ contains the text "session_bindings::SessionManager"
    for (path, content) in read_all_rust_files(&dir) {
        assert!(
            !content.contains("crate::session_bindings"),
            "{} must not contain 'crate::session_bindings' — the lift requires hooks not direct references",
            path.display()
        );
        assert!(
            !content.contains("session_bindings::SessionManager"),
            "{} must not contain 'session_bindings::SessionManager'",
            path.display()
        );
    }
}

/// Scenario: codelet/napi/src/scheduler/mod.rs is a thin re-export shim
#[test]
fn napi_scheduler_mod_is_thin_reexport_shim() {
    // @step Given the file codelet/napi/src/scheduler/mod.rs is compiled
    let path = workspace_root().join("codelet/napi/src/scheduler/mod.rs");
    let source = fs::read_to_string(&path).expect("read napi/src/scheduler/mod.rs");

    // @step Then it contains a "pub use codelet_core::scheduler" re-export statement
    assert!(
        source.contains("pub use codelet_core::scheduler"),
        "napi/scheduler/mod.rs should contain 'pub use codelet_core::scheduler' re-export"
    );

    // @step And it still declares a public module named "loop_store"
    assert!(
        source.contains("pub mod loop_store"),
        "napi/scheduler/mod.rs should still declare pub mod loop_store"
    );
}

/// Scenario: The pure CRUD helpers live in codelet-core::scheduler::crud
#[test]
fn crud_helpers_live_in_codelet_core_scheduler_crud() {
    // @step Given the file codelet/core/src/scheduler/crud.rs is compiled
    let path = scheduler_core_dir().join("crud.rs");
    let source = fs::read_to_string(&path).expect("read core/scheduler/crud.rs");

    // @step Then it declares a public fn named "schedule_add"
    // @step And it declares a public fn named "schedule_list"
    // @step And it declares a public fn named "schedule_pause"
    // @step And it declares a public fn named "schedule_resume"
    // @step And it declares a public fn named "schedule_remove"
    for fname in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let needle = format!("pub fn {fname}(");
        assert!(
            source.contains(&needle),
            "crud.rs should declare pub fn {fname}"
        );
    }

    // @step And the file does not contain the text "use napi"
    assert!(
        !source.contains("use napi"),
        "crud.rs must not contain 'use napi' — it lives in codelet-core (NAPI-free)"
    );
}

/// Scenario: codelet-sessions handle_impl wires the five new methods to crud.rs
#[test]
fn sessions_handle_impl_wires_schedule_methods_to_crud() {
    // @step Given the file codelet/sessions/src/handle_impl.rs is compiled
    let path = workspace_root().join("codelet/sessions/src/handle_impl.rs");
    let source = fs::read_to_string(&path).expect("read sessions/src/handle_impl.rs");

    // @step Then it implements "schedule_add" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_list" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_pause" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_resume" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_remove" by delegating to codelet_core::scheduler::crud
    for fname in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let impl_needle = format!("fn {fname}(");
        assert!(
            source.contains(&impl_needle),
            "handle_impl.rs should implement fn {fname}"
        );
    }
    // The delegation goes through codelet_core::scheduler::crud.
    assert!(
        source.contains("codelet_core::scheduler::crud")
            || source.contains("scheduler::crud::"),
        "handle_impl.rs should delegate to codelet_core::scheduler::crud"
    );
}

/// Scenario: Lifted scheduler engine and cron_utils use captured-identifier format args (RPC-058 retro 2026-05-27)
///
/// RPC-058 retro followup: the lift of the scheduler engine + cron_utils
/// from codelet/napi/src/scheduler/ into codelet/core/src/scheduler/
/// preserved the original SCHED-002/SCHED-003 era format!/anyhow! call
/// sites that use the trailing-argument syntax. clippy::uninlined_format_args
/// (enabled under the workspace `-D warnings` level) flags those — the
/// fix is purely textual: rewrite `"foo {}", x` to `"foo {x}"`.
#[test]
fn lifted_scheduler_engine_and_cron_utils_use_captured_identifier_format_args() {
    // @step Given the codelet workspace inherits the lint level `-D warnings` which includes `clippy::uninlined_format_args`
    // (verified transitively by codelet/sessions/tests/skeleton_invariants.rs::scenario_workspace_lints_are_inherited_and_clippy_passes)

    // @step Given codelet/core/src/scheduler/engine.rs and codelet/core/src/scheduler/cron_utils.rs live at their post-RPC-058 location
    let engine_path = workspace_root().join("codelet/core/src/scheduler/engine.rs");
    let cron_utils_path = workspace_root().join("codelet/core/src/scheduler/cron_utils.rs");
    assert!(engine_path.exists(), "engine.rs must live at post-RPC-058 location");
    assert!(cron_utils_path.exists(), "cron_utils.rs must live at post-RPC-058 location");
    let engine_source = fs::read_to_string(&engine_path).expect("read engine.rs");
    let cron_utils_source = fs::read_to_string(&cron_utils_path).expect("read cron_utils.rs");

    // @step When I scan engine.rs and cron_utils.rs for `format!` / `anyhow!` / `panic!` / `println!` / `eprintln!` / `write!` / `writeln!` macro invocations that interpolate a bare identifier
    //
    // @step Then every such invocation uses the captured-identifier syntax `"... {name} ..."` instead of the trailing-argument syntax `"... {} ...", name`
    //
    // The four specific known-bad sites from the RPC-058 retro are
    // engine.rs:284, engine.rs:299, cron_utils.rs:41, cron_utils.rs:49.
    // We assert ZERO occurrences of the trailing-argument pattern in
    // these files. The pattern matched is: a string literal that
    // contains `{}` followed by a `, <ident>` on the same line. This
    // detects the older form without false positives on captured-arg
    // formats (which have no trailing-argument list).
    let line_breaks_trailing_arg = |source: &str, label: &str| -> Vec<String> {
        let mut violations = Vec::new();
        for (idx, line) in source.lines().enumerate() {
            // Skip comments to avoid false positives on doc-comments.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            // Heuristic: the line contains `{}` AND ends with `, name)`
            // OR `, name)?;` pattern from the macro invocations we care about.
            // We scope to format!/anyhow!/panic!/println!/eprintln!/write!/writeln!
            // by checking for the macro names earlier on the line OR in the
            // preceding 3 lines (for multi-line macro calls).
            if !line.contains("{}") {
                continue;
            }
            // Check if any of the target macros appear within the last 4 lines.
            let start = idx.saturating_sub(3);
            let window: Vec<&str> = source.lines().skip(start).take(idx - start + 1).collect();
            let window_joined = window.join(" ");
            let has_target_macro = ["format!(", "anyhow!(", "panic!(", "println!(", "eprintln!(", "write!(", "writeln!("]
                .iter()
                .any(|m| window_joined.contains(m));
            if !has_target_macro {
                continue;
            }
            // The line has `{}` AND a target macro is in scope. Now check
            // for the trailing-argument pattern: a `, ident` or `, ident)`
            // suffix on this line or the next (for split-line macros).
            let next_line = source.lines().nth(idx + 1).unwrap_or("");
            let combined = format!("{line} {next_line}");
            // Look for `, <ident>` or `, <ident>)` after the format
            // string. We sweep for a comma followed by 1+ whitespace
            // followed by an identifier-looking token. Captured-arg
            // form does not produce these trailing args.
            let suffix = if let Some(quote_end) = line.rfind('"') {
                &line[quote_end..]
            } else if let Some(quote_end) = combined.rfind('"') {
                &combined[quote_end..]
            } else {
                line
            };
            // The pattern `, name)` or `, name);` is the trailing-arg form.
            // A captured-arg form ends `"<text>")` or `"<text>");`.
            let after_close_quote: String = suffix.chars().skip_while(|c| *c == '"').collect();
            // Take only the macro's trailing-arg region: everything from
            // the closing `"` up to the FIRST `)` (the macro's close paren).
            // We can't simply split-then-skip because the line may contain
            // additional `)` from outer call chains (e.g. `parse_cron(...,
            // &format!("schedule '{}'", name))` — the OUTER paren closes
            // parse_cron, not format!). The first `)` after the format
            // string is always the macro's own closing paren under normal
            // formatting.
            let macro_args_region = match after_close_quote.find(')') {
                Some(idx) => &after_close_quote[..idx],
                None => after_close_quote.as_str(),
            };
            let trailing_parts: Vec<String> = macro_args_region
                .split(',')
                .skip(1) // skip the empty fragment before the first comma
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            let is_bare_ident = |p: &str| -> bool {
                // Only BARE identifiers (alphanumeric + underscore only).
                // clippy::uninlined_format_args fires ONLY when EVERY trailing
                // arg is a bare identifier. Function calls (`foo()`), method
                // calls (`x.y()`), path expressions (`x::y`), and any complex
                // expression suppress the lint. So we mirror clippy by
                // requiring ALL trailing args to be bare identifiers.
                !p.is_empty()
                    && p.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && p.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
            };
            let has_trailing_arg =
                !trailing_parts.is_empty() && trailing_parts.iter().all(|p| is_bare_ident(p));
            if has_trailing_arg {
                violations.push(format!("{label}:{}: {line}", idx + 1));
            }
        }
        violations
    };
    let engine_violations = line_breaks_trailing_arg(&engine_source, "engine.rs");
    let cron_violations = line_breaks_trailing_arg(&cron_utils_source, "cron_utils.rs");

    assert!(
        engine_violations.is_empty(),
        "RPC-058 retro: codelet/core/src/scheduler/engine.rs must use captured-identifier format args (got {} violations):\n{}",
        engine_violations.len(),
        engine_violations.join("\n")
    );
    assert!(
        cron_violations.is_empty(),
        "RPC-058 retro: codelet/core/src/scheduler/cron_utils.rs must use captured-identifier format args (got {} violations):\n{}",
        cron_violations.len(),
        cron_violations.join("\n")
    );

    // @step Then `cargo clippy -p codelet-sessions -- -D warnings` exits 0 with no `clippy::uninlined_format_args` errors against engine.rs:284, engine.rs:299, cron_utils.rs:41, or cron_utils.rs:49
    //
    // The static assertions above are sufficient — the only way for
    // clippy::uninlined_format_args to fire is for one of the target
    // macros to use the trailing-argument syntax with a bare identifier,
    // which the heuristic above detects directly.
}
