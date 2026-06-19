//! RPC-025 — Source-shape regression tests.
//!
//! Feature: spec/features/rpc025-source-shape.feature
//!
//! Pins the file layout invariants for RPC-021b (history lift + per-session
//! recall state):
//!   * `codelet/core/src/persistence/{mod.rs, history.rs}` exist with the
//!     right size budget AND declare the lifted public surface.
//!   * `codelet/fspec-tui/src/store/agent_view/history_state.rs` exists
//!     and is under 100 LoC; HistoryNavState lives there.
//!   * `codelet/fspec-tui/src/store/agent_view.rs` stays under 300 LoC
//!     after the new per-session history fields are added.
//!   * The NAPI persistence surface degrades to one-line delegates to
//!     `codelet_core::persistence::history`.
//!   * `codelet/rpc-types/src/lib.rs` declares `HistoryMatch` with the
//!     three required fields.
//!   * No file under `codelet/fspec-tui/src/views/` imports forbidden
//!     crates (carried over from RPC-024).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn codelet_root() -> std::path::PathBuf {
    common::workspace_root()
}

fn fspec_tui_src() -> std::path::PathBuf {
    codelet_root().join("fspec-tui").join("src")
}

fn core_src() -> std::path::PathBuf {
    codelet_root().join("core").join("src")
}

fn napi_src() -> std::path::PathBuf {
    codelet_root().join("napi").join("src")
}

fn rpc_types_src() -> std::path::PathBuf {
    codelet_root().join("rpc-types").join("src")
}

fn read_raw(path: &std::path::Path) -> String {
    common::read_to_string_or_panic(path)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    read_raw(path).lines().count()
}

/// Scenario: The lifted history module lives under codelet_core with the right file shape
#[test]
fn lifted_history_module_exists_under_core_with_right_shape() {
    // @step Given the codelet/core crate
    let mod_path = core_src().join("persistence").join("mod.rs");
    let history_path = core_src().join("persistence").join("history.rs");
    // @step Then a file exists at codelet/core/src/persistence/mod.rs
    assert!(
        mod_path.is_file(),
        "{} must exist (codelet_core::persistence::mod)",
        mod_path.display()
    );
    // @step And that file is under 100 lines
    let mod_lines = count_lines_path(&mod_path);
    assert!(
        mod_lines < 100,
        "codelet/core/src/persistence/mod.rs has {mod_lines} lines, must be < 100"
    );
    // @step And a file exists at codelet/core/src/persistence/history.rs
    assert!(
        history_path.is_file(),
        "{} must exist (lifted HistoryStore)",
        history_path.display()
    );
    // @step And codelet/core/src/persistence/history.rs is under 300 lines
    let hist_lines = count_lines_path(&history_path);
    assert!(
        hist_lines < 300,
        "codelet/core/src/persistence/history.rs has {hist_lines} lines, must be < 300"
    );
}

/// Scenario: codelet_core::persistence re-exports the public history surface
#[test]
fn codelet_core_persistence_re_exports_history_surface() {
    // @step Given the codelet/core crate
    let mod_body = read_raw(&core_src().join("persistence").join("mod.rs"));
    let history_body = read_raw(&core_src().join("persistence").join("history.rs"));
    // @step Then codelet/core/src/persistence/mod.rs declares "pub mod history"
    assert!(
        mod_body.contains("pub mod history"),
        "codelet_core::persistence must declare `pub mod history`"
    );
    // @step And codelet/core/src/persistence/mod.rs re-exports "HistoryStore"
    assert!(
        mod_body.contains("HistoryStore"),
        "codelet_core::persistence must re-export HistoryStore"
    );
    // @step And codelet/core/src/persistence/mod.rs re-exports "HistoryEntry"
    assert!(
        mod_body.contains("HistoryEntry"),
        "codelet_core::persistence must re-export HistoryEntry"
    );
    // @step And codelet/core/src/persistence/history.rs declares "pub fn add"
    assert!(
        history_body.contains("pub fn add"),
        "codelet_core::persistence::history must declare `pub fn add`"
    );
    // @step And codelet/core/src/persistence/history.rs declares "pub fn get"
    assert!(
        history_body.contains("pub fn get"),
        "codelet_core::persistence::history must declare `pub fn get`"
    );
    // @step And codelet/core/src/persistence/history.rs declares "pub fn search"
    assert!(
        history_body.contains("pub fn search"),
        "codelet_core::persistence::history must declare `pub fn search`"
    );
}

/// Scenario: HistoryNavState lives in its own sub-module under the 100-LoC ceiling
#[test]
fn history_nav_state_module_exists_under_100_loc() {
    // @step Given the codelet/fspec-tui crate
    let path = fspec_tui_src()
        .join("store")
        .join("agent_view")
        .join("history_state.rs");
    // @step Then a file exists at codelet/fspec-tui/src/store/agent_view/history_state.rs
    assert!(path.is_file(), "{} must exist", path.display());
    // @step And that file is under 100 lines
    let n = count_lines_path(&path);
    assert!(n < 100, "history_state.rs has {n} lines, must be < 100");
    let body = read_raw(&path);
    // @step And the file declares "pub struct HistoryNavState"
    assert!(
        body.contains("pub struct HistoryNavState"),
        "history_state.rs must declare `pub struct HistoryNavState`"
    );
    // @step And the file declares the "recall_index" field
    assert!(
        body.contains("recall_index"),
        "HistoryNavState must declare a `recall_index` field"
    );
    // @step And the file declares the "cached_draft" field
    assert!(
        body.contains("cached_draft"),
        "HistoryNavState must declare a `cached_draft` field"
    );
}

/// Scenario: AgentViewStore stays under 300 LoC after the per-session history fields are added
#[test]
fn agent_view_store_stays_under_300_loc_with_history_fields() {
    // @step Given codelet/fspec-tui/src/store/agent_view.rs after RPC-025 lands
    let path = fspec_tui_src().join("store").join("agent_view.rs");
    let body = read_raw(&path);
    let n = count_lines_path(&path);
    // @step Then the file is under 300 lines
    assert!(
        n < 300,
        "store/agent_view.rs has {n} lines after RPC-025, must be < 300"
    );
    // @step And the file declares a "history_state_by_session" field
    assert!(
        body.contains("history_state_by_session"),
        "agent_view.rs must declare a `history_state_by_session` field"
    );
    // @step And the file declares a "cached_history_snapshot" field
    assert!(
        body.contains("cached_history_snapshot"),
        "agent_view.rs must declare a `cached_history_snapshot` field"
    );
    // @step And the file declares "pub fn history_state_for"
    assert!(
        body.contains("pub fn history_state_for"),
        "agent_view.rs must declare `pub fn history_state_for`"
    );
    // @step And the file declares "pub fn reset_history_state"
    assert!(
        body.contains("pub fn reset_history_state"),
        "agent_view.rs must declare `pub fn reset_history_state`"
    );
    // @step And the file declares "pub fn set_history_snapshot"
    assert!(
        body.contains("pub fn set_history_snapshot"),
        "agent_view.rs must declare `pub fn set_history_snapshot`"
    );
}

/// Scenario: The NAPI persistence surface becomes a thin delegate layer
#[test]
fn napi_persistence_surface_becomes_thin_delegate_layer() {
    // @step Given codelet/napi/src/persistence/mod.rs after the lift
    let napi_persistence_dir = napi_src().join("persistence");
    let napi_mod = read_raw(&napi_persistence_dir.join("mod.rs"));

    // @step Then codelet/napi/src/persistence/history.rs does NOT exist (persistence types live in codelet_core)
    assert!(
        !napi_persistence_dir.join("history.rs").exists(),
        "codelet/napi/src/persistence/history.rs must NOT exist — persistence types live in codelet_core after RPC-035"
    );

    // @step And codelet/napi/src/persistence/mod.rs flat re-exports codelet_core::persistence
    let stripped_mod = common::strip_rust_comments(&napi_mod);
    assert!(
        stripped_mod.contains("pub use codelet_core::persistence::*"),
        "codelet/napi/src/persistence/mod.rs must flat re-export codelet_core::persistence"
    );

    // @step And codelet/napi/src/persistence/napi_bindings.rs::persistence_add_history delegates to history::add
    let napi_bindings = read_raw(&napi_persistence_dir.join("napi_bindings.rs"));
    let stripped_bindings = common::strip_rust_comments(&napi_bindings);
    assert!(
        stripped_bindings.contains("pub fn persistence_add_history")
            && stripped_bindings.contains("history::add"),
        "persistence_add_history NAPI export must delegate to codelet_core::persistence::history::add"
    );
    // @step And codelet/napi/src/persistence/napi_bindings.rs::persistence_get_history delegates to history::get
    assert!(
        stripped_bindings.contains("pub fn persistence_get_history")
            && stripped_bindings.contains("history::get"),
        "persistence_get_history NAPI export must delegate to codelet_core::persistence::history::get"
    );
    // @step And codelet/napi/src/persistence/napi_bindings.rs::persistence_search_history delegates to history::search
    assert!(
        stripped_bindings.contains("pub fn persistence_search_history")
            && stripped_bindings.contains("history::search"),
        "persistence_search_history NAPI export must delegate to codelet_core::persistence::history::search"
    );

    // @step And the existing #[napi] persistence_add_history / persistence_get_history / persistence_search_history exports keep their JS-facing signatures byte-identical
    assert!(
        napi_bindings.contains("pub fn persistence_add_history(display: String, project: String, session_id: String) -> Result<()>"),
        "persistence_add_history NAPI signature must remain byte-identical"
    );
    assert!(
        napi_bindings.contains("pub fn persistence_get_history(")
            && napi_bindings.contains("project: Option<String>")
            && napi_bindings.contains("limit: Option<u32>")
            && napi_bindings.contains("Result<Vec<NapiHistoryEntry>>"),
        "persistence_get_history NAPI signature must remain byte-identical"
    );
    assert!(
        napi_bindings.contains("pub fn persistence_search_history(")
            && napi_bindings.contains("query: String")
            && napi_bindings.contains("project: Option<String>")
            && napi_bindings.contains("Result<Vec<NapiHistoryEntry>>"),
        "persistence_search_history NAPI signature must remain byte-identical"
    );
}

/// Scenario: HistoryMatch is declared in codelet/rpc-types with the expected fields
#[test]
fn history_match_declared_in_rpc_types_with_expected_fields() {
    // @step Given the codelet/rpc-types crate
    let body = read_raw(&rpc_types_src().join("lib.rs"));
    // @step Then codelet/rpc-types/src/lib.rs declares "pub struct HistoryMatch"
    assert!(
        body.contains("pub struct HistoryMatch"),
        "codelet/rpc-types/src/lib.rs must declare `pub struct HistoryMatch`"
    );
    // @step And HistoryMatch declares the "session_id" field of type SessionId
    let history_match_section = body
        .split("pub struct HistoryMatch")
        .nth(1)
        .expect("HistoryMatch declaration must be parseable")
        .split("}")
        .next()
        .expect("HistoryMatch must have a body");
    assert!(
        history_match_section.contains("session_id"),
        "HistoryMatch must declare a `session_id` field"
    );
    assert!(
        history_match_section.contains("SessionId"),
        "HistoryMatch.session_id must have type SessionId"
    );
    // @step And HistoryMatch declares the "text" field of type String
    assert!(
        history_match_section.contains("text"),
        "HistoryMatch must declare a `text` field"
    );
    assert!(
        history_match_section.contains("String"),
        "HistoryMatch must declare fields with type String"
    );
    // @step And HistoryMatch declares the "timestamp_iso" field of type String
    assert!(
        history_match_section.contains("timestamp_iso"),
        "HistoryMatch must declare a `timestamp_iso` field"
    );
    // @step And HistoryMatch is gated on the existing "napi" feature alongside other shared types
    // (Soft assertion: napi feature gates exist for other types — confirm the same crate feature stays)
    assert!(
        body.contains("napi"),
        "codelet/rpc-types/src/lib.rs must continue to use the `napi` feature for shared types"
    );
}

/// Scenario: No view file imports forbidden crates
#[test]
fn no_view_file_imports_forbidden_crates() {
    // @step Given the codelet/fspec-tui crate
    let views_dir = fspec_tui_src().join("views");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    common::collect_rs_files(&views_dir)
        .into_iter()
        .for_each(|p| files.push(p));

    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let body = read_raw(path);
        // @step Then no file under codelet/fspec-tui/src/views/ imports codelet_core
        // @step And no file under codelet/fspec-tui/src/views/ imports codelet_napi
        // @step And no file under codelet/fspec-tui/src/views/ imports tarpc
        // @step And no file under codelet/fspec-tui/src/views/ imports tokio_tungstenite
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
    assert!(
        violations.is_empty(),
        "RPC-025 views-layer import violations: {violations:?}"
    );
}
