//! RPC-012 — Source-shape assertion that the Action enum gains the
//! navigator-slice variants without losing the existing ones.
//!
//! Feature: spec/features/rpc012-action-variants.feature

mod common;

use codelet_fspec_tui::Action;
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{OrFail, TestResult};

/// Scenario: Action enum gains the navigator slice variants
#[test]
fn action_enum_gains_navigator_slice_variants() -> TestResult {
    // @step Given the Action enum in rust/fspec-tui/src/components/mod.rs
    let source = codelet_test_helpers::source_composition::read_source(
        &common::workspace_root()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("mod.rs"),
    )
    .or_fail("resolve Action enum source")?;
    // @step Then it contains the variant EnterWorkUnit(String)
    assert!(source.contains("EnterWorkUnit(String)"));
    let _ = Action::EnterWorkUnit("AUTH-001".to_string());
    // @step And it contains the variant OpenAgentView(Option<SessionId>)
    assert!(source.contains("OpenAgentView(Option<codelet_rpc_types::SessionId>)"));
    let _ = Action::OpenAgentView(Some(SessionId::new("s-1")));
    let _ = Action::OpenAgentView(None);
    // @step And it contains the variant BackToBoard
    assert!(source.contains("BackToBoard"));
    let _ = Action::BackToBoard;
    Ok(())
}
