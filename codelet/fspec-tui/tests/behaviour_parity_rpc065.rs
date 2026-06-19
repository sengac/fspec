//! RPC-065 — Behaviour-parity test suite for every slash command +
//! keyboard shortcut.
//!
//! Feature: spec/features/agent-view-behaviour-parity-matrix.feature
//!
//! Walks the full matrix from spec/attachments/RPC-065/behaviour-parity-tests.md
//! through a deterministic `MockBackend` driven by the reusable
//! `AppTestHarness` in `tests/common/harness.rs`. Each test asserts
//! ONLY OBSERVABLE store-state transitions — the deep behaviour
//! (scrollback-text formatting, error branches, debounce timing) is
//! covered by the canonical card test referenced via `DEEP-REF`.
//!
//! Test sections (one mod per matrix row family):
//!   slash_help, slash_clear, slash_quit, slash_model,
//!   slash_thinking, slash_role, slash_resume, slash_search,
//!   slash_provider, slash_debug, slash_compact,
//!   slash_isolation, slash_blocklist, slash_detach, slash_merge_worktree,
//!   slash_schedule, slash_loop, key_shift_arrows, key_history_recall,
//!   key_tab_turn_selection, key_ctrl_r, key_esc_cascade,
//!   key_enter_submit, key_ctrl_c_interrupt, key_pagedown_end.
//!
//! NOTE: `slash_providers_alias` was removed on 2026-06-01. The
//! TypeScript SLASH_COMMANDS registry (src/tui/utils/slashCommands.ts)
//! defines exactly one provider-related command: `name: 'provider'`.
//! There is no `/providers` alias in TS, so the Rust frontend mirrors
//! that 1:1 — see the deletion notice further down in this file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{
    Action, ViewMode, CREATE_SESSION_DIALOG_ID, MODEL_SELECTOR_DIALOG_ID, ROLE_DIALOG_ID,
    THINKING_LEVEL_DIALOG_ID,
};
use codelet_rpc_types::{SessionId, SessionStatus, ThinkingLevel, WorkUnitContext};
use crossterm::event::KeyCode;

mod common;
use common::harness::{seed_sid, AppTestHarness};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

// ─────────────────────────────────────────────────────────────────────────
// /help — pushes HelpDialog onto the compositor
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (slash /help → setHelpDialogOpen(true))
/// DEEP-REF: tests/app_dispatch_rpc020.rs::slash_help_pushes_help_dialog
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_help_pushes_help_dialog_onto_compositor() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/help"
    h.dispatch_slash(SlashCommandAction::Help);

    // @step Then the compositor contains a layer with id "help-dialog"
    assert!(h.compositor_contains("help-dialog"));
}

// ─────────────────────────────────────────────────────────────────────────
// /quit — flips should_quit
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (slash /quit → process.exit / store.quit)
/// DEEP-REF: tests/app_dispatch_rpc020.rs::slash_quit_emits_quit_action
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_quit_flips_should_quit_flag() {
    // @step Given a fresh AppTestHarness with focused session s-1
    // @step And the app's should_quit flag is false
    let mut h = AppTestHarness::new();
    assert!(!h.should_quit());

    // @step When I dispatch the slash command "/quit"
    h.dispatch_slash(SlashCommandAction::Quit);

    // @step Then the app's should_quit flag is true
    assert!(h.should_quit());

    // @step And the MockBackend has received no calls (matrix smoke: no
    // backend call is triggered by /quit)
    assert_eq!(h.mock.send_input_calls(), 0);
    assert_eq!(h.mock.clear_history_calls(), 0);
    assert_eq!(h.mock.interrupt_calls(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// /clear — wipes scrollback + calls backend.clear_history
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleClearCommand)
/// DEEP-REF: tests/slash_clear_rpc046.rs (full Ok/Err + multi-session)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_clear_resets_scrollback_and_calls_backend() {
    // @step Given a fresh AppTestHarness with focused session s-1 seeded with 5 scrollback chunks
    let mut h = AppTestHarness::new();
    h.seed_chunks(&seed_sid(), 5);
    assert_eq!(h.scrollback_chunk_count(&seed_sid()), 5);

    // @step When I dispatch the slash command "/clear"
    h.dispatch_slash(SlashCommandAction::Clear);

    // @step Then the focused session's scrollback chunk count is 0 synchronously
    assert_eq!(h.scrollback_chunk_count(&seed_sid()), 0);

    // @step And within 1 second MockBackend.clear_history_calls() is 1
    h.wait_for_mock(
        |m| m.clear_history_calls() == 1,
        "MockBackend.clear_history_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_clear_history_session() is Some(s-1)
    assert_eq!(h.mock.last_clear_history_session(), Some(seed_sid()));
}

// ─────────────────────────────────────────────────────────────────────────
// /model — activates ViewMode::ModelSelector (RPC-337: full-screen
// mode-view replaces the retired RPC-022 Compositor modal)
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/components/ModelSelectorView.tsx (full-screen view)
/// DEEP-REF: tests/rpc337_navigator_model_selector.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_model_activates_model_selector_view() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/model"
    h.dispatch_slash(SlashCommandAction::Model);

    // @step And I drain pending tasks and actions
    h.drain_pending().await;

    // @step Then the navigator's active_view is ViewMode::ModelSelector
    assert_eq!(h.active_view(), ViewMode::ModelSelector);
    // @step And no Compositor modal is pushed (the modal is retired)
    assert!(!h.compositor_contains(MODEL_SELECTOR_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// /thinking (bare) — pushes ThinkingLevelDialog
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleThinkingCommand bare)
/// DEEP-REF: tests/thinking_level_dialog_rpc022.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_thinking_bare_pushes_thinking_level_dialog() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/thinking"
    h.dispatch_slash(SlashCommandAction::Thinking);

    // @step Then the compositor contains a layer with id THINKING_LEVEL_DIALOG_ID
    assert!(h.compositor_contains(THINKING_LEVEL_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// /thinking high (inline) — calls backend.set_thinking_level
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleThinkingCommand inline-arg)
/// DEEP-REF: tests/slash_thinking_rpc048.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_thinking_high_inline_sets_level_without_dialog() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I submit the input text "/thinking high"
    h.submit_input("/thinking high");

    // @step Then the compositor does NOT contain a layer with id THINKING_LEVEL_DIALOG_ID
    assert!(!h.compositor_contains(THINKING_LEVEL_DIALOG_ID));

    // @step And within 1 second MockBackend.set_thinking_level_calls() is 1
    h.wait_for_mock(
        |m| m.set_thinking_level_calls() == 1,
        "MockBackend.set_thinking_level_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_set_thinking_level() is Some((s-1, ThinkingLevel::High))
    assert_eq!(
        h.mock.last_set_thinking_level(),
        Some((seed_sid(), ThinkingLevel::High))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /role (bare) — pushes RoleDialog
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleRoleCommand bare)
/// DEEP-REF: tests/role_dialog_rpc063.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_role_bare_pushes_role_dialog() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/role"
    h.dispatch_slash(SlashCommandAction::Role);

    // @step Then the compositor contains a layer with id ROLE_DIALOG_ID
    assert!(h.compositor_contains(ROLE_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// /role <text> (inline) — calls backend.set_session_role
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleRoleCommand inline-arg)
/// DEEP-REF: tests/slash_role_rpc063.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_role_inline_text_sets_session_role_without_dialog() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I submit the input text "/role You are a security reviewer"
    h.submit_input("/role You are a security reviewer");

    // @step Then the compositor does NOT contain a layer with id ROLE_DIALOG_ID
    assert!(!h.compositor_contains(ROLE_DIALOG_ID));

    // @step And within 1 second MockBackend.set_session_role_calls() is 1
    h.wait_for_mock(
        |m| m.set_session_role_calls() == 1,
        "MockBackend.set_session_role_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_set_session_role() is Some((s-1, Some("You are a security reviewer")))
    assert_eq!(
        h.mock.last_set_session_role(),
        Some((seed_sid(), Some("You are a security reviewer".to_string())))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /resume — opens ResumeSessionView mode-view
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleResumeCommand)
/// DEEP-REF: tests/slash_resume_rpc049.rs + tests/rpc026_resume_session_view.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_resume_opens_resume_session_view() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/resume"
    h.dispatch_slash(SlashCommandAction::Resume);

    // @step Then the AgentView's resume_view is Some(_)
    assert!(h.resume_view_active());
}

// ─────────────────────────────────────────────────────────────────────────
// /search — opens SearchHistoryView with no backend call
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx line ~2721 (setIsSearchMode/setSearchQuery)
/// DEEP-REF: tests/search_view_rpc064.rs::picking_search_from_palette_opens_empty_view
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_search_opens_search_view_without_backend_call() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/search"
    h.dispatch_slash(SlashCommandAction::Search);

    // @step Then the AgentView's search_view is Some(_)
    assert!(h.search_view_active());

    // @step And MockBackend.search_history_calls() is 0
    assert_eq!(h.mock.search_history_calls(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// /provider — activates ViewMode::ProviderSettings
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleProviderCommand)
/// DEEP-REF: tests/provider_settings_view_rpc054.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_provider_activates_provider_settings_view() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/provider"
    h.dispatch_slash(SlashCommandAction::Provider);

    // @step And I drain pending tasks and actions
    h.drain_pending().await;

    // @step Then the navigator's active_view is ViewMode::ProviderSettings
    assert_eq!(h.active_view(), ViewMode::ProviderSettings);
}

// ─────────────────────────────────────────────────────────────────────────
// /providers — DELETED (no alias). The TypeScript SLASH_COMMANDS registry
// (src/tui/utils/slashCommands.ts) defines exactly one provider-related
// entry: `name: 'provider'`. The Rust frontend mirrors that 1:1 — there
// is no `SlashCommandAction::Providers` variant and no `/providers`
// command. The previous `slash_providers_alias_activates_provider_settings_view`
// test asserted a fabrication that did not exist in the TS reference and
// has been deleted as part of the RPC-054 / RPC-020 / RPC-065 cross-card
// revision (2026-06-01).
// ─────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────
// /debug — calls backend.toggle_debug
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleDebugCommand)
/// DEEP-REF: tests/slash_debug_rpc055.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_debug_calls_backend_toggle_debug() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/debug"
    h.dispatch_slash(SlashCommandAction::Debug);

    // @step Then within 1 second MockBackend.toggle_debug_calls() is 1
    h.wait_for_mock(
        |m| m.toggle_debug_calls() == 1,
        "MockBackend.toggle_debug_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_toggle_debug() references session s-1
    let last = h.mock.last_toggle_debug();
    assert!(
        matches!(last.as_ref(), Some((sid, _)) if *sid == seed_sid()),
        "expected last_toggle_debug to reference s-1, got {last:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /compact — calls backend.compact_session
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx line ~2673 (handleCompactCommand)
/// DEEP-REF: tests/slash_compact_rpc047.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_compact_calls_backend_compact_session() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/compact"
    h.dispatch_slash(SlashCommandAction::Compact);

    // @step Then within 1 second MockBackend.compact_session_calls() is 1
    h.wait_for_mock(
        |m| m.compact_session_calls() == 1,
        "MockBackend.compact_session_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_compact_session() is Some(s-1)
    assert_eq!(h.mock.last_compact_session(), Some(seed_sid()));
}

// ─────────────────────────────────────────────────────────────────────────
// /isolation — opens CreateSessionDialog (preselect Isolated)
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleIsolationCommand)
/// DEEP-REF: tests/isolated_session_dialog_rpc060.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_isolation_opens_create_session_dialog() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/isolation"
    h.dispatch_slash(SlashCommandAction::Isolation);

    // @step And I drain pending tasks and actions
    h.drain_pending().await;

    // @step Then the compositor contains a layer with id CREATE_SESSION_DIALOG_ID
    assert!(h.compositor_contains(CREATE_SESSION_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// /blocklist — activates ViewMode::Blocklist
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleBlocklistCommand)
/// DEEP-REF: tests/blocklist_view_rpc056.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_blocklist_activates_blocklist_view() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/blocklist"
    h.dispatch_slash(SlashCommandAction::Blocklist);

    // @step And I drain pending tasks and actions
    h.drain_pending().await;

    // @step Then the navigator's active_view is ViewMode::Blocklist
    assert_eq!(h.active_view(), ViewMode::Blocklist);
}

// ─────────────────────────────────────────────────────────────────────────
// /detach — calls backend.set_work_unit_context(None)
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleDetachCommand)
/// DEEP-REF: tests/slash_detach_rpc050.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_detach_clears_work_unit_context() {
    // @step Given a fresh AppTestHarness with focused session s-1 bound to a WorkUnitContext
    let mut h = AppTestHarness::new();
    let ctx = WorkUnitContext {
        id: "AUTH-001".to_string(),
        title: "User Login".to_string(),
        status: "implementing".to_string(),
    };
    h.app
        .agent_view_store_mut()
        .set_work_unit_context(seed_sid(), ctx);

    // @step When I dispatch the slash command "/detach"
    h.dispatch_slash(SlashCommandAction::Detach);

    // @step Then within 1 second MockBackend.set_work_unit_context_calls() is 1
    h.wait_for_mock(
        |m| m.set_work_unit_context_calls() == 1,
        "MockBackend.set_work_unit_context_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_set_work_unit_context() is Some((s-1, None))
    assert_eq!(
        h.mock.last_set_work_unit_context(),
        Some((seed_sid(), None))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /merge-worktree — calls backend.inspect_session_changes
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleMergeWorktreeCommand)
/// DEEP-REF: tests/merge_worktree_rpc057.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_merge_worktree_calls_backend_inspect_session_changes() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/merge-worktree"
    h.dispatch_slash(SlashCommandAction::MergeWorktree);

    // @step Then within 1 second MockBackend.inspect_session_changes_calls() is 1
    h.wait_for_mock(
        |m| m.inspect_session_changes_calls() == 1,
        "MockBackend.inspect_session_changes_calls() == 1",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// /schedule (bare) — emits help notice
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleScheduleCommand help)
/// DEEP-REF: tests/schedule_dispatch_rpc058.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_schedule_bare_emits_help_notice() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/schedule"
    h.dispatch_slash(SlashCommandAction::Schedule);
    h.drain_pending().await;

    // @step Then the focused session's scrollback gains a help notice chunk
    assert!(
        h.scrollback_chunk_count(&seed_sid()) >= 1,
        "expected at least one help-notice chunk in s-1 scrollback",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /schedule list — calls backend.schedule_list via parser
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (parseSchedule → schedule.list)
/// DEEP-REF: tests/schedule_dispatch_rpc058.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_schedule_list_calls_backend_schedule_list() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I submit the input text "/schedule list"
    h.submit_input("/schedule list");

    // @step Then within 1 second MockBackend.schedule_list_calls() is 1
    h.wait_for_mock(
        |m| m.schedule_list_calls() == 1,
        "MockBackend.schedule_list_calls() == 1",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// /loop (bare) — emits help notice
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleLoopCommand help)
/// DEEP-REF: tests/loop_dispatch_rpc059.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_loop_bare_emits_help_notice() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I dispatch the slash command "/loop"
    h.dispatch_slash(SlashCommandAction::Loop);
    h.drain_pending().await;

    // @step Then the focused session's scrollback gains a help notice chunk
    assert!(
        h.scrollback_chunk_count(&seed_sid()) >= 1,
        "expected at least one help-notice chunk in s-1 scrollback",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// /loop list — calls backend.loop_list via parser
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (parseLoop → loop.list)
/// DEEP-REF: tests/loop_dispatch_rpc059.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_loop_list_calls_backend_loop_list() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I submit the input text "/loop list"
    h.submit_input("/loop list");

    // @step Then within 1 second MockBackend.loop_list_calls() is 1
    h.wait_for_mock(
        |m| m.loop_list_calls() == 1,
        "MockBackend.loop_list_calls() == 1",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Shift+←/→ — cycles between open sessions
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (Shift+Arrow → cycleSession)
/// DEEP-REF: tests/app_dispatch_rpc024.rs (full SessionPrev/SessionNext coverage)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_shift_right_and_left_cycle_focus_between_open_sessions() {
    // @step Given a fresh AppTestHarness with two open sessions s-1 and s-2, focused on s-1
    let mut h = AppTestHarness::new();
    h.add_session(sid("s-2"));
    // s-2 was created second → currently focused. Cycle back to s-1.
    h.app.dispatch(Action::SessionPrev);
    assert_eq!(h.current_session(), Some(seed_sid()));

    // @step When I press Shift+Right
    h.press_key(AppTestHarness::key_shift_right());
    h.drain_pending().await;

    // @step Then the focused session is s-2
    assert_eq!(h.current_session(), Some(sid("s-2")));

    // @step When I press Shift+Left
    h.press_key(AppTestHarness::key_shift_left());
    h.drain_pending().await;

    // @step Then the focused session is s-1
    assert_eq!(h.current_session(), Some(seed_sid()));
}

// ─────────────────────────────────────────────────────────────────────────
// Shift+↑ — loads most-recent history entry into the input
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (Shift+Up → historyRecall)
/// DEEP-REF: tests/app_dispatch_history_rpc025.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_shift_up_loads_most_recent_history_entry_into_input() {
    // @step Given a fresh AppTestHarness with focused session s-1
    // @step And MockBackend has scripted persistence history ["old2", "old1"] for s-1
    //
    // Note: `script_history` uses index-0 = most-recent (see the
    // RPC-025 test fixture which scripts `["third", "second", "first"]`
    // and expects the first Shift+↑ press to surface `"third"`). Our
    // matrix-row narrative ("most-recent entry into the input") thus
    // maps onto `script_history`'s index 0 — i.e. `"old2"` when the
    // human-ordered list is `[older, …, newer]`.
    let mut h = AppTestHarness::new();
    h.mock
        .script_history(seed_sid(), vec!["old2".to_string(), "old1".to_string()]);

    // @step When I press Shift+Up
    h.press_key(AppTestHarness::key_shift_up());
    h.drain_pending().await;

    // @step Then within 1 second the input value is "old2"
    h.wait_for_self(
        |hh| hh.input_value() == "old2",
        "input value to become \"old2\" after Shift+Up history recall",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Tab — turn-selection mode (placeholder, ignored)
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (Tab → turn-selection-mode)
/// DEEP-REF: (none — Rust AgentView has no Tab handler today)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Tab turn-selection mode pending future RPC card — placeholder behaviour-parity assertion documented but not yet wired in the Rust AgentView"]
async fn key_tab_enters_turn_selection_mode() {
    // @step Given a fresh AppTestHarness with focused session s-1 seeded with 3 scrollback chunks
    let mut h = AppTestHarness::new();
    h.seed_chunks(&seed_sid(), 3);

    // @step When I press Tab
    h.press_key(AppTestHarness::key(KeyCode::Tab));

    // @step Then the AgentView is in turn-selection mode
    // @step And a turn-selection cursor is visible
    //
    // Placeholder — the Rust AgentView has no `turn_selection_mode`
    // store field today. When the future RPC card lands the assertion
    // becomes:
    //
    //     assert!(h.app.agent_view_store().turn_selection_mode());
    //     assert!(h.app.agent_view_store().turn_selection_cursor().is_some());
    //
    // Until then this test is `#[ignore]`d so the matrix entry stays
    // breadcrumb-tracked.
    panic!("placeholder test — remove #[ignore] when Tab turn-selection lands");
}

// ─────────────────────────────────────────────────────────────────────────
// Ctrl+R — opens SearchHistoryView with no backend call
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx line ~2721 (Ctrl+R → setIsSearchMode)
/// DEEP-REF: tests/search_view_rpc064.rs::pressing_ctrl_r_opens_empty_view
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_ctrl_r_opens_search_view_without_backend_call() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I press Ctrl+R
    h.press_key(AppTestHarness::key_ctrl_r());
    h.drain_pending().await;

    // @step Then the AgentView's search_view is Some(_)
    assert!(h.search_view_active());

    // @step And MockBackend.search_history_calls() is 0
    assert_eq!(h.mock.search_history_calls(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Esc — single dialog pop (level 2 of the 5-level cascade)
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (Esc cascade level 2)
/// DEEP-REF: tests/keyboard_cascade_rpc051.rs (full 5-level cascade)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_esc_with_one_help_dialog_pops_that_dialog() {
    // @step Given a fresh AppTestHarness with focused session s-1 and a HelpDialog on the compositor
    let mut h = AppTestHarness::new();
    h.set_input("preserved-input");
    h.dispatch_slash(SlashCommandAction::Help);
    assert!(h.compositor_contains("help-dialog"));

    // @step When I press Esc
    h.press_key(AppTestHarness::key(KeyCode::Esc));

    // @step Then the compositor does NOT contain a layer with id "help-dialog"
    assert!(!h.compositor_contains("help-dialog"));

    // @step And the input value is unchanged
    assert_eq!(h.input_value(), "preserved-input");
}

// ─────────────────────────────────────────────────────────────────────────
// Enter (plain text) — forwards to backend.send_input
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (handleSubmit → backend.send_input)
/// DEEP-REF: tests/app_dispatch_rpc020.rs::handle_input_submitted_plain_text
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_enter_on_plain_text_forwards_to_backend_send_input() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mut h = AppTestHarness::new();

    // @step When I submit the input text "hello world"
    h.submit_input("hello world");

    // @step Then within 1 second MockBackend.send_input_calls() is 1
    h.wait_for_mock(
        |m| m.send_input_calls() == 1,
        "MockBackend.send_input_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_send_input() is Some((s-1, "hello world"))
    assert_eq!(
        h.mock.last_send_input(),
        Some((seed_sid(), "hello world".to_string()))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Ctrl+C — calls backend.interrupt when session is Running
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (Ctrl+C → backend.interrupt)
/// DEEP-REF: tests/keyboard_cascade_rpc051.rs + tests/app_dispatch_rpc024.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_ctrl_c_while_running_calls_backend_interrupt() {
    // @step Given a fresh AppTestHarness with focused session s-1
    // @step And the focused session's status is SessionStatus::Running
    let mut h = AppTestHarness::new();
    h.app
        .agent_view_store_mut()
        .set_session_status(seed_sid(), SessionStatus::Running);

    // @step When I press Ctrl+C
    h.press_key(AppTestHarness::key_ctrl_c());

    // @step Then within 1 second MockBackend.interrupt_calls() is 1
    h.wait_for_mock(
        |m| m.interrupt_calls() == 1,
        "MockBackend.interrupt_calls() == 1",
    )
    .await;

    // @step And MockBackend.last_interrupt() is Some(s-1)
    assert_eq!(h.mock.last_interrupt(), Some(seed_sid()));
}

// ─────────────────────────────────────────────────────────────────────────
// PageDown / End — scrollback viewport navigation
// ─────────────────────────────────────────────────────────────────────────

/// TS-REF: src/tui/views/AgentView.tsx (PageDown/End → scrollback viewport)
/// DEEP-REF: tests/view_agent_scrollback_rpc019.rs
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_pagedown_and_end_navigate_scrollback_viewport() {
    // @step Given a fresh AppTestHarness with focused session s-1 seeded with 30 scrollback chunks
    let mut h = AppTestHarness::new();
    h.seed_chunks(&seed_sid(), 30);
    // Seed a known viewport height so the page-size arithmetic in
    // App::dispatch (which reads `scrollback_viewport_hint()`) is
    // deterministic. With 30 chunks and a 10-row viewport the offset
    // moves by 10 chunks per PageUp/PageDown.
    h.set_scrollback_viewport_height(&seed_sid(), 10);

    // @step And the scrollback is scrolled to the top
    //
    // The seed_chunks helper pushes chunks while stick_to_bottom is
    // true, so the offset auto-tails to the bottom. Pressing PageUp
    // drops out of stick mode and rewinds the offset by one page —
    // two PageUps from the bottom (chunks=30, viewport=10) parks the
    // offset at 0. The press_key emits Action::ScrollbackPageUp via
    // action_tx; drain_pending() folds that action through dispatch
    // so the scrollback state actually updates.
    h.press_key(AppTestHarness::key(KeyCode::PageUp));
    h.drain_pending().await;
    h.press_key(AppTestHarness::key(KeyCode::PageUp));
    h.drain_pending().await;
    assert!(
        !h.scrollback_stick_to_bottom(&seed_sid()),
        "expected PageUp to drop out of stick-to-bottom mode",
    );
    let offset_at_top = h.scrollback_offset(&seed_sid());
    assert_eq!(
        offset_at_top, 0,
        "expected offset to rewind to top, got {offset_at_top}",
    );

    // @step When I press PageDown
    h.press_key(AppTestHarness::key(KeyCode::PageDown));
    h.drain_pending().await;

    // @step Then the scrollback viewport has advanced by one page
    let offset_after_pagedown = h.scrollback_offset(&seed_sid());
    assert!(
        offset_after_pagedown > offset_at_top,
        "expected PageDown to advance offset past {offset_at_top}, got {offset_after_pagedown}",
    );

    // @step When I press End
    h.press_key(AppTestHarness::key(KeyCode::End));
    h.drain_pending().await;

    // @step Then the scrollback is at the bottom
    assert!(
        h.scrollback_stick_to_bottom(&seed_sid()),
        "expected End to re-stick scrollback to bottom",
    );
}
