//! RPC-012 — Source-shape assertion that the Action enum gains the
//! navigator-slice variants without losing the existing ones.
//!
//! Feature: spec/features/rpc012-action-variants.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_fspec_tui::Action;
use codelet_rpc_types::SessionId;

/// Scenario: Action enum gains four new variants for the navigator slice
#[test]
fn action_enum_gains_navigator_slice_variants() {
    // @step Given the Action enum in codelet/fspec-tui/src/components/mod.rs
    let source = common::read_to_string_or_panic(
        &common::workspace_root()
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("mod.rs"),
    );
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
    // @step And it contains the variant NavigationTargetSet(Option<SessionId>)
    assert!(source.contains("NavigationTargetSet(Option<codelet_rpc_types::SessionId>)"));
    let _ = Action::NavigationTargetSet(None);
    // @step And it contains the variant AttachSession(String, SessionId)
    assert!(source.contains("AttachSession(String, codelet_rpc_types::SessionId)"));
    let _ = Action::AttachSession("AUTH-001".to_string(), SessionId::new("s-1"));
}
