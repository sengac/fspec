//! RPC-060 — Source-shape assertions for the CreateSessionDialog +
//! isolated-session creation surface.
//!
//! Feature: spec/features/rpc060-isolated-session-dialog.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the new CreateSessionDialog component, the three new
//! Action variants, the dispatch_create_session_dialog helper module, and the
//! /isolation slash-command dispatch rewiring. Mirrors the
//! source_shape_rpc059 pattern.

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .and_then(|p| p.parent())
        .or_fail("workspace root above rust/fspec-tui")?
        .to_path_buf())
}

fn count_lines(path: &PathBuf) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(fs::read_to_string(path)
        .or_fail("read source")?
        .lines()
        .count())
}

/// Scenario: CreateSessionDialog component file exists with the documented surface
#[test]
fn create_session_dialog_component_exists() -> TestResult {
    // @step Given the file rust/fspec-tui/src/components/create_session_dialog.rs exists
    let path = workspace_root()?.join("rust/fspec-tui/src/components/create_session_dialog.rs");
    let source = fs::read_to_string(&path).or_fail("read create_session_dialog.rs")?;

    // @step Then it declares a public struct named "CreateSessionDialog"
    assert!(
        source.contains("pub struct CreateSessionDialog"),
        "create_session_dialog.rs should declare pub struct CreateSessionDialog"
    );

    // @step And it declares a public enum named "CreateSessionOption" with variants Yes, Isolated, Cancel
    assert!(
        source.contains("pub enum CreateSessionOption"),
        "create_session_dialog.rs should declare pub enum CreateSessionOption"
    );
    for variant in ["Yes,", "Isolated,", "Cancel,"] {
        assert!(
            source.contains(variant),
            "CreateSessionOption should declare variant {variant}"
        );
    }

    // @step And it declares a public constant CREATE_SESSION_DIALOG_ID
    assert!(
        source.contains("pub const CREATE_SESSION_DIALOG_ID"),
        "create_session_dialog.rs should declare pub const CREATE_SESSION_DIALOG_ID"
    );

    // @step And it uses the shared dialog_theme renderer
    // TUI-112: the three-button dialogs now delegate to
    // `three_button_dialog::render_three_button_dialog`, which itself
    // calls `dialog_theme::render_dialog` — either counts as using the
    // shared renderer (the contract is "no direct Block/Paragraph").
    assert!(
        source.contains("render_dialog") || source.contains("render_three_button_dialog"),
        "production source must use dialog_theme::render_dialog"
    );

    // @step And the file stays under 300 lines
    assert!(
        count_lines(&path)? < 300,
        "create_session_dialog.rs has {} lines (>= 300)",
        count_lines(&path)?
    );
    Ok(())
}

/// Scenario: Action enum gains the three RPC-060 variants
#[test]
fn action_enum_gains_rpc060_variants() -> TestResult {
    // @step Given the file rust/fspec-tui/src/components/mod.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/components/mod.rs");
    let source = codelet_test_helpers::source_composition::read_source(&path)
        .or_fail("read components/mod.rs")?;

    // @step Then it declares Action::OpenCreateSessionDialog
    assert!(
        source.contains("OpenCreateSessionDialog"),
        "Action enum should declare OpenCreateSessionDialog"
    );
    // @step And it declares Action::CreateSessionSubmitted { isolated: bool }
    assert!(
        source.contains("CreateSessionSubmitted"),
        "Action enum should declare CreateSessionSubmitted"
    );
    // @step And it declares Action::CreateSessionCancelled
    assert!(
        source.contains("CreateSessionCancelled"),
        "Action enum should declare CreateSessionCancelled"
    );
    Ok(())
}

/// Scenario: /isolation slash command dispatches Action::OpenCreateSessionDialog
#[test]
fn dispatch_slash_commands_routes_isolation_to_open_create_session_dialog() -> TestResult {
    // @step Given the file rust/fspec-tui/src/app/dispatch_slash_commands.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/app/dispatch_slash_commands.rs");
    let source = fs::read_to_string(&path).or_fail("read app/dispatch_slash_commands.rs")?;

    // @step Then it matches SlashCommandAction::Isolation
    assert!(
        source.contains("SlashCommandAction::Isolation"),
        "dispatch_slash_commands.rs should match SlashCommandAction::Isolation"
    );
    // @step And it dispatches Action::OpenCreateSessionDialog with Some(Isolated)
    assert!(
        source.contains("OpenCreateSessionDialog"),
        "dispatch_slash_commands.rs should dispatch OpenCreateSessionDialog"
    );
    assert!(
        source.contains("CreateSessionOption::Isolated"),
        "dispatch_slash_commands.rs should preselect CreateSessionOption::Isolated"
    );
    Ok(())
}

/// Scenario: codelet-fspec-tui lib re-exports the new dialog surface
#[test]
fn lib_re_exports_create_session_dialog() -> TestResult {
    // @step Given the file rust/fspec-tui/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/lib.rs");
    let source = fs::read_to_string(&path).or_fail("read lib.rs")?;

    // @step Then it re-exports CreateSessionDialog and CreateSessionOption
    assert!(
        source.contains("CreateSessionDialog"),
        "lib.rs should re-export CreateSessionDialog"
    );
    assert!(
        source.contains("CreateSessionOption"),
        "lib.rs should re-export CreateSessionOption"
    );
    assert!(
        source.contains("CREATE_SESSION_DIALOG_ID"),
        "lib.rs should re-export CREATE_SESSION_DIALOG_ID"
    );
    Ok(())
}
