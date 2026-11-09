//! RPC-025 — Source-shape regression tests.
//!
//! Feature: spec/features/rpc025-source-shape.feature
//!
//! Pins the file layout invariants for RPC-021b (history lift + per-session
//! recall state):
//!   * `rust/core/src/persistence/{mod.rs, history.rs}` exist with the
//!     right size budget AND declare the lifted public surface.
//!   * `rust/fspec-tui/src/store/agent_view/history_state.rs` exists
//!     and is under 100 LoC; HistoryNavState lives there.
//!   * `rust/fspec-tui/src/store/agent_view.rs` stays under 300 LoC
//!     after the new per-session history fields are added.
//!   * The NAPI persistence surface degrades to one-line delegates to
//!     `codelet_core::persistence::history`.
//!   * `rust/rpc-types/src/lib.rs` declares `HistoryMatch` with the
//!     three required fields.
//!   * No file under `rust/fspec-tui/src/views/` imports forbidden
//!     crates (carried over from RPC-024).

use codelet_test_helpers::{OrFail, TestResult};
mod common;

fn codelet_root() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    common::workspace_root()
}

fn fspec_tui_src() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(codelet_root()?.join("fspec-tui").join("src"))
}

fn core_src() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(codelet_root()?.join("core").join("src"))
}

fn rpc_types_src() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(codelet_root()?.join("rpc-types").join("src"))
}

fn read_raw(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    common::read_to_string_or_fail(path)
}

fn count_lines_path(path: &std::path::Path) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(read_raw(path)?.lines().count())
}

/// Scenario: The lifted history module lives under codelet_core with the right file shape
#[test]
fn lifted_history_module_exists_under_core_with_right_shape() -> TestResult {
    // @step Given the rust/core crate
    let mod_path = core_src()?.join("persistence").join("mod.rs");
    let history_path = core_src()?.join("persistence").join("history.rs");
    // @step Then a file exists at rust/core/src/persistence/mod.rs
    assert!(
        mod_path.is_file(),
        "{} must exist (codelet_core::persistence::mod)",
        mod_path.display()
    );
    // @step And that file is under 100 lines
    let mod_lines = count_lines_path(&mod_path)?;
    assert!(
        mod_lines < 100,
        "rust/core/src/persistence/mod.rs has {mod_lines} lines, must be < 100"
    );
    // @step And a file exists at rust/core/src/persistence/history.rs
    assert!(
        history_path.is_file(),
        "{} must exist (lifted HistoryStore)",
        history_path.display()
    );
    // @step And rust/core/src/persistence/history.rs is under 300 lines
    let hist_lines = count_lines_path(&history_path)?;
    assert!(
        hist_lines < 300,
        "rust/core/src/persistence/history.rs has {hist_lines} lines, must be < 300"
    );
    Ok(())
}

/// Scenario: codelet_core::persistence re-exports the public history surface
#[test]
fn codelet_core_persistence_re_exports_history_surface() -> TestResult {
    // @step Given the rust/core crate
    let mod_body = read_raw(&core_src()?.join("persistence").join("mod.rs"))?;
    let history_body = read_raw(&core_src()?.join("persistence").join("history.rs"))?;
    // @step Then rust/core/src/persistence/mod.rs declares "pub mod history"
    assert!(
        mod_body.contains("pub mod history"),
        "codelet_core::persistence must declare `pub mod history`"
    );
    // @step And rust/core/src/persistence/mod.rs re-exports "HistoryStore"
    assert!(
        mod_body.contains("HistoryStore"),
        "codelet_core::persistence must re-export HistoryStore"
    );
    // @step And rust/core/src/persistence/mod.rs re-exports "HistoryEntry"
    assert!(
        mod_body.contains("HistoryEntry"),
        "codelet_core::persistence must re-export HistoryEntry"
    );
    // @step And rust/core/src/persistence/history.rs declares "pub fn add"
    assert!(
        history_body.contains("pub fn add"),
        "codelet_core::persistence::history must declare `pub fn add`"
    );
    // @step And rust/core/src/persistence/history.rs declares "pub fn get"
    assert!(
        history_body.contains("pub fn get"),
        "codelet_core::persistence::history must declare `pub fn get`"
    );
    // @step And rust/core/src/persistence/history.rs declares "pub fn search"
    assert!(
        history_body.contains("pub fn search"),
        "codelet_core::persistence::history must declare `pub fn search`"
    );
    Ok(())
}

/// Scenario: HistoryNavState lives in its own sub-module under the 100-LoC ceiling
#[test]
fn history_nav_state_module_exists_under_100_loc() -> TestResult {
    // @step Given the rust/fspec-tui crate
    let path = fspec_tui_src()?
        .join("store")
        .join("agent_view")
        .join("history_state.rs");
    // @step Then a file exists at rust/fspec-tui/src/store/agent_view/history_state.rs
    assert!(path.is_file(), "{} must exist", path.display());
    // @step And that file is under 100 lines
    let n = count_lines_path(&path)?;
    assert!(n < 100, "history_state.rs has {n} lines, must be < 100");
    let body = read_raw(&path)?;
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
    Ok(())
}

/// Scenario: AgentViewStore stays under 300 LoC after the per-session history fields are added
#[test]
fn agent_view_store_stays_under_300_loc_with_history_fields() -> TestResult {
    // @step Given rust/fspec-tui/src/store/agent_view.rs after RPC-025 lands
    let path = fspec_tui_src()?.join("store").join("agent_view.rs");
    let body = read_raw(&path)?;
    let n = count_lines_path(&path)?;
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
    // @step And store/agent_view/history_accessors.rs declares "pub fn history_state_for"
    let history_accessors = fspec_tui_src()?
        .join("store")
        .join("agent_view")
        .join("history_accessors.rs");
    let accessors = read_raw(&history_accessors)?;
    assert!(
        accessors.contains("pub fn history_state_for"),
        "store/agent_view/history_accessors.rs must declare `pub fn history_state_for`"
    );
    // @step And store/agent_view/history_accessors.rs declares "pub fn reset_history_state"
    assert!(
        accessors.contains("pub fn reset_history_state"),
        "store/agent_view/history_accessors.rs must declare `pub fn reset_history_state`"
    );
    // @step And store/agent_view/history_accessors.rs declares "pub fn set_history_snapshot"
    assert!(
        accessors.contains("pub fn set_history_snapshot"),
        "store/agent_view/history_accessors.rs must declare `pub fn set_history_snapshot`"
    );
    Ok(())
}

/// Scenario: HistoryMatch is declared in rust/rpc-types with the expected fields
#[test]
fn history_match_declared_in_rpc_types_with_expected_fields() -> TestResult {
    // @step Given the rust/rpc-types crate
    let body = read_tree(&rpc_types_src()?.join("lib.rs"))?;
    // @step Then rust/rpc-types/src/lib.rs declares "pub struct HistoryMatch"
    assert!(
        body.contains("pub struct HistoryMatch"),
        "rust/rpc-types/src/lib.rs must declare `pub struct HistoryMatch`"
    );
    // @step And HistoryMatch declares the "session_id" field of type SessionId
    let history_match_section = body
        .split("pub struct HistoryMatch")
        .nth(1)
        .or_fail("HistoryMatch declaration must be parseable")?
        .split("}")
        .next()
        .or_fail("HistoryMatch must have a body")?;
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
    let history_match_prelude = body
        .split("pub struct HistoryMatch")
        .next()
        .or_fail("HistoryMatch prelude must exist")?;
    let history_match_derive = history_match_prelude
        .rfind("#[derive(")
        .map_or("", |idx| &history_match_prelude[idx..]);
    assert!(
        history_match_derive.contains("Serialize") && history_match_derive.contains("Deserialize"),
        "HistoryMatch must carry the same serde derives as the other shared rpc-types types"
    );
    Ok(())
}

/// Scenario: No view file imports forbidden crates
#[test]
fn no_view_file_imports_forbidden_crates() -> TestResult {
    // @step Given the rust/fspec-tui crate
    let views_dir = fspec_tui_src()?.join("views");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    common::collect_rs_files(&views_dir)
        .into_iter()
        .for_each(|p| files.push(p));

    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let body = read_raw(path)?;
        // @step Then no file under rust/fspec-tui/src/views/ imports codelet_core
        // @step And no file under rust/fspec-tui/src/views/ imports codelet_napi
        // @step And no file under rust/fspec-tui/src/views/ imports tarpc
        // @step And no file under rust/fspec-tui/src/views/ imports tokio_tungstenite
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
    Ok(())
}

/// REFAC-021/022: read a source file together with its include!d fragments
/// and file-backed child modules (split modules read as one text).
fn read_tree(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_module_tree(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?,
    )
}
