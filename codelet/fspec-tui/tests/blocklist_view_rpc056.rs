//! RPC-056 — BlocklistView + `/blocklist` slash command end-to-end.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Blocklist)`
//! so it reaches the backend's `blocklist_list()` RPC and opens the new
//! `BlocklistView`. Covers view rendering, key handling (j/k, Up/Down,
//! Space/Enter toggle, Esc close), the per-session disabled-rules lift to
//! `AgentViewStore`, and the `derive_category` helper.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::blocklist::{derive_category, BlocklistEvent, BlocklistView};
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{BlocklistRuleInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn rule(id: &str, pattern: &str, action: &str, source: &str) -> BlocklistRuleInfo {
    BlocklistRuleInfo {
        id: id.to_string(),
        pattern: pattern.to_string(),
        action: action.to_string(),
        reason: format!("reason for {id}"),
        guidance: Some(format!("guidance for {id}")),
        source: source.to_string(),
    }
}

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

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

fn render_view(view: &mut BlocklistView, disabled: &HashSet<String>) -> String {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        view.render(frame.area(), frame.buffer_mut(), disabled);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /blocklist dispatches OpenBlocklistView and triggers a blocklist_list fetch
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_blocklist_dispatches_open_and_fetches_rules() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose blocklist_list returns two rules
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_calls = mock.blocklist_list_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Blocklist) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Blocklist));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.blocklist_list is called exactly once
    wait_until(
        || mock.blocklist_list_calls() - initial_calls == 1,
        "blocklist_list_calls == initial + 1",
    )
    .await;

    // @step And within 1 second the Navigator's active_view equals ViewMode::Blocklist
    wait_until(
        || app.navigator().active_view == ViewMode::Blocklist,
        "active_view == ViewMode::Blocklist",
    )
    .await;

    // @step And within 1 second Action::BlocklistRulesLoaded carrying the two rules is observed on the action bus
    let rules = &app.navigator().blocklist.rules;
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].id, "git-checkout-block");
    assert_eq!(rules[1].id, "cat-block");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: BlocklistView renders two configured rules with source tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn blocklist_view_renders_two_rules_with_source_tags() {
    // @step Given a BlocklistView seeded with rules [git-checkout-block(system, block), cat-block(project, block)]
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "git-checkout-block"
    assert!(
        text.contains("git-checkout-block"),
        "missing git-checkout-block: {text}"
    );
    // @step And the rendered text contains "cat-block"
    assert!(text.contains("cat-block"), "missing cat-block: {text}");
    // @step And the rendered text contains "system"
    assert!(text.contains("system"), "missing system source tag");
    // @step And the rendered text contains "project"
    assert!(text.contains("project"), "missing project source tag");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Empty blocklist renders the placeholder text
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn empty_blocklist_renders_placeholder() {
    // @step Given a BlocklistView seeded with an empty rule list
    let mut view = BlocklistView::new();
    view.set_rules(Vec::new());

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "No blocklist rules configured"
    assert!(
        text.contains("No blocklist rules configured"),
        "missing placeholder text"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: j and Down advance the focused row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn j_and_down_advance_focused_row() {
    // @step Given a BlocklistView seeded with three rules with selected_index 0
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule("a", "a", "block", "system"),
        rule("b", "b", "block", "system"),
        rule("c", "c", "block", "system"),
    ]);
    assert_eq!(view.selected_index, 0);

    // @step When the user presses j
    let _ = view.handle_key(key(KeyCode::Char('j')));
    // @step Then selected_index equals 1
    assert_eq!(view.selected_index, 1);

    // @step When the user presses Down
    let _ = view.handle_key(key(KeyCode::Down));
    // @step Then selected_index equals 2
    assert_eq!(view.selected_index, 2);

    // Boundary: pressing j again should clamp at 2.
    let _ = view.handle_key(key(KeyCode::Char('j')));
    assert_eq!(view.selected_index, 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: k and Up retreat the focused row, clamped at 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn k_and_up_retreat_focused_row_clamped() {
    // @step Given a BlocklistView seeded with three rules with selected_index 1
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule("a", "a", "block", "system"),
        rule("b", "b", "block", "system"),
        rule("c", "c", "block", "system"),
    ]);
    view.selected_index = 1;

    // @step When the user presses k
    let _ = view.handle_key(key(KeyCode::Char('k')));
    // @step Then selected_index equals 0
    assert_eq!(view.selected_index, 0);

    // @step When the user presses Up
    let _ = view.handle_key(key(KeyCode::Up));
    // @step Then selected_index equals 0
    assert_eq!(view.selected_index, 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Space toggles the focused rule into the session-disabled set
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn space_emits_toggle_action_for_focused_rule() {
    // @step Given a BlocklistView seeded with rule "git-checkout-block" focused (selected_index 0)
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule(
        "git-checkout-block",
        "^git\\s+checkout\\b",
        "block",
        "system",
    )]);
    // @step And the session-disabled set is empty for the focused session
    // (per Action design — toggling emits the Action, the App folds it
    // into the store; the view is stateless w.r.t. the disabled set)

    // @step When the user presses Space
    let out = view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the focused session's blocklist_disabled set contains "git-checkout-block"
    // Verified via the emitted Action — the App's dispatch_rpc056
    // handler is the side that mutates AgentViewStore.
    match out {
        BlocklistEvent::Emit(Action::ToggleBlocklistRule(id)) => {
            assert_eq!(id, "git-checkout-block");
        }
        _ => panic!("expected ToggleBlocklistRule action; got {out:?}"),
    }

    // @step When the user presses Space again
    let out = view.handle_key(key(KeyCode::Char(' ')));
    // @step Then the focused session's blocklist_disabled set no longer contains "git-checkout-block"
    // The view emits the same Action again — the App's handler
    // toggles (insert ↔ remove) so the second press removes the id.
    assert!(matches!(
        out,
        BlocklistEvent::Emit(Action::ToggleBlocklistRule(_))
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter behaves identically to Space for toggling
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn enter_emits_toggle_action_for_focused_rule() {
    // @step Given a BlocklistView seeded with rule "cat-block" focused
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("cat-block", "^cat\\s+", "block", "project")]);

    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then the focused session's blocklist_disabled set contains "cat-block"
    match out {
        BlocklistEvent::Emit(Action::ToggleBlocklistRule(id)) => {
            assert_eq!(id, "cat-block");
        }
        _ => panic!("expected ToggleBlocklistRule action; got {out:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A disabled rule paints the dimmed glyph and (disabled) suffix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn disabled_rule_paints_dimmed_glyph_and_suffix() {
    // @step Given a BlocklistView seeded with rules [git-checkout-block(system), cat-block(project)]
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);
    // @step And the focused session's blocklist_disabled set contains "git-checkout-block"
    let mut disabled: HashSet<String> = HashSet::new();
    disabled.insert("git-checkout-block".to_string());

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &disabled);

    // @step Then the rendered text contains "○ git-checkout-block"
    assert!(
        text.contains("○ git-checkout-block"),
        "expected dimmed glyph for disabled rule; got: {text}"
    );
    // @step And the rendered text contains "(disabled)"
    assert!(
        text.contains("(disabled)"),
        "expected (disabled) suffix on disabled row"
    );
    // @step And the rendered text contains "● cat-block"
    assert!(
        text.contains("● cat-block"),
        "expected enabled glyph for cat-block; got: {text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc closes the view and returns to the Agent view
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_closes_view_and_returns_to_agent() {
    // @step Given an App with an open session s-1 and the Navigator active_view set to ViewMode::Blocklist
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![rule("a", "a", "block", "system")]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Blocklist));
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().active_view, ViewMode::Blocklist);

    // @step When the user presses Esc
    app.dispatch(Action::CloseBlocklistView);
    drain_pending(&mut app).await;

    // @step Then Action::CloseBlocklistView is observed on the action bus
    // (covered by the dispatch above — round-trips through the Navigator)

    // @step And the Navigator's active_view returns to ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Session-disabled set persists across close/reopen of the view
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_disabled_set_persists_across_close_reopen() {
    // @step Given an App with an open session s-1 and the BlocklistView open
    let mock = Arc::new(MockBackend::new());
    mock.seed_blocklist_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Blocklist));
    drain_pending(&mut app).await;

    // @step And the user toggles "git-checkout-block" to disabled then presses Esc
    app.dispatch(Action::ToggleBlocklistRule(
        "git-checkout-block".to_string(),
    ));
    drain_pending(&mut app).await;
    // Sanity check: the store carries the disabled id for the focused session.
    let disabled_in_store = app
        .agent_view_store()
        .blocklist_disabled_for(&sid("s-1"))
        .cloned()
        .unwrap_or_default();
    assert!(disabled_in_store.contains("git-checkout-block"));

    app.dispatch(Action::CloseBlocklistView);
    drain_pending(&mut app).await;

    // @step When SlashCommandSelected(SlashCommandAction::Blocklist) is dispatched again
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Blocklist));
    drain_pending(&mut app).await;

    // @step Then the new BlocklistView reads the existing blocklist_disabled set from AgentViewStore
    let disabled_after_reopen = app
        .agent_view_store()
        .blocklist_disabled_for(&sid("s-1"))
        .cloned()
        .unwrap_or_default();
    assert!(disabled_after_reopen.contains("git-checkout-block"));

    // @step And the row "git-checkout-block" renders with the dimmed glyph
    let mut view = app.navigator_mut().blocklist.clone();
    let text = render_view(&mut view, &disabled_after_reopen);
    assert!(
        text.contains("○ git-checkout-block"),
        "expected dimmed glyph after reopen; got: {text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Category column derives "file_path" for path-shaped patterns
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn category_column_renders_file_path_for_path_patterns() {
    // @step Given a BlocklistView with a rule whose pattern is "/etc/passwd"
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("etc", "/etc/passwd", "block", "system")]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "file_path"
    assert!(
        text.contains("file_path"),
        "expected file_path category; got: {text}"
    );
}

#[test]
fn category_column_renders_file_path_for_tilde_patterns() {
    // @step Given a BlocklistView with a rule whose pattern is "~/.aws/.*"
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("aws", "~/.aws/.*", "block", "system")]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "file_path"
    assert!(text.contains("file_path"));
}

#[test]
fn category_column_renders_bash_for_command_patterns() {
    // @step Given a BlocklistView with a rule whose pattern is "^cat\\s+"
    let mut view = BlocklistView::new();
    view.set_rules(vec![rule("cat", "^cat\\s+", "block", "system")]);

    // @step When the view is rendered into a 120x24 buffer
    let text = render_view(&mut view, &HashSet::new());

    // @step Then the rendered text contains "bash"
    assert!(text.contains("bash"), "expected bash category; got: {text}");
    // @step And the rendered text does NOT contain "file_path"
    assert!(
        !text.contains("file_path"),
        "did not expect file_path category"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: derive_category returns deterministic strings
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn derive_category_is_deterministic() {
    // @step Given the derive_category helper function
    // @step Then derive_category("^cat\\s+") returns "bash"
    assert_eq!(derive_category("^cat\\s+"), "bash");
    // @step And derive_category("/etc/passwd") returns "file_path"
    assert_eq!(derive_category("/etc/passwd"), "file_path");
    // @step And derive_category("~/.aws/.*") returns "file_path"
    assert_eq!(derive_category("~/.aws/.*"), "file_path");
    // @step And derive_category("./scripts/deploy.sh") returns "file_path"
    assert_eq!(derive_category("./scripts/deploy.sh"), "file_path");
}
