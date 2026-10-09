//! BUG-205 — `/detach` slash command REMOVAL + attach-path invariants.
//!
//! Feature: spec/features/remove-detach-slash-command-with-attach-path-intact.feature
//!          spec/features/slash-command-detach-and-work-unit-binding.feature (superseded)
//!
//! The `/detach` command (RPC-050) no longer exists: `SlashCommandAction::Detach`
//! was removed from the registry, so:
//!   1. `SlashCommandAction::from_name("detach")` resolves to `None`.
//!   2. Typing/picking `/detach` falls through to `backend.send_input`
//!      (the BUG-169 parser's `NotASlashCommand` path) — no backend
//!      `set_work_unit_context(None)` call, no notice, no binding change.
//!   3. The slash-command registry contains no command named "detach".
//!   4. The attach path is intact: `Action::AttachWorkUnitToSession` still
//!      calls `backend.set_work_unit_context(s-1, Some(ctx))` and folds the
//!      result into the AgentViewStore.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::{
    filter_commands, SlashCommandAction, SLASH_COMMANDS,
};
use codelet_fspec_tui::{parse_slash_command, Action, App, FspecBackend, SlashCommandParse, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitInfo};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

/// Drain spawned tokio tasks and any queued action-bus messages.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The slash command palette no longer offers /detach
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn slash_command_registry_no_longer_offers_detach() {
    // @step Given the slash command registry after the removal
    // @step Then the registry contains no command named "detach"
    assert!(
        SLASH_COMMANDS
            .iter()
            .all(|c| !c.name().eq_ignore_ascii_case("detach")),
        "SLASH_COMMANDS must not contain a 'detach' entry"
    );
    // @step And SlashCommandAction::from_name("detach") resolves to None
    assert_eq!(
        SlashCommandAction::from_name("detach"),
        None,
        "from_name(\"detach\") must be None after the removal"
    );
    assert_eq!(
        SlashCommandAction::from_name("DETACH"),
        None,
        "from_name must stay case-insensitive-None"
    );
    // @step And the agent help slash-command list contains no "/detach" row
    // The agent help rows are derived from SLASH_COMMANDS, so the name-absence
    // assertion above is the single source of truth for the help list too.
    // (A description-tier palette hit is acceptable: /isolation's description
    // mentions the different worktree "detach" concept — only a command
    // NAMED 'detach' is forbidden.)
    assert!(
        filter_commands("detach")
            .iter()
            .all(|c| !c.name().to_lowercase().contains("detach")),
        "filter_commands(\"detach\") must not surface any command named 'detach'"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: parse_slash_command keeps /detach out of the intercept registry
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn parse_slash_command_keeps_detach_as_plain_input() {
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text="/detach"
    // @step Then it returns NotASlashCommand
    assert_eq!(
        parse_slash_command("/detach"),
        SlashCommandParse::NotASlashCommand,
        "bare /detach must NOT be intercepted anymore (BUG-205)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Typing /detach falls through to the LLM like any unregistered
// command
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_detach_falls_through_to_send_input_and_leaves_the_binding_alone() {
    // @step Given an App with one open session s-1 in AgentView
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    drain_pending(&mut app).await;
    // @step And s-1 is bound to work unit AUTH-001
    app.agent_view_store_mut().set_work_unit_context(
        sid("s-1"),
        WorkUnitContext {
            id: "AUTH-001".to_string(),
            title: "AUTH-001".to_string(),
            status: "implementing".to_string(),
        },
    );

    // @step When the input is submitted with text "/detach"
    let prior_send = mock.send_input_calls();
    app.dispatch(Action::InputSubmitted("/detach".to_string()));
    drain_pending(&mut app).await;

    // @step Then the text "/detach" is forwarded to backend.send_input
    wait_until(
        || mock.send_input_calls() == prior_send + 1,
        "backend.send_input to fire",
    )
    .await;
    assert_eq!(
        mock.last_send_input(),
        Some((sid("s-1"), "/detach".to_string())),
        "the literal '/detach' must be forwarded to the LLM"
    );
    // @step And no work-unit binding change is made for session "s-1"
    assert_eq!(
        mock.set_work_unit_context_calls(),
        0,
        "typed /detach must NOT call backend.set_work_unit_context"
    );
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|c| c.id.as_str()),
        Some("AUTH-001"),
        "the local binding must survive — nothing cleared it"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The attach path and the backend work-unit context RPC surface
// remain intact
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_path_still_binds_and_folds_the_work_unit_context() {
    // @step Given the codebase after the removal
    // @step And an App with open session s-1 whose BoardStore holds AUTH-001
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    drain_pending(&mut app).await;
    app.board_store_mut().replace_work_units(vec![WorkUnitInfo {
        id: "AUTH-001".to_string(),
        title: "AUTH-001".to_string(),
        work_type: "story".to_string(),
        status: "implementing".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: vec![],
        last_state_change_at: None,
    }]);

    // @step When Action::AttachWorkUnitToSession("AUTH-001") is dispatched
    app.dispatch(Action::AttachWorkUnitToSession("AUTH-001".to_string()));
    drain_pending(&mut app).await;

    // @step Then backend.set_work_unit_context(Some(ctx)) still binds a work unit
    wait_until(
        || mock.set_work_unit_context_calls() == 1,
        "set_work_unit_context to fire",
    )
    .await;
    let (session, ctx) = mock.last_set_work_unit_context().expect("last set call");
    assert_eq!(session, sid("s-1"));
    assert_eq!(
        ctx.as_ref().map(|c| c.id.as_str()),
        Some("AUTH-001"),
        "the attach path must still call set_work_unit_context with Some(ctx)"
    );
    // @step And folds it into AgentViewStore
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|c| c.id.as_str()),
        Some("AUTH-001"),
        "the WorkUnitAttached fold must land in the AgentViewStore"
    );
}
