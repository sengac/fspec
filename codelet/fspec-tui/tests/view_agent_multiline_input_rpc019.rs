//! RPC-019 — MultiLineInput unit tests (red phase).
//!
//! Feature: spec/features/rpc019-multiline-input.feature
//!
//! Drives the behavior-level scenarios for the new tui-textarea-backed
//! MultiLineInput widget: plain-Enter submit, Shift+Enter newline,
//! bracketed-paste preservation, auto-grow cap, Shift+arrow chord
//! forwarding, ESC dispatch, placeholder hint rendering.
//!
//! NOTE: these tests REQUIRE the RPC-019 implementation — they will
//! fail to compile until `MultiLineInput`, `InputEventOutcome` and the
//! four new Action variants land. That is intentional (red phase).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::multiline_input::{
    InputEventOutcome, MultiLineInput,
};
use codelet_fspec_tui::{Action, AgentView, AgentViewStore, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

fn fresh_view() -> (AgentView, UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    (AgentView::new(tx), rx)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn type_chars(view: &mut AgentView, s: &str) {
    for ch in s.chars() {
        view.handle_event(&key(KeyCode::Char(ch), KeyModifiers::NONE));
    }
}

fn render_rows(width: u16, height: u16, store: &AgentViewStore, view: &mut AgentView) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

/// Scenario: Plain Enter on the multi-line input submits and resets the buffer
#[test]
fn plain_enter_on_multi_line_input_submits_and_resets_the_buffer() {
    // @step Given an AgentView with an empty MultiLineInput
    let (mut view, mut rx) = fresh_view();
    assert!(view.input.is_empty());

    // @step When the user types "hello world" then presses plain Enter
    type_chars(&mut view, "hello world");
    let _ = view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::InputSubmitted("hello world")
    let action = rx.try_recv().expect("Action::InputSubmitted on bus");
    match action {
        Action::InputSubmitted(s) => assert_eq!(s, "hello world"),
        other => panic!("expected InputSubmitted, got {other:?}"),
    }

    // @step And the MultiLineInput's buffer is empty after submit
    assert!(view.input.is_empty(), "buffer should be empty after submit");

    // @step And the MultiLineInput's visible-row count is 1
    assert_eq!(view.input.visible_rows(), 1);
}

/// Scenario: Shift+Enter inserts a literal newline instead of submitting
#[test]
fn shift_enter_inserts_a_literal_newline_instead_of_submitting() {
    // @step Given an AgentView with an empty MultiLineInput
    let (mut view, mut rx) = fresh_view();

    // @step When the user types "hello" then presses Shift+Enter then types "world"
    type_chars(&mut view, "hello");
    let _ = view.handle_event(&key(KeyCode::Enter, KeyModifiers::SHIFT));
    type_chars(&mut view, "world");

    // @step Then no Action::InputSubmitted is emitted yet
    assert!(
        rx.try_recv().is_err(),
        "no submit Action expected after Shift+Enter"
    );

    // @step And the MultiLineInput's buffer is exactly "hello\nworld"
    assert_eq!(view.input.value(), "hello\nworld");

    // @step And the MultiLineInput's visible-row count is 2
    assert_eq!(view.input.visible_rows(), 2);
}

/// Scenario: Plain Enter submits a multi-line buffer with embedded newlines
#[test]
fn plain_enter_submits_a_multi_line_buffer_with_embedded_newlines() {
    // @step Given an AgentView whose MultiLineInput contains "hello\nworld"
    let (mut view, mut rx) = fresh_view();
    view.input.set_value("hello\nworld");
    assert_eq!(view.input.value(), "hello\nworld");

    // @step When the user presses plain Enter
    let _ = view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then AgentView emits Action::InputSubmitted("hello\nworld")
    let action = rx.try_recv().expect("Action::InputSubmitted on bus");
    match action {
        Action::InputSubmitted(s) => assert_eq!(s, "hello\nworld"),
        other => panic!("expected InputSubmitted, got {other:?}"),
    }

    // @step And the MultiLineInput's buffer is empty after submit
    assert!(view.input.is_empty());
}

/// Scenario: Pasted text with embedded newlines preserves them in the buffer
#[test]
fn pasted_text_with_embedded_newlines_preserves_them_in_the_buffer() {
    // @step Given an AgentView with an empty MultiLineInput
    let (mut view, _rx) = fresh_view();

    // @step When the MultiLineInput is fed the bracketed-paste payload "line-a\nline-b\nline-c"
    view.handle_event(&Event::Paste("line-a\nline-b\nline-c".to_string()));

    // @step Then the MultiLineInput's buffer is exactly "line-a\nline-b\nline-c"
    assert_eq!(view.input.value(), "line-a\nline-b\nline-c");

    // @step And the MultiLineInput's visible-row count is 3
    assert_eq!(view.input.visible_rows(), 3);
}

/// Scenario: MultiLineInput auto-grows up to its max visible rows cap of 6
#[test]
fn multi_line_input_auto_grows_up_to_its_max_visible_rows_cap_of_6() {
    // @step Given an AgentView with MultiLineInput max_visible_rows = 6
    let (mut view, _rx) = fresh_view();
    view.input = MultiLineInput::with_max_visible_rows(6);

    // @step When the user inserts 8 newlines back-to-back
    for _ in 0..8 {
        let _ = view.handle_event(&key(KeyCode::Enter, KeyModifiers::SHIFT));
    }

    // @step Then the MultiLineInput's visible-row count is exactly 6
    assert_eq!(view.input.visible_rows(), 6);

    // @step And the MultiLineInput's logical line count is 9
    assert_eq!(view.input.line_count(), 9);
}

/// Scenario: Empty MultiLineInput paints the dim placeholder hint with a green > prefix
#[test]
fn empty_multi_line_input_paints_the_dim_placeholder_hint_with_a_green_prefix() {
    // @step Given an AgentView whose MultiLineInput is empty
    let (mut view, _rx) = fresh_view();
    let store = AgentViewStore::default();

    // @step When the App renders AgentView against a 100x12 TestBackend
    let rows = render_rows(100, 12, &store, &mut view);
    let input_row: String = rows.iter().find(|r| r.contains("Type a message...")).cloned().unwrap_or_default();

    // @step Then the rendered buffer's input row contains the substring "> Type a message..."
    assert!(input_row.contains("> Type a message..."), "input row missing prompt + hint: {input_row:?}");
    // @step And the rendered buffer's input row contains the substring "'Shift+↑/↓' history"
    assert!(input_row.contains("'Shift+↑/↓' history"), "input row missing history hint: {input_row:?}");
    // @step And the rendered buffer's input row contains the substring "'Shift+←/→' sessions"
    assert!(input_row.contains("'Shift+←/→' sessions"), "input row missing sessions hint: {input_row:?}");
    // @step And the rendered buffer's input row contains the substring "'Tab' select turn"
    assert!(input_row.contains("'Tab' select turn"), "input row missing turn hint: {input_row:?}");
}

/// Scenario: Non-empty MultiLineInput hides the placeholder hint
#[test]
fn non_empty_multi_line_input_hides_the_placeholder_hint() {
    // @step Given an AgentView whose MultiLineInput contains "draft"
    let (mut view, _rx) = fresh_view();
    view.input.set_value("draft");
    let store = AgentViewStore::default();

    // @step When the App renders AgentView against a 100x12 TestBackend
    let rows = render_rows(100, 12, &store, &mut view);
    let joined = rows.join("\n");

    // @step Then the rendered buffer's input area contains the substring "draft"
    assert!(joined.contains("draft"), "input area should display 'draft'; got:\n{joined}");
    // @step And the rendered buffer does NOT contain the substring "Type a message..."
    assert!(!joined.contains("Type a message..."), "placeholder should be hidden when input is non-empty");
}

/// Scenario: Shift+Up emits Action::HistoryPrev without modifying the buffer
#[test]
fn shift_up_emits_action_history_prev_without_modifying_the_buffer() {
    // @step Given an AgentView whose MultiLineInput contains "draft"
    let (mut view, mut rx) = fresh_view();
    view.input.set_value("draft");

    // @step When the user presses Shift+Up
    let _ = view.handle_event(&key(KeyCode::Up, KeyModifiers::SHIFT));

    // @step Then AgentView emits Action::HistoryPrev
    let action = rx.try_recv().expect("Action::HistoryPrev on bus");
    assert!(matches!(action, Action::HistoryPrev), "expected HistoryPrev, got {action:?}");

    // @step And the MultiLineInput's buffer is still exactly "draft"
    assert_eq!(view.input.value(), "draft");
}

/// Scenario: Shift+Down emits Action::HistoryNext without modifying the buffer
#[test]
fn shift_down_emits_action_history_next_without_modifying_the_buffer() {
    // @step Given an AgentView whose MultiLineInput contains "draft"
    let (mut view, mut rx) = fresh_view();
    view.input.set_value("draft");

    // @step When the user presses Shift+Down
    let _ = view.handle_event(&key(KeyCode::Down, KeyModifiers::SHIFT));

    // @step Then AgentView emits Action::HistoryNext
    let action = rx.try_recv().expect("Action::HistoryNext on bus");
    assert!(matches!(action, Action::HistoryNext), "expected HistoryNext, got {action:?}");

    // @step And the MultiLineInput's buffer is still exactly "draft"
    assert_eq!(view.input.value(), "draft");
}

/// Scenario: Shift+Left emits Action::SessionPrev without modifying the buffer
#[test]
fn shift_left_emits_action_session_prev_without_modifying_the_buffer() {
    // @step Given an AgentView whose MultiLineInput contains "draft"
    let (mut view, mut rx) = fresh_view();
    view.input.set_value("draft");

    // @step When the user presses Shift+Left
    let _ = view.handle_event(&key(KeyCode::Left, KeyModifiers::SHIFT));

    // @step Then AgentView emits Action::SessionPrev
    let action = rx.try_recv().expect("Action::SessionPrev on bus");
    assert!(matches!(action, Action::SessionPrev), "expected SessionPrev, got {action:?}");

    // @step And the MultiLineInput's buffer is still exactly "draft"
    assert_eq!(view.input.value(), "draft");
}

/// Scenario: Shift+Right emits Action::SessionNext without modifying the buffer
#[test]
fn shift_right_emits_action_session_next_without_modifying_the_buffer() {
    // @step Given an AgentView whose MultiLineInput contains "draft"
    let (mut view, mut rx) = fresh_view();
    view.input.set_value("draft");

    // @step When the user presses Shift+Right
    let _ = view.handle_event(&key(KeyCode::Right, KeyModifiers::SHIFT));

    // @step Then AgentView emits Action::SessionNext
    let action = rx.try_recv().expect("Action::SessionNext on bus");
    assert!(matches!(action, Action::SessionNext), "expected SessionNext, got {action:?}");

    // @step And the MultiLineInput's buffer is still exactly "draft"
    assert_eq!(view.input.value(), "draft");
}

/// Scenario: ESC inside AgentView emits Action::BackToBoard
#[test]
fn esc_inside_agent_view_emits_action_back_to_board() {
    // @step Given an AgentView whose MultiLineInput contains "draft\nstill drafting"
    let (mut view, mut rx) = fresh_view();
    view.input.set_value("draft\nstill drafting");

    // @step When the user presses ESC
    let result = view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(result, EventResult::Consumed(_)), "ESC should be Consumed");

    // @step Then AgentView emits Action::BackToBoard
    let action = rx.try_recv().expect("Action::BackToBoard on bus");
    assert!(matches!(action, Action::BackToBoard), "expected BackToBoard, got {action:?}");

    // @step And the MultiLineInput's buffer is still exactly "draft\nstill drafting"
    assert_eq!(view.input.value(), "draft\nstill drafting");
}

/// MultiLineInput::handle_key directly should surface Submitted /
/// Continued / Ignored without going through AgentView's wrapper.
#[test]
fn input_event_outcome_distinguishes_submitted_continued_ignored() {
    let mut input = MultiLineInput::new();
    // Continued — typing a printable char
    let outcome = input.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    assert!(matches!(outcome, InputEventOutcome::Continued), "char => Continued; got {outcome:?}");
    // Continued — Shift+Enter
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    assert!(matches!(outcome, InputEventOutcome::Continued), "Shift+Enter => Continued; got {outcome:?}");
    // Submitted — plain Enter
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    match outcome {
        InputEventOutcome::Submitted(s) => assert_eq!(s, "a\n"),
        other => panic!("plain Enter should Submit; got {other:?}"),
    }
    // Ignored — Shift+Up (caller forwards as HistoryPrev)
    let outcome = input.handle_key(KeyCode::Up, KeyModifiers::SHIFT);
    assert!(matches!(outcome, InputEventOutcome::Ignored), "Shift+Up => Ignored; got {outcome:?}");
}
