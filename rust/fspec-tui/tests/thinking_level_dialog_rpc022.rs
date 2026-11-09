//! RPC-022 — ThinkingLevelDialog component unit tests.
//!
//! Feature: spec/features/rpc022-thinking-level-dialog.feature
//!
//! Drives the Priority::Foreground modal dialog through its public
//! Component surface: `priority()`, `render()`, `handle_event()`, and
//! the test-only `take_pending_action()` accessor. Mirrors the
//! existing HelpDialog / DisconnectDialog component tests.

mod common;

use codelet_fspec_tui::{
    Action, Compositor, EventResult, Priority, ThinkingLevelDialog, THINKING_LEVEL_DIALOG_ID,
};
use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Render `dialog` against an 80x24 TestBackend and return the buffer
/// as a single \n-delimited String for substring assertions.
fn render_to_string(
    dialog: &mut ThinkingLevelDialog,
) -> Result<String, codelet_test_helpers::TestError> {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).or_fail("Terminal::new")?;
    term.draw(|frame| {
        use codelet_fspec_tui::Component;
        Component::render(dialog, frame.area(), frame.buffer_mut());
    })
    .or_fail("draw")?;
    let buf: Buffer = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    Ok(out)
}

/// Scenario: ThinkingLevelDialog renders at Priority::Foreground
#[test]
fn thinking_level_dialog_renders_at_priority_foreground() -> TestResult {
    // @step Given a fresh ThinkingLevelDialog with id "thinking-level-dialog"
    let dialog = ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Off,
        common::off_low_medium_high_options(),
    );
    use codelet_fspec_tui::Component;
    assert_eq!(dialog.id(), THINKING_LEVEL_DIALOG_ID);
    // @step When its priority() method is invoked
    let prio = dialog.priority();
    // @step Then the result is Priority::Foreground
    assert_eq!(prio, Priority::Foreground);
    Ok(())
}

/// Scenario: ThinkingLevelDialog renders the four canonical labels
#[test]
fn thinking_level_dialog_renders_the_four_canonical_labels() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Off
    let mut dialog = ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Off,
        common::off_low_medium_high_options(),
    );
    // @step When the dialog is rendered onto an 80x24 TestBackend
    let painted = render_to_string(&mut dialog)?;
    // @step Then the rendered buffer contains the substring "Thinking Level"
    assert!(painted.contains("Thinking Level"));
    // @step And the rendered buffer contains the substring "Off"
    assert!(painted.contains("Off"));
    // @step And the rendered buffer contains the substring "low"
    assert!(painted.contains("low"));
    // @step And the rendered buffer contains the substring "medium"
    assert!(painted.contains("medium"));
    // @step And the rendered buffer contains the substring "high"
    assert!(painted.contains("high"));
    // @step And the production source uses the shared dialog_theme renderer
    // (RPC-027 replaced the tui_popup::Popup adapter with dialog_theme::render_dialog;
    // the original RPC-022 SizedWidgetRef adapter contract has been superseded.)
    let src = common::read_to_string_or_fail(
        &common::workspace_root()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("thinking_level_dialog.rs"),
    )?;
    assert!(
        src.contains("render_dialog"),
        "production source must use dialog_theme::render_dialog"
    );
    // @step And the production source does NOT define a hand-rolled centered_rect helper
    assert!(
        !src.contains("fn centered_rect"),
        "production source must NOT define a centered_rect helper"
    );
    Ok(())
}

/// Scenario: Dialog opens with the currently-active level pre-selected
#[test]
fn dialog_opens_with_currently_active_level_pre_selected() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Effort("medium")
    let mut dialog = ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Effort("medium".to_string()),
        common::off_low_medium_high_options(),
    );
    // @step When the dialog is rendered
    let painted = render_to_string(&mut dialog)?;
    // @step Then the Medium row is rendered with the selection marker "▸"
    let medium_row = painted
        .lines()
        .find(|l| l.contains("medium"))
        .or_fail("Medium row not found")?;
    assert!(
        medium_row.contains('▸'),
        "Medium row missing ▸ marker, got {medium_row:?}"
    );
    // @step And the Off / Low / High rows are rendered without the selection marker
    for label in ["Off", "low", "high"] {
        let row = painted
            .lines()
            .find(|l| l.contains(label) && !l.contains("Thinking Level"))
            .or_fail(&format!("{label} row not found"))?;
        assert!(
            !row.contains('▸'),
            "{label} row should not carry ▸ marker, got {row:?}"
        );
    }
    Ok(())
}

/// Scenario Outline: Arrow keys navigate the 5 levels with wrap-around
/// (BUG-455 appended "Model default").
#[test]
fn arrow_keys_navigate_the_5_levels_with_wrap_around() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Off
    let cases: &[(KeyCode, usize, ThinkingLevel)] = &[
        // Down navigation:
        (KeyCode::Down, 1, ThinkingLevel::Effort("low".to_string())),
        (
            KeyCode::Down,
            2,
            ThinkingLevel::Effort("medium".to_string()),
        ),
        (KeyCode::Down, 3, ThinkingLevel::Effort("high".to_string())),
        (KeyCode::Down, 4, ThinkingLevel::ModelDefault),
        (KeyCode::Down, 5, ThinkingLevel::Off),
        // Up navigation (with wrap-around at index 0):
        (KeyCode::Up, 1, ThinkingLevel::ModelDefault),
        (KeyCode::Up, 2, ThinkingLevel::Effort("high".to_string())),
        (KeyCode::Up, 5, ThinkingLevel::Off),
    ];
    use codelet_fspec_tui::Component;
    for (k, count, expected) in cases {
        let mut dialog = ThinkingLevelDialog::new(
            SessionId::new("s-1"),
            ThinkingLevel::Off,
            common::off_low_medium_high_options(),
        );
        // @step When the user presses <key> <count> times
        for _ in 0..*count {
            let _ = dialog.handle_event(&key(*k));
        }
        // @step Then the highlighted row has label <expected>
        assert_eq!(
            dialog.selected_level(),
            Some(expected.clone()),
            "key {k:?} x{count} → expected {expected:?}, got {:?}",
            dialog.selected_level()
        );
    }
    Ok(())
}

/// Scenario: Enter on a level emits Action::ThinkingLevelSelected
#[test]
fn enter_on_a_level_emits_action_thinking_level_selected() -> TestResult {
    use codelet_fspec_tui::Component;
    // @step Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Off
    // @step And the dialog was constructed against SessionId::new("s-1")
    let mut dialog = ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Off,
        common::off_low_medium_high_options(),
    );
    // @step When the user navigates Down 3 times so High is highlighted
    for _ in 0..3 {
        let _ = dialog.handle_event(&key(KeyCode::Down));
    }
    assert_eq!(
        dialog.selected_level(),
        Some(ThinkingLevel::Effort("high".to_string()))
    );
    // @step And the user presses Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));
    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        other => {
            return Err(format!(
                "expected Consumed(Some(callback)), got {:?}",
                std::mem::discriminant(&other)
            )
            .into())
        }
    };
    // @step And the callback emits Action::ThinkingLevelSelected(SessionId::new("s-1"), ThinkingLevel::Effort("high"))
    let action = dialog
        .take_pending_action()
        .or_fail("pending action must be set")?;
    match action {
        Action::ThinkingLevelSelected(sid, level) => {
            assert_eq!(sid, SessionId::new("s-1"));
            assert_eq!(level, ThinkingLevel::Effort("high".to_string()));
        }
        other => return Err(format!("expected ThinkingLevelSelected, got {other:?}").into()),
    }
    // @step And the callback removes the dialog from the Compositor via its id
    let mut compositor = Compositor::new();
    compositor.push(Box::new(ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Off,
        common::off_low_medium_high_options(),
    )));
    callback(&mut compositor);
    assert!(
        !compositor.contains(THINKING_LEVEL_DIALOG_ID),
        "callback must remove the dialog from the Compositor"
    );
    Ok(())
}

/// Scenario: Esc dismisses the ThinkingLevelDialog without side effects
#[test]
fn esc_dismisses_the_thinking_level_dialog_without_side_effects() -> TestResult {
    use codelet_fspec_tui::Component;
    // @step Given a ThinkingLevelDialog seeded with current_level = ThinkingLevel::Effort("high")
    let mut dialog = ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Effort("high".to_string()),
        common::off_low_medium_high_options(),
    );
    // @step When the user presses Esc
    let result = dialog.handle_event(&key(KeyCode::Esc));
    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        other => {
            return Err(format!(
                "expected Consumed(Some(callback)), got {:?}",
                std::mem::discriminant(&other)
            )
            .into())
        }
    };
    // @step And the callback removes the dialog from the Compositor via its id
    let mut compositor = Compositor::new();
    compositor.push(Box::new(ThinkingLevelDialog::new(
        SessionId::new("s-1"),
        ThinkingLevel::Effort("high".to_string()),
        common::off_low_medium_high_options(),
    )));
    callback(&mut compositor);
    assert!(
        !compositor.contains(THINKING_LEVEL_DIALOG_ID),
        "callback must remove the dialog from the Compositor"
    );
    // @step And no Action::ThinkingLevelSelected is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "Esc must not emit any pending action"
    );
    Ok(())
}

/// Scenario: thinking_level_dialog.rs stays under 300 lines
#[test]
fn thinking_level_dialog_rs_stays_under_300_lines() -> TestResult {
    // @step Given the file rust/fspec-tui/src/components/thinking_level_dialog.rs after RPC-022 lands
    let path = common::workspace_root()?
        .join("fspec-tui")
        .join("src")
        .join("components")
        .join("thinking_level_dialog.rs");
    // @step When a test counts the line-count of the file
    let lines = common::read_to_string_or_fail(&path)?.lines().count();
    // @step Then the file has fewer than 300 lines
    assert!(
        lines < 300,
        "thinking_level_dialog.rs has {lines} lines (>= 300)"
    );
    Ok(())
}
