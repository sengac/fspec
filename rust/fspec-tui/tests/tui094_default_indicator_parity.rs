//! TUI-094 — (default) indicator parity tests for ThinkingLevelDialog.
//!
//! Feature: spec/features/thinking-dialog-default-indicator.feature
//!
//! Asserts the rendered buffer marks the persisted-default row with
//! ` (default)` (riding on the dimmable description span), mirroring the
//! TS `ThinkingLevelDialog.tsx` `isDefault` branch (lines 129, 140-144).
//! Uses the new `with_default_level(Option<ThinkingLevel>)` builder.

use codelet_test_helpers::{OrFail, TestResult};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;
use ratatui::Terminal;

use codelet_fspec_tui::components::thinking_level_dialog::ThinkingLevelDialog;
use codelet_fspec_tui::components::Component;
use codelet_rpc_types::{SessionId, ThinkingLevel};

fn render_component_80x24<C: Component>(
    component: &mut C,
) -> Result<Buffer, codelet_test_helpers::TestError> {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).or_fail("Terminal::new(TestBackend)")?;
    terminal
        .draw(|frame| {
            component.render(frame.area(), frame.buffer_mut());
        })
        .or_fail("draw")?;
    Ok(terminal.backend().buffer().clone())
}

fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// First (x, y) where `needle` appears on a single row.
fn find_text(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() {
        return None;
    }
    for y in 0..buf.area.height {
        let row: Vec<char> = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
            .collect();
        for start in 0..row.len() {
            if start + needle_chars.len() > row.len() {
                break;
            }
            if row[start..start + needle_chars.len()] == needle_chars[..] {
                return Some((start as u16, y));
            }
        }
    }
    None
}

/// The full text of the row (y) that contains `label`.
fn row_containing(buf: &Buffer, label: &str) -> Result<String, codelet_test_helpers::TestError> {
    let (_, y) = find_text(buf, label).or_fail(&format!("{label} present"))?;
    let mut row = String::new();
    for x in 0..buf.area.width {
        row.push_str(buf[(x, y)].symbol());
    }
    Ok(row)
}

fn dialog(current: ThinkingLevel, default: Option<ThinkingLevel>) -> ThinkingLevelDialog {
    ThinkingLevelDialog::new(SessionId::new("tui094"), current, four_levels())
        .with_default_level(default)
}

/// Scenario: Default row shows the (default) marker appended to its description
#[test]
fn default_row_shows_default_marker_appended_to_description() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level High
    let mut d = dialog(
        ThinkingLevel::Off,
        Some(ThinkingLevel::Effort("high".to_string())),
    );
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d)?;
    // @step Then the high row reads "high - Named effort (default)"
    let high_row = row_containing(&buf, "high")?;
    assert!(
        high_row.contains("high - Named effort (default)"),
        "High row missing the (default) marker, got {high_row:?}"
    );
    Ok(())
}

/// Scenario: No default persisted shows no (default) marker on any row
#[test]
fn no_default_persisted_shows_no_marker_on_any_row() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level None
    let mut d = dialog(ThinkingLevel::Off, None);
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d)?;
    // @step Then no row in the rendered buffer contains the text "(default)"
    let text = buffer_text(&buf);
    assert!(
        !text.contains("(default)"),
        "no (default) marker expected when default is None, got:\n{text}"
    );
    Ok(())
}

/// Scenario: Default marker is independent of the selection highlight
#[test]
fn default_marker_is_independent_of_selection_highlight() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level Medium
    let mut d = dialog(
        ThinkingLevel::Off,
        Some(ThinkingLevel::Effort("medium".to_string())),
    );
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d)?;
    // @step Then the Off row is highlighted with the "▸" marker and shows no "(default)" text
    let off_row = row_containing(&buf, "Off")?;
    assert!(
        off_row.contains('▸'),
        "Off row must be highlighted, got {off_row:?}"
    );
    assert!(
        !off_row.contains("(default)"),
        "Off row must NOT carry (default), got {off_row:?}"
    );
    // @step Then the Medium row is not highlighted and reads "(default)"
    let medium_row = row_containing(&buf, "medium")?;
    assert!(
        !medium_row.contains('▸'),
        "Medium row must not be highlighted, got {medium_row:?}"
    );
    assert!(
        medium_row.contains("(default)"),
        "Medium row must carry (default), got {medium_row:?}"
    );
    Ok(())
}

/// Scenario: Default equals current selection shows both the highlight and the (default) marker
#[test]
fn default_equals_current_shows_both_highlight_and_marker() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current level High and default level High
    let mut d = dialog(
        ThinkingLevel::Effort("high".to_string()),
        Some(ThinkingLevel::Effort("high".to_string())),
    );
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d)?;
    // @step Then the High row begins with the "▸" selection marker
    let high_row = row_containing(&buf, "high")?;
    assert!(
        high_row.contains('▸'),
        "High row must be highlighted, got {high_row:?}"
    );
    // @step Then the High row also reads "(default)"
    assert!(
        high_row.contains("(default)"),
        "High row must also carry (default), got {high_row:?}"
    );
    Ok(())
}

/// Scenario: Default marker on an unselected row is dimmed on the description span
#[test]
fn default_marker_on_unselected_row_is_dimmed() -> TestResult {
    // @step Given a ThinkingLevelDialog seeded with current level Off and default level High
    let mut d = dialog(
        ThinkingLevel::Off,
        Some(ThinkingLevel::Effort("high".to_string())),
    );
    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_component_80x24(&mut d)?;
    // @step Then the "(default)" cells on the unselected High row carry the Modifier::DIM style
    let (dx, dy) = find_text(&buf, "(default)").or_fail("(default) present in buffer")?;
    for i in 0..("(default)".chars().count() as u16) {
        let cell = &buf[(dx + i, dy)];
        assert!(
            cell.modifier.contains(Modifier::DIM),
            "(default) cell at +{i} must be DIM (rides on the dimmed description span)"
        );
    }
    Ok(())
}

/// MODEL-011: a model offering Off and the efforts low, medium and high.
fn four_levels() -> codelet_rpc_types::ThinkingOptions {
    codelet_rpc_types::ThinkingOptions::Levels {
        model: "test/model".to_string(),
        choices: vec![
            codelet_rpc_types::ThinkingChoice::Off,
            codelet_rpc_types::ThinkingChoice::Effort("low".to_string()),
            codelet_rpc_types::ThinkingChoice::Effort("medium".to_string()),
            codelet_rpc_types::ThinkingChoice::Effort("high".to_string()),
        ],
        takes_off: true,
    }
}
