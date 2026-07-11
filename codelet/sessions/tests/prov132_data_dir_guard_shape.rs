//! PROV-132: Deterministic test isolation for the process-global data
//! directory / default-model state.
//!
//! Feature: spec/features/sessions-global-data-dir-test-isolation.feature
//!
//! ROOT CAUSE (confirmed): `codelet_common::set_data_directory` writes a single
//! process-global `static DATA_DIRECTORY: Mutex<Option<PathBuf>>`
//! (codelet/common/src/data_dir.rs). `SessionManager::new()` eagerly loads
//! `<data_dir>/default-model.json` at construction. When the multi_thread
//! `#[tokio::test]`s in a single integration binary run in parallel, one test
//! swaps the global data-dir pointer out from under another mid-flight, so the
//! racing `SessionManager::new()` loads a *foreign* test's persisted default
//! model. That makes both
//! `rpc081::restore_messages_returns_err_on_malformed_envelope_json` and
//! `prov101::create_session_declines_when_no_default_model`
//! fail on the FIRST full-suite run then pass on re-run / in isolation.
//!
//! FIX (proven in-tree pattern, PROV-118/119/123/129/130): each offending test
//! file declares a file-scoped `static DATA_DIR_GUARD: Mutex<()>` and every
//! `#[test]` that touches the global data dir locks it (poison-tolerantly) and
//! holds the guard across its critical section, serialising the swap+construct.
//!
//! This is a SOURCE-SHAPE regression test (mirrors
//! rpc076_session_manager_handle_imports_shape.rs): it pins the guard contract
//! statically so the flake cannot silently regress. It FAILS before the fix
//! (the offending files have no guard) and PASSES after.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

// =============================================================================
// Path / read helpers — mirror of rpc076_session_manager_handle_imports_shape.rs
// =============================================================================

/// The `codelet/sessions` crate directory (the manifest dir of this crate).
fn sessions_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tests_dir() -> PathBuf {
    sessions_crate_dir().join("tests")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Strip `//` line comments and `/* ... */` block comments from Rust source so
/// substring scans don't get fooled by needle references inside doc comments.
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

/// Split a comment-stripped Rust source file into its top-level `fn` bodies,
/// keyed by function name. Returns a Vec of `(fn_name, is_test, body)` in source
/// order, where `is_test` is true when the function is annotated with a `#[test]`
/// or `#[tokio::test...]` attribute immediately preceding it. Only test
/// functions establish a guard boundary; plain helpers are serialized
/// transitively via their (guarded) callers, so callers filter on `is_test`.
///
/// This is a brace-matcher good enough for well-formed test files: it finds
/// each `fn NAME(` occurrence, walks to the opening `{` of the body, and
/// captures up to the matching `}`.
fn extract_fn_bodies(stripped_src: &str) -> Vec<(String, bool, String)> {
    let bytes = stripped_src.as_bytes();
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = stripped_src[search_from..].find("fn ") {
        let fn_kw = search_from + rel;
        // Guard against matching `fn` inside an identifier (e.g. `my_fn `).
        let prev_ok = fn_kw == 0
            || !stripped_src.as_bytes()[fn_kw - 1].is_ascii_alphanumeric()
                && stripped_src.as_bytes()[fn_kw - 1] != b'_';
        let name_start = fn_kw + 3;
        // Parse the identifier.
        let mut j = name_start;
        while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            j += 1;
        }
        let name = stripped_src[name_start..j].to_string();
        // Find the body opening brace after the parameter/return clause.
        let brace_open = stripped_src[j..].find('{').map(|p| j + p);
        if !prev_ok || name.is_empty() || brace_open.is_none() {
            search_from = name_start.max(fn_kw + 3);
            continue;
        }
        let open = brace_open.expect("checked is_some above");
        // Look back over the ~200 chars before `fn` for a test attribute in the
        // function's attribute block (`#[test]` / `#[tokio::test(...)]`).
        let lookback_start = fn_kw.saturating_sub(200);
        let preamble = &stripped_src[lookback_start..fn_kw];
        let is_test = preamble.contains("#[test]") || preamble.contains("#[tokio::test");
        // Walk to the matching close brace.
        let mut depth = 0usize;
        let mut end = open;
        for (k, ch) in stripped_src[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + k;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = stripped_src[open..=end].to_string();
        out.push((name, is_test, body));
        search_from = end + 1;
    }
    out
}

/// The two integration test files this card is fixing. Both mutate the
/// process-global data dir and were the confirmed flake sources.
fn offending_test_files() -> Vec<PathBuf> {
    vec![
        tests_dir().join("rpc081_restore_session_messages.rs"),
        tests_dir().join("prov101_no_selection_fallbacks.rs"),
    ]
}

/// Does the function body (or a helper it calls that is inlined into the same
/// file) reach the global data dir? We approximate "touches the global" by the
/// presence of a `set_data_directory(` call OR a call to a file-local seed
/// helper (`manager_with_seeded_cache` / `seed_models_cache`) that itself calls
/// `set_data_directory`. Both offending files use one of these forms.
fn body_touches_global_data_dir(body: &str) -> bool {
    body.contains("set_data_directory(")
        || body.contains("manager_with_seeded_cache(")
        || body.contains("seed_models_cache(")
        || body.contains("fresh_session(")
}

/// Does the function body acquire the file-scoped `DATA_DIR_GUARD`?
fn body_acquires_guard(body: &str) -> bool {
    body.contains("DATA_DIR_GUARD")
}

/// Is the file's guard acquisition poison-tolerant (rule 3)? Two equivalent
/// forms satisfy it:
///   * a `std::sync::Mutex` locked via `PoisonError::into_inner`, or
///   * a `tokio::sync::Mutex` (constructed with `Mutex::const_new`), which is
///     inherently poison-free — a panicking task simply releases the guard
///     without poisoning it, so a sibling can still acquire it.
///
/// A bare `std::sync::Mutex` `.lock().unwrap()` is NOT tolerant and fails here.
fn guard_is_poison_tolerant(stripped_src: &str) -> bool {
    stripped_src.contains("PoisonError::into_inner") || stripped_src.contains("Mutex::const_new")
}

// =============================================================================
// Scenario: rpc081 restore-messages tests serialize global data-dir access via
// a poison-tolerant guard
// =============================================================================

#[test]
fn rpc081_restore_messages_tests_hold_data_dir_guard() {
    // @step Given the rpc081_restore_session_messages integration test file declares a file-scoped DATA_DIR_GUARD Mutex
    let path = tests_dir().join("rpc081_restore_session_messages.rs");
    assert!(
        path.exists(),
        "expected rpc081 test file at {} — has it relocated?",
        path.display()
    );
    let stripped = strip_rust_comments(&read(&path));
    assert!(
        stripped.contains("static DATA_DIR_GUARD"),
        "rpc081_restore_session_messages.rs must declare a file-scoped \
         `static DATA_DIR_GUARD: Mutex<()>` to serialize global data-dir access \
         (PROV-132). None found."
    );

    // @step When any restore-messages test acquires the guard before calling set_data_directory
    let unguarded: Vec<String> = extract_fn_bodies(&stripped)
        .into_iter()
        .filter(|(_, is_test, _)| *is_test)
        .filter(|(_, _, body)| body_touches_global_data_dir(body))
        .filter(|(_, _, body)| !body_acquires_guard(body))
        .map(|(name, _, _)| name)
        .collect();

    // @step Then the guard is locked poison-tolerantly via PoisonError::into_inner
    assert!(
        guard_is_poison_tolerant(&stripped),
        "rpc081 guard acquisition must be poison-tolerant — either a \
         `std::sync::Mutex` locked via `PoisonError::into_inner`, or an \
         inherently poison-free `tokio::sync::Mutex` (`Mutex::const_new`) — so a \
         panicking test does not cascade-fail its serialized siblings."
    );

    // @step And the guard is bound to a live _guard binding held across the whole test body
    assert!(
        unguarded.is_empty(),
        "these rpc081 tests touch the process-global data directory but do NOT \
         hold DATA_DIR_GUARD across their body — they can race a sibling's \
         set_data_directory and load a foreign default-model.json (PROV-132):\n  {}",
        unguarded.join("\n  ")
    );
    assert!(
        stripped.contains("let _guard = DATA_DIR_GUARD"),
        "rpc081 tests must bind the guard to a live `let _guard = DATA_DIR_GUARD` \
         binding so it is held for the whole critical section (not dropped \
         immediately)."
    );
}

// =============================================================================
// Scenario: prov101 no-default-model tests serialize global data-dir access via
// a poison-tolerant guard
// =============================================================================

#[test]
fn prov101_no_selection_tests_hold_data_dir_guard() {
    // @step Given the prov101_no_selection_fallbacks integration test file declares a file-scoped DATA_DIR_GUARD Mutex
    let path = tests_dir().join("prov101_no_selection_fallbacks.rs");
    assert!(
        path.exists(),
        "expected prov101 test file at {} — has it relocated?",
        path.display()
    );
    let stripped = strip_rust_comments(&read(&path));
    assert!(
        stripped.contains("static DATA_DIR_GUARD"),
        "prov101_no_selection_fallbacks.rs must declare a file-scoped \
         `static DATA_DIR_GUARD: Mutex<()>` to serialize global data-dir access \
         (PROV-132). None found."
    );

    // @step When any no-selection-fallback test acquires the guard before seeding its data dir
    let unguarded: Vec<String> = extract_fn_bodies(&stripped)
        .into_iter()
        .filter(|(_, is_test, _)| *is_test)
        .filter(|(_, _, body)| body_touches_global_data_dir(body))
        .filter(|(_, _, body)| !body_acquires_guard(body))
        .map(|(name, _, _)| name)
        .collect();

    // @step Then the guard is locked poison-tolerantly via PoisonError::into_inner
    assert!(
        guard_is_poison_tolerant(&stripped),
        "prov101 guard acquisition must be poison-tolerant — either \
         `PoisonError::into_inner` on a `std::sync::Mutex`, or an inherently \
         poison-free `tokio::sync::Mutex` (`Mutex::const_new`)."
    );

    // @step And the guard is bound to a live _guard binding held across the whole test body
    assert!(
        unguarded.is_empty(),
        "these prov101 tests seed the process-global data directory but do NOT \
         hold DATA_DIR_GUARD across their body — a racing SessionManager::new() \
         can observe a foreign default-model.json and break the decline \
         assertion (PROV-132):\n  {}",
        unguarded.join("\n  ")
    );
    assert!(
        stripped.contains("let _guard = DATA_DIR_GUARD"),
        "prov101 tests must bind the guard to a live `let _guard = DATA_DIR_GUARD` \
         binding held for the whole critical section."
    );
}

// =============================================================================
// Scenario: full codelet-sessions suite is deterministic across repeated
// parallel runs
// =============================================================================
//
// The determinism outcome (3x consecutive green full-suite runs) is verified at
// the VALIDATING gate by the supervisor-run harness, because a bare re-run of a
// flake is not a deterministic in-process assertion. Here we pin the *structural
// precondition* that guarantees that outcome: BOTH offending files hold the
// guard across every global-data-dir critical section. If either regresses, this
// test fails, catching the flake's return before it reaches CI as a flake.

#[test]
fn offending_files_are_guarded_so_full_suite_is_deterministic() {
    // @step Given the offending rpc081 and prov101 tests each hold the DATA_DIR_GUARD across their critical section
    let mut violations: Vec<String> = Vec::new();
    for path in offending_test_files() {
        assert!(path.exists(), "{} must exist", path.display());
        let stripped = strip_rust_comments(&read(&path));
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !stripped.contains("static DATA_DIR_GUARD") {
            violations.push(format!("{file}: no `static DATA_DIR_GUARD` declared"));
        }
        for (name, is_test, body) in extract_fn_bodies(&stripped) {
            if is_test && body_touches_global_data_dir(&body) && !body_acquires_guard(&body) {
                violations.push(format!(
                    "{file}::{name}: touches global data dir without the guard"
                ));
            }
        }
    }

    // @step When the full codelet-sessions test suite runs three times consecutively in parallel
    // (Executed by the VALIDATING harness; here we assert the structural precondition.)

    // @step Then every run reports zero failures
    // @step And the malformed-envelope and no-default-model tests pass on every run with no first-run flake
    assert!(
        violations.is_empty(),
        "PROV-132 determinism precondition violated — unguarded global-data-dir \
         access remains, so the full suite can still flake:\n  {}",
        violations.join("\n  ")
    );
}
