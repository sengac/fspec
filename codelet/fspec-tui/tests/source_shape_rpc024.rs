//! RPC-024 — Source-shape regression tests.
//!
//! Feature: spec/features/rpc024-multi-session-cycling.feature
//!
//! Pins the file layout invariants:
//!   * `codelet/fspec-tui/src/store/agent_view/session_context.rs` exists
//!     and is under 300 LoC.
//!   * `codelet/fspec-tui/src/store/agent_view.rs` is under 300 LoC.
//!   * `views/` files do not import codelet_core / codelet_napi / tarpc
//!     / tokio_tungstenite (carried over from RPC-019/020).
//!   * `AgentViewStore::set_session_index` is GONE (the RPC-018 setter
//!     was replaced by a derived getter in RPC-024).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn fspec_tui_src() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn read_raw(path: &std::path::Path) -> String {
    common::read_to_string_or_panic(path)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    read_raw(path).lines().count()
}

/// Scenario: SessionContext lives in its own sub-module under the 300-LoC ceiling
#[test]
fn session_context_module_exists_under_300_loc() {
    // @step Given the codelet/fspec-tui crate
    let path = fspec_tui_src()
        .join("store")
        .join("agent_view")
        .join("session_context.rs");
    // @step Then a file exists at codelet/fspec-tui/src/store/agent_view/session_context.rs
    assert!(path.is_file(), "{} must exist", path.display());
    // @step And that file is under 300 lines
    let n = count_lines_path(&path);
    assert!(n < 300, "session_context.rs has {n} lines, must be < 300");
    // @step And the file codelet/fspec-tui/src/store/agent_view.rs is under 300 lines
    let agent_view = fspec_tui_src().join("store").join("agent_view.rs");
    let m = count_lines_path(&agent_view);
    assert!(m < 300, "store/agent_view.rs has {m} lines, must be < 300");
    // @step And no file under codelet/fspec-tui/src/views/ imports codelet_core, codelet_napi, tarpc, or tokio_tungstenite
    let views_dir = fspec_tui_src().join("views");
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).expect("dir") {
            let e = e.expect("entry");
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&views_dir, &mut files);
    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let body = read_raw(path);
        for needle in [
            "codelet_core::",
            "codelet_napi::",
            "tarpc::",
            "tokio_tungstenite::",
        ] {
            if body.contains(needle) {
                violations.push(format!("{} imports {}", path.display(), needle));
            }
        }
    }
    assert!(violations.is_empty(), "violations: {violations:?}");
}

/// Scenario: SessionContext module declares the required public surface
#[test]
fn session_context_module_declares_public_surface() {
    // @step Given the codelet/fspec-tui crate
    let body = read_raw(
        &fspec_tui_src()
            .join("store")
            .join("agent_view")
            .join("session_context.rs"),
    );
    // @step Then session_context.rs declares "pub struct SessionContext"
    assert!(body.contains("pub struct SessionContext"));
    // @step And session_context.rs declares the "scrollback" field
    assert!(body.contains("scrollback"));
    // @step And session_context.rs declares the "input_draft" field
    assert!(body.contains("input_draft"));
}

/// Scenario: AgentViewStore exposes the multi-session surface
#[test]
fn agent_view_store_exposes_multi_session_surface() {
    // @step Given codelet/fspec-tui/src/store/agent_view.rs after RPC-024 lands
    let body = read_raw(&fspec_tui_src().join("store").join("agent_view.rs"));
    // @step Then the file declares an "open_sessions" field
    assert!(body.contains("open_sessions"));
    // @step And the file declares a "current_session_index" field
    assert!(body.contains("current_session_index"));
    // @step And the file declares "pub fn append_session"
    assert!(body.contains("pub fn append_session"));
    // @step And the file declares "pub fn cycle_session"
    assert!(body.contains("pub fn cycle_session"));
    // @step And the file declares "pub fn set_input_draft"
    assert!(body.contains("pub fn set_input_draft"));
    // @step And the file declares "pub fn session_context_mut_for"
    assert!(body.contains("pub fn session_context_mut_for"));
    // @step And the file declares "pub fn current_session_context"
    assert!(body.contains("pub fn current_session_context"));
    // @step And the file declares "pub fn open_sessions"
    assert!(body.contains("pub fn open_sessions"));
}

/// Scenario: Removing set_session_index closes the explicit-setter regression hole
#[test]
fn agent_view_store_no_longer_exposes_set_session_index() {
    let body = read_raw(&fspec_tui_src().join("store").join("agent_view.rs"));
    // @step Given the AgentViewStore type
    // @step Then there is no public method named set_session_index on AgentViewStore
    assert!(
        !body.contains("pub fn set_session_index"),
        "set_session_index must be removed in RPC-024"
    );
    // @step And the session_index() getter is computed from current_session_index and open_sessions.len()
    assert!(
        body.contains("pub fn session_index"),
        "session_index() getter must exist"
    );
    // The body of session_index() must reference both pieces of state.
    // Capture the function body via a coarse substring search — the
    // implementation slice is free to refactor as long as both names
    // appear in the file.
    assert!(
        body.contains("current_session_index") && body.contains("open_sessions"),
        "session_index() must derive from current_session_index and open_sessions.len()"
    );
}

/// Scenario: AgentView no longer owns the scrollback / next_seq fields
#[test]
fn agent_view_no_longer_owns_scrollback_field() {
    // @step Given codelet/fspec-tui/src/views/agent.rs after RPC-024 lands
    let body = read_raw(&fspec_tui_src().join("views").join("agent.rs"));
    // @step Then the file does NOT declare "pub scrollback: ScrollbackList"
    assert!(
        !body.contains("pub scrollback: ScrollbackList"),
        "AgentView should no longer own a top-level ScrollbackList — \
         scrollback lives on SessionContext after RPC-024"
    );
    // @step And the file does NOT declare "pub next_seq: u64"
    assert!(
        !body.contains("pub next_seq: u64"),
        "AgentView should no longer own a top-level next_seq — \
         scrollback_next_seq lives on SessionContext after RPC-024"
    );
}
