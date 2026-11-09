//! RPC-015 — BoardView header strip render tests.
//!
//! Feature: spec/features/rpc015-board-header.feature
//!
//! Drives `BoardView::render_with_store` against a `TestBackend` after
//! RPC-015's 4-row header strip + 1-row separator inserts itself between
//! the top border and the existing 5-row details strip.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, AgentViewStore, BoardStore, BoardView, Theme};
use codelet_rpc_types::{CheckpointCounts, WorkUnitInfo};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn make_unit(id: &str, status: &str, work_type: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: work_type.to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn render(
    width: u16,
    height: u16,
    store: &BoardStore,
) -> Result<Buffer, codelet_test_helpers::TestError> {
    let (view, _rx) = fresh();
    let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        view.render_with_store(
            frame.area(),
            frame.buffer_mut(),
            store,
            &AgentViewStore::default(),
            false,
        );
    })
    .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn join_buffer(buf: &Buffer) -> String {
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

fn row_string(buf: &Buffer, y: u16) -> String {
    let mut row = String::with_capacity(buf.area.width as usize);
    for x in 0..buf.area.width {
        row.push_str(buf[(x, y)].symbol());
    }
    row
}

/// Scenario: Empty BoardStore paints the FSPEC logo and Checkpoints: None
#[test]
fn empty_board_store_paints_the_fspec_logo_and_checkpoints_none() -> TestResult {
    // @step Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store)?;
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "┏┓┏┓┏┓┏┓┏┓"
    assert!(
        joined.contains("┏┓┏┓┏┓┏┓┏┓"),
        "missing FSPEC logo glyph row:\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Checkpoints: None"
    assert!(
        joined.contains("Checkpoints: None"),
        "missing 'Checkpoints: None':\n{joined}"
    );
    Ok(())
}

/// Scenario: Non-zero checkpoint counts paint the Manual/Auto breakdown
#[test]
fn non_zero_checkpoint_counts_paint_the_manual_auto_breakdown() -> TestResult {
    // @step Given a BoardStore whose checkpoint_counts has been set to { manual: 2, auto: 5 }
    let mut store = BoardStore::default();
    store.set_checkpoint_counts(CheckpointCounts { manual: 2, auto: 5 });
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store)?;
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "Checkpoints: 2 Manual, 5 Auto"
    assert!(
        joined.contains("Checkpoints: 2 Manual, 5 Auto"),
        "missing 'Checkpoints: 2 Manual, 5 Auto':\n{joined}"
    );
    Ok(())
}

/// Scenario: The 2-zone menu bar row is painted in the header
/// (BOARD-023 R10 supersession: the six-action chord was replaced by the
/// short 'u Actions' hint; MENU-002 R1 supersedes that hint with the live
/// 2-zone menu bar — Zone A "Kanban"/"Tools"/"Settings"/"Help" items after
/// MENU-008, no session chips on a fresh store)
#[test]
fn the_two_zone_menu_bar_row_is_painted_in_the_header() -> TestResult {
    // @step Given a BoardStore with any selection state
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store)?;
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the Zone A items "Kanban", "Tools", "Settings" and "Help" (MENU-008 categories)
    for item in ["Kanban", "Tools", "Settings", "Help"] {
        assert!(
            joined.contains(item),
            "missing the 2-zone bar's Zone A item {item:?}: {joined}"
        );
    }
    // @step And the rendered buffer does NOT contain the substring "u Actions"
    assert!(
        !joined.contains("u Actions"),
        "the 'u Actions' hint must be gone, replaced by the menu bar: {joined}"
    );
    // @step And the rendered buffer does NOT contain the substring "C Checkpoints"
    assert!(
        !joined.contains("C Checkpoints"),
        "the old chord must no longer render in the header: {joined}"
    );
    Ok(())
}

/// Scenario: New ├──┤ separator sits between the header strip and the details strip
#[test]
fn new_plain_separator_sits_between_the_header_strip_and_the_details_strip() -> TestResult {
    // @step Given a BoardStore containing AUTH-001 in backlog
    let mut store = BoardStore::default();
    store.replace_work_units(vec![make_unit("AUTH-001", "backlog", "story")]);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store)?;
    // Header layout after RPC-015:
    //   row 0       = ┌───────┐
    //   rows 1..=4  = 4-row header strip
    //   row 5       = ├───────┤  (plain — between header and details strip)
    //   rows 6..=10 = 5-row details strip
    //   row 11      = ├┬────┬┬┤  (top junctions)
    //   row 12      = column header
    //   row 13      = ├┼────┼┼┤  (cross junctions)
    //   ... content rows ...
    //   row N-2     = ├┴────┴┴┤  (bottom junctions)
    //   row N-1     = footer string
    //   row N       = └───────┘
    let row5 = row_string(&buf, 5);
    // @step Then one of the inner rows contains the glyph "├" and the glyph "┤" with NO inner "┬" or "┼" or "┴" junctions on that same row
    assert!(row5.contains('├'), "row 5 must contain ├, got `{row5}`");
    assert!(row5.contains('┤'), "row 5 must contain ┤, got `{row5}`");
    assert!(
        !row5.contains('┬'),
        "row 5 (header→details separator) must NOT contain ┬, got `{row5}`"
    );
    assert!(
        !row5.contains('┼'),
        "row 5 (header→details separator) must NOT contain ┼, got `{row5}`"
    );
    assert!(
        !row5.contains('┴'),
        "row 5 (header→details separator) must NOT contain ┴, got `{row5}`"
    );
    // @step And the four existing details/columns/footer separator rows (├┬┤ / ├┼┤ / ├┴┤) are still painted exactly as before
    let has_top = (0..buf.area.height).any(|y| {
        let r = row_string(&buf, y);
        r.contains('├') && r.contains('┬') && r.contains('┤')
    });
    assert!(has_top, "expected one row with ├ ┬ ┤ junctions");
    let has_cross = (0..buf.area.height).any(|y| {
        let r = row_string(&buf, y);
        r.contains('├') && r.contains('┼') && r.contains('┤')
    });
    assert!(has_cross, "expected one row with ├ ┼ ┤ junctions");
    let has_bottom = (0..buf.area.height).any(|y| {
        let r = row_string(&buf, y);
        r.contains('├') && r.contains('┴') && r.contains('┤')
    });
    assert!(has_bottom, "expected one row with ├ ┴ ┤ junctions");
    Ok(())
}

/// Scenario: KeybindingShortcuts are visible hints only — no Action wiring lands in this card
#[test]
fn keybinding_shortcuts_are_visible_hints_only_no_action_wiring_lands_in_this_card() -> TestResult {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    // @step Given the App has rendered BoardView with the new header strip painted
    let (view, mut rx) = fresh();
    let store = BoardStore::default();
    // Drive a single render to "paint" the header — used only to mirror
    // the Gherkin Given clause; the assertion is on the next key press.
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        view.render_with_store(
            frame.area(),
            frame.buffer_mut(),
            &store,
            &AgentViewStore::default(),
            false,
        )
    })
    .or_fail("draw")?;
    // @step When the user presses the key 'C'
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::empty())),
        &store,
    );
    // @step Then NO new Action variant is emitted that opens a checkpoint viewer
    // @step And NO new Action variant is emitted that opens the FOUNDATION.md viewer
    // @step And NO new Action variant is emitted that opens the changed-files viewer
    // NOTE (RPC-364): the board `C` key now legitimately opens the
    // three-pane Checkpoints view (Action::OpenCheckpointsView), which is
    // whitelisted in the match below. The RPC-015 card established C as a
    // visible hint only; RPC-364 delivers its action wiring.
    let mut actions: Vec<Action> = Vec::new();
    while let Ok(a) = rx.try_recv() {
        actions.push(a);
    }
    for a in &actions {
        // Only the established RPC-012/013/014 variants are expected.
        // No new opener variants should appear in this card.
        match a {
            Action::SelectNext
            | Action::SelectPrev
            | Action::ReorderUp
            | Action::ReorderDown
            | Action::EnterWorkUnit(_)
            | Action::OpenAgentView(_)
            | Action::BackToBoard => {}
            // RPC-364: the board `C`/`c` key now opens the three-pane
            // Checkpoints view. This supersedes the RPC-015-era assertion
            // that C emitted no opener — the checkpoints opener is the
            // intended wiring delivered by RPC-364.
            Action::OpenCheckpointsView => {}
            other => {
                return Err(format!(
                    "C-press must not emit a new opener-action — observed {other:?} in {actions:?}"
                )
                .into())
            }
        }
    }
    // @step And BoardView continues to emit existing Action variants on existing key events (← / → / ↑ / ↓ / Enter / [ / ] / Shift+Right / ESC)
    // MENU-002 R2 supersedes: Right now walks the continuous
    // column⇄menu⇄chip ring via Action::MenuMove(1) instead of
    // Action::FocusNextColumn.
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty())),
        &store,
    );
    let mut had_menu_move = false;
    while let Ok(a) = rx.try_recv() {
        if matches!(a, Action::MenuMove(1)) {
            had_menu_move = true;
        }
    }
    assert!(
        had_menu_move,
        "Right arrow must emit Action::MenuMove(1) after MENU-002 (supersedes FocusNextColumn)"
    );
    Ok(())
}

/// Scenario: RPC-014 details strip and RPC-013 footer are still painted after RPC-015 inserts its header
#[test]
fn rpc014_details_strip_and_rpc013_footer_are_still_painted_after_rpc015_header() -> TestResult {
    // @step Given a BoardStore containing AUTH-001 (story, backlog, title "User Login", description "Sign in with email/password", estimate 5, epic "authentication", no attachments)
    let unit = WorkUnitInfo {
        id: "AUTH-001".to_string(),
        title: "User Login".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: Some("Sign in with email/password".to_string()),
        estimate: Some(5),
        epic: Some("authentication".to_string()),
        attachments: Vec::new(),
        last_state_change_at: None,
    };
    let mut store = BoardStore::default();
    store.replace_work_units(vec![unit]);
    // @step And the focused column is "backlog" and the selected index is 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store)?;
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "AUTH-001: User Login"
    assert!(
        joined.contains("AUTH-001: User Login"),
        "missing details title:\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Epic: authentication"
    assert!(
        joined.contains("Epic: authentication"),
        "missing Epic line:\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Estimate: 5pts"
    assert!(
        joined.contains("Estimate: 5pts"),
        "missing Estimate line:\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Status: backlog"
    assert!(
        joined.contains("Status: backlog"),
        "missing Status line:\n{joined}"
    );
    // @step And the rendered buffer contains the substring "← →"
    assert!(joined.contains("← →"), "missing footer arrows:\n{joined}");
    // @step And the rendered buffer contains the substring "Choose"
    assert!(
        joined.contains("Choose"),
        "missing footer 'Choose':\n{joined}"
    );
    Ok(())
}
