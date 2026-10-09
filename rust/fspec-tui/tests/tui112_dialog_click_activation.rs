//! TUI-112 — left-click activates buttons/rows in all Enter-selectable
//! modal dialogs (R1 three-button dialogs, R2 shared-button-row dialogs,
//! R3 row-list dialogs, R4/R5 hit-testing semantics).
//!
//! Feature: spec/features/left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc.feature
//!          spec/features/exit-session-dialog-click-activation.feature
//!
//! Each scenario in the feature file maps 1:1 to a test below; every
//! Gherkin step is annotated with its `// @step` comment (ACDD).
//!
//! RED note (R2/R3): `MergeConfirmDialog`, `SessionWorktreesDialog`,
//! `ConfirmDialog`, `ThinkingLevelDialog`, `MuxConfigDialog`,
//! `RoleDialog`, `AttachmentPickerDialog` and `WorkUnitSearchDialog`
//! do not implement left-click commit routing yet — those tests are
//! RED until the TUI-112 R2/R3 implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::board_exit_confirmation_dialog::{
    BoardExitChoice, BoardExitConfirmationDialog,
};
use codelet_fspec_tui::components::dialog_button_hits::LastLayout;
use codelet_fspec_tui::components::exit_confirmation_dialog::{ExitChoice, ExitConfirmationDialog};
use codelet_fspec_tui::components::mux_config_dialog::MuxConfigDialog;
use codelet_fspec_tui::views::agent::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
use codelet_fspec_tui::views::agent::merge_confirm_dialog::{
    MergeConfirmDialog, MergeConfirmDialogOutcome,
};
use codelet_fspec_tui::views::agent::session_worktrees_dialog::{
    SessionWorktreesDialog, SessionWorktreesDialogOutcome,
};
use codelet_fspec_tui::{
    Action, AttachmentPickerDialog, Component, CreateSessionDialog, CreateSessionOption,
    RoleDialog, ThinkingLevelDialog, WorkUnitSearchDialog,
};
use codelet_rpc_types::{
    SessionChangesSummary, SessionId, SessionWorktreeInfo, ThinkingLevel, WorkUnitInfo,
};
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use codelet_fspec_tui::views::multiplex::{MuxConfig, MuxOrientation, MuxPaneKind};

// ─────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────

/// Render `dialog` into an 80x24 TestBackend (80x24 frame) and return the
/// buffer. All hit-testing in these tests derives from the LAST rendered
/// frame (R4).
fn render_80x24<D: codelet_fspec_tui::Component>(dialog: &mut D) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    term.draw(|frame| dialog.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

/// Render a `&ConfirmDialog` (NOT a `Component` — embedded behind `&` in
/// the view scaffolds) into an 80x24 TestBackend.
fn render_80x24_ref(dialog: &codelet_fspec_tui::views::agent::confirm_dialog::ConfirmDialog) {
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    term.draw(|frame| dialog.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
}

fn down_at(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn key(code: KeyCode) -> Event {
    Event::Key(crossterm::event::KeyEvent::new(code, KeyModifiers::NONE))
}

/// The center column of a control rect (buttons are ≥3 wide).
fn col_of(layout: &LastLayout, idx: usize) -> u16 {
    let r = layout.controls[idx];
    r.x + r.width / 2
}

/// The middle column of the gap BETWEEN two adjacent control rects
/// (R5: the gap is never part of either button's ` <label> ` cells).
fn gap_col(layout: &LastLayout, left: usize, right: usize) -> u16 {
    let a = &layout.controls[left];
    let b = &layout.controls[right];
    assert!(
        a.x + a.width < b.x,
        "controls must be ordered left-to-right"
    );
    assert!(
        b.x.saturating_sub(a.x + a.width) >= 2,
        "gap must be ≥ 2 columns"
    );
    a.x + a.width + 1
}

// ─────────────────────────────────────────────────────────────────────
// R1 — CreateSessionDialog (Start New Agent?)
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Start New Agent — clicking Yes - Isolated commits it
#[test]
fn start_new_agent_clicking_yes_isolated_commits_it() {
    // @step Given the "Start New Agent?" dialog is open with "Yes" highlighted
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Yes - Isolated" button
    let ev = down_at(col_of(layout, 1), layout.controls[1].y);
    let result = dialog.handle_event(&ev);

    // @step Then a non-isolated session creation is NOT emitted
    let action = dialog.take_pending_action();
    assert!(
        !matches!(
            action,
            Some(Action::CreateSessionSubmitted { isolated: false })
        ),
        "the isolated button must not emit the non-isolated action"
    );
    // @step And an isolated session creation is emitted
    assert!(
        matches!(
            action,
            Some(Action::CreateSessionSubmitted { isolated: true })
        ),
        "clicking 'Yes - Isolated' must emit the isolated creation action"
    );
    // @step And the dialog is removed
    assert!(result.is_consumed(), "a committing click is consumed");
}

/// Scenario: Start New Agent — clicking Cancel dismisses without creating
#[test]
fn start_new_agent_clicking_cancel_dismisses_without_creating() {
    // @step Given the "Start New Agent?" dialog is open
    let mut dialog = CreateSessionDialog::new(None, None);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Cancel" button
    let ev = down_at(col_of(layout, 2), layout.controls[2].y);
    let result = dialog.handle_event(&ev);

    // @step Then no session is created
    assert!(
        !matches!(
            dialog.take_pending_action(),
            Some(Action::CreateSessionSubmitted { .. })
        ),
        "clicking 'Cancel' must not emit any creation action"
    );
    // @step And the dialog is removed
    assert!(result.is_consumed(), "the dismiss click must be consumed");
}

/// Scenario: Start New Agent — keyboard behavior is unchanged
#[test]
fn start_new_agent_keyboard_behavior_is_unchanged() {
    // @step Given the "Start New Agent?" dialog is open with "Yes" highlighted
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    render_80x24(&mut dialog);

    // @step When the user presses Right then Enter
    let r1 = dialog.handle_event(&key(KeyCode::Right));
    assert!(r1.is_consumed());
    let r2 = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then an isolated session creation is emitted
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::CreateSessionSubmitted { isolated: true })
        ),
        "Right then Enter must commit the Isolated option"
    );
    // @step And the dialog is removed
    assert!(r2.is_consumed(), "the Enter commit must be consumed");
}

// ─────────────────────────────────────────────────────────────────────
// R1 — ExitConfirmationDialog (Exit Session?)
// ─────────────────────────────────────────────────────────────────────

fn exit_choice_of(action: Option<Action>) -> Option<ExitChoice> {
    match action {
        Some(Action::AgentExitChoice { choice }) => Some(choice),
        _ => None,
    }
}

/// Scenario: Exit Session — clicking Close Session commits it
#[test]
fn exit_session_clicking_close_session_commits_it() {
    // @step Given the "Exit Session?" dialog is open with "Close Session" highlighted
    let mut dialog = ExitConfirmationDialog::new(false);
    assert_eq!(dialog.selected_choice(), ExitChoice::CloseSession);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Close Session" button
    let ev = down_at(col_of(layout, 0), layout.controls[0].y);
    let result = dialog.handle_event(&ev);

    // @step Then the exit choice CloseSession is committed
    assert_eq!(
        exit_choice_of(dialog.take_pending_action()),
        Some(ExitChoice::CloseSession),
        "clicking 'Close Session' must commit the CloseSession choice"
    );
    // @step And the dialog is removed
    assert!(result.is_consumed(), "a committing click is consumed");
}

/// Scenario: Exit Session — clicking Cancel dismisses it
#[test]
fn exit_session_clicking_cancel_dismisses_it() {
    // @step Given the "Exit Session?" dialog is open with "Close Session" highlighted
    let mut dialog = ExitConfirmationDialog::new(false);
    assert_eq!(dialog.selected_choice(), ExitChoice::CloseSession);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Cancel" button
    let ev = down_at(col_of(layout, 1), layout.controls[1].y);
    let result = dialog.handle_event(&ev);

    // @step Then the exit choice Cancel is committed
    assert_eq!(
        exit_choice_of(dialog.take_pending_action()),
        Some(ExitChoice::Cancel),
        "clicking 'Cancel' must commit the Cancel choice"
    );
    // @step And the dialog is removed
    assert!(result.is_consumed(), "a dismissing click is consumed");
}

// ─────────────────────────────────────────────────────────────────────
// R1 — BoardExitConfirmationDialog (Exit fspec?)
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Exit fspec — clicking Exit commits quit
#[test]
fn exit_fspec_clicking_exit_commits_quit() {
    // @step Given the "Exit fspec?" dialog is open with "Exit" highlighted
    let mut dialog = BoardExitConfirmationDialog::new();
    assert_eq!(dialog.selected_choice(), BoardExitChoice::Exit);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Exit" button
    let ev = down_at(col_of(layout, 0), layout.controls[0].y);
    let result = dialog.handle_event(&ev);

    // @step Then the app quit action is emitted
    assert!(
        matches!(dialog.take_pending_action(), Some(Action::Quit)),
        "clicking 'Exit' must emit Action::Quit"
    );
    // @step And the dialog is removed
    assert!(result.is_consumed(), "a committing click is consumed");
}

/// Scenario: Exit fspec — clicking Cancel stays on the board
#[test]
fn exit_fspec_clicking_cancel_stays_on_the_board() {
    // @step Given the "Exit fspec?" dialog is open with "Exit" highlighted
    let mut dialog = BoardExitConfirmationDialog::new();
    assert_eq!(dialog.selected_choice(), BoardExitChoice::Exit);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Cancel" button
    let ev = down_at(col_of(layout, 1), layout.controls[1].y);
    let result = dialog.handle_event(&ev);

    // @step Then no quit action is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "clicking 'Cancel' must not emit any action"
    );
    // @step And the dialog is removed
    assert!(
        result.is_consumed(),
        "the dismiss click must be consumed (the dialog removes itself)"
    );
    // @step And the user remains on the board
    // (no Action::Quit → App::dispatch keeps the BoardView; asserted by
    // the absence of the quit action above)
}

// ─────────────────────────────────────────────────────────────────────
// R2 — MergeConfirmDialog
// ─────────────────────────────────────────────────────────────────────

fn merge_summary() -> SessionChangesSummary {
    SessionChangesSummary {
        files_changed: 2,
        insertions: 10,
        deletions: 3,
        commits: vec!["abc1234".to_string()],
        files_ignored: 0,
    }
}

/// Scenario: Merge confirm — clicking Discard returns Discard
#[test]
fn merge_confirm_clicking_discard_returns_discard() {
    // @step Given the merge-confirm dialog is open with "Merge" focused
    let mut dialog = MergeConfirmDialog::new(SessionId::new("s-1"), merge_summary());
    assert_eq!(dialog.focused_button(), 0);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Discard" button
    let ev = down_at(col_of(&layout, 1), layout.controls[1].y);
    let result = dialog.handle_event(&ev);

    // @step Then the outcome is Discard
    assert!(
        matches!(
            dialog.take_pending_outcome(),
            Some(MergeConfirmDialogOutcome::Discard { .. })
        ),
        "clicking 'Discard' must return the Discard outcome"
    );
    // @step And the dialog is dismissed
    assert!(
        result.is_consumed(),
        "the dismissing click must be consumed"
    );
}

/// Scenario: Merge confirm — clicking Cancel returns Cancel
#[test]
fn merge_confirm_clicking_cancel_returns_cancel() {
    // @step Given the merge-confirm dialog is open with "Merge" focused
    let mut dialog = MergeConfirmDialog::new(SessionId::new("s-1"), merge_summary());
    assert_eq!(dialog.focused_button(), 0);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Cancel" button
    let ev = down_at(col_of(&layout, 2), layout.controls[2].y);
    let result = dialog.handle_event(&ev);

    // @step Then the outcome is Cancel
    assert!(
        matches!(
            dialog.take_pending_outcome(),
            Some(MergeConfirmDialogOutcome::Cancel)
        ),
        "clicking 'Cancel' must return the Cancel outcome"
    );
    assert!(
        result.is_consumed(),
        "the dismissing click must be consumed"
    );
}

// ─────────────────────────────────────────────────────────────────────
// R2 — SessionWorktreesDialog
// ─────────────────────────────────────────────────────────────────────

fn worktree_rows() -> Vec<SessionWorktreeInfo> {
    vec![SessionWorktreeInfo {
        session_id: SessionId::new("s-1"),
        worktree_path: "/tmp/wt-s-1".to_string(),
        base_commit: "base".to_string(),
        head_commit: "head".to_string(),
        dirty: true,
    }]
}

/// Scenario: Session worktrees — clicking Prune returns Prune
#[test]
fn session_worktrees_clicking_prune_returns_prune() {
    // @step Given the session-worktrees dialog is open with "Prune" focused
    let mut dialog = SessionWorktreesDialog::new(worktree_rows());
    assert_eq!(dialog.focused_button(), 0);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Prune" button
    let ev = down_at(col_of(layout, 0), layout.controls[0].y);
    let result = dialog.handle_event(&ev);

    // @step Then the outcome is Prune
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::PruneLeakedWorktrees)
        ),
        "clicking 'Prune' must emit the Prune action"
    );
    assert!(
        result.is_consumed(),
        "the dismissing click must be consumed"
    );
}

/// Scenario: Session worktrees — clicking Cancel returns Cancel
#[test]
fn session_worktrees_clicking_cancel_returns_cancel() {
    // @step Given the session-worktrees dialog is open with "Prune" focused
    let mut dialog = SessionWorktreesDialog::new(worktree_rows());
    assert_eq!(dialog.focused_button(), 0);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Cancel" button
    let ev = down_at(col_of(layout, 1), layout.controls[1].y);
    let result = dialog.handle_event(&ev);

    // @step Then the outcome is Cancel
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::CancelWorktreesDialog)
        ),
        "clicking 'Cancel' must emit the Cancel action"
    );
    assert!(
        result.is_consumed(),
        "the dismissing click must be consumed"
    );
}

// ─────────────────────────────────────────────────────────────────────
// R2 — ConfirmDialog
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Confirm dialog — clicking the primary button returns Primary
#[test]
fn confirm_dialog_clicking_the_primary_button_returns_primary() {
    // @step Given a two-button confirm dialog (Delete / Cancel) is open with "Delete" focused
    let dialog = ConfirmDialog::new(
        "Delete Session?",
        "This action cannot be undone.",
        "Delete",
        None,
        "Cancel",
    );
    assert_eq!(dialog.focused(), 0);
    render_80x24_ref(&dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks the "Cancel" button
    let outcome = dialog.handle_click(col_of(&layout, 1), layout.controls[1].y);

    // @step Then the outcome is Cancel
    assert_eq!(
        outcome,
        ConfirmDialogOutcome::Cancel,
        "clicking 'Cancel' on the two-button dialog must return Cancel"
    );
}

// ─────────────────────────────────────────────────────────────────────
// R3 — ThinkingLevelDialog
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Thinking Level — clicking High commits High
#[test]
fn thinking_level_clicking_high_commits_high() {
    // @step Given the Thinking Level dialog is open with "Off" highlighted
    let mut dialog = ThinkingLevelDialog::new(SessionId::new("s-1"), ThinkingLevel::Off);
    assert_eq!(dialog.selected_level(), ThinkingLevel::Off);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();
    let high_row = layout.controls[3]; // Off, Low, Medium, High

    // @step When the user left-clicks the "High" row
    let ev = down_at(high_row.x + high_row.width / 2, high_row.y);
    let result = dialog.handle_event(&ev);

    // @step Then the thinking level High is committed for the session
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::ThinkingLevelSelected(_, ThinkingLevel::High))
        ),
        "clicking the 'High' row must commit High"
    );
    // @step And the dialog is closed
    assert!(result.is_consumed(), "a committing click is consumed");
}

// ─────────────────────────────────────────────────────────────────────
// R3 — MuxConfigDialog
// ─────────────────────────────────────────────────────────────────────

fn two_pane_config() -> MuxConfig {
    MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50],
        panes: vec![MuxPaneKind::Board, MuxPaneKind::Agent],
        focused_pane: 1,
        enabled: false,
    }
}

fn click_row(dialog: &mut MuxConfigDialog, row: usize) {
    let layout = dialog.last_layout();
    let r = layout.controls[row];
    let result = dialog.handle_event(&down_at(r.x + r.width / 2, r.y));
    assert!(result.is_consumed(), "a row click must be consumed");
}

/// Scenario: Mux Layout — clicking Pane 2 applies with the cursor on Pane 2
#[test]
fn mux_layout_clicking_pane_2_applies_with_the_cursor_on_pane_2() {
    // @step Given the Mux Layout dialog is open with the cursor on "Enabled"
    let mut dialog = MuxConfigDialog::new(two_pane_config());
    assert_eq!(dialog.cursor(), 0);
    render_80x24(&mut dialog);

    // @step When the user left-clicks the "Pane 2" row
    click_row(&mut dialog, 3); // rows: Enabled, Orientation, Pane 1, Pane 2

    // @step Then the mux config is applied
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::MuxConfigApplied(_))
        ),
        "clicking a row must apply the draft config"
    );
    // @step And the cursor is on the "Pane 2" row
    assert_eq!(
        dialog.cursor(),
        3,
        "the cursor must land on the clicked row"
    );
}

/// Scenario: Mux Layout — clicking Orientation applies with the cursor on Orientation
#[test]
fn mux_layout_clicking_orientation_applies_with_the_cursor_on_orientation() {
    // @step Given the Mux Layout dialog is open with the cursor on "Enabled"
    let mut dialog = MuxConfigDialog::new(two_pane_config());
    assert_eq!(dialog.cursor(), 0);
    render_80x24(&mut dialog);

    // @step When the user left-clicks the "Orientation" row
    click_row(&mut dialog, 1);

    // @step Then the mux config is applied
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::MuxConfigApplied(_))
        ),
        "clicking a row must apply the draft config"
    );
    // @step And the cursor is on the "Orientation" row
    assert_eq!(
        dialog.cursor(),
        1,
        "the cursor must land on the clicked row"
    );
}

// ─────────────────────────────────────────────────────────────────────
// R3 — RoleDialog
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Role dialog — clicking the draft row saves the role
#[test]
fn role_dialog_clicking_the_draft_row_saves_the_role() {
    // @step Given the Role dialog is open with a non-empty draft
    let mut dialog = RoleDialog::new(SessionId::new("s-1"), Some("code reviewer".to_string()));
    assert_eq!(dialog.draft(), "code reviewer");
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();
    let row = &layout.controls[0];

    // @step When the user left-clicks the draft row
    let ev = down_at(row.x + row.width / 2, row.y);
    let result = dialog.handle_event(&ev);

    // @step Then the role is saved
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::SetSessionRole(_, Some(role))) if role == "code reviewer"
        ),
        "clicking the draft row must save the draft as the role"
    );
    // @step And the dialog is closed
    assert!(result.is_consumed(), "a committing click is consumed");
}

// ─────────────────────────────────────────────────────────────────────
// R3 — AttachmentPickerDialog
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Attachment picker — clicking notes.pdf opens it
#[test]
fn attachment_picker_clicking_notes_pdf_opens_it() {
    // @step Given the attachment picker is open with "design.md" and "notes.pdf" listed
    let mut dialog = AttachmentPickerDialog::new(vec![
        "spec/attachments/TUI-112/design.md".to_string(),
        "spec/attachments/TUI-112/notes.pdf".to_string(),
    ]);
    assert_eq!(
        dialog.row_labels(),
        vec!["design.md".to_string(), "notes.pdf".to_string()]
    );
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();
    let notes_row = &layout.controls[1];

    // @step When the user left-clicks the "notes.pdf" row
    let ev = down_at(notes_row.x + notes_row.width / 2, notes_row.y);
    let result = dialog.handle_event(&ev);

    // @step Then the attachment "notes.pdf" full path is opened
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::OpenAttachment(path))
                if path == "spec/attachments/TUI-112/notes.pdf"
        ),
        "clicking a row must open that attachment's full path"
    );
    // @step And the dialog is closed
    assert!(result.is_consumed(), "a committing click is consumed");
}

// ─────────────────────────────────────────────────────────────────────
// R3 — WorkUnitSearchDialog
// ─────────────────────────────────────────────────────────────────────

fn work_unit(id: &str, title: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// Scenario: Work unit search — clicking a result row commits it
#[test]
fn work_unit_search_clicking_a_result_row_commits_it() {
    // @step Given the work-unit search dialog is open showing a "MENU-009" result row
    let mut dialog = WorkUnitSearchDialog::new(vec![
        work_unit("MENU-008", "Another button"),
        work_unit("MENU-009", "Menu bar Zone C"),
    ]);
    assert!(dialog.matches().contains(&"MENU-009".to_string()));
    render_80x24(&mut dialog);
    let row = dialog.last_row_rects().get(1).copied();
    let row = row.expect("two matches fit the visible rows after render");

    // @step When the user left-clicks the "MENU-009" row
    let ev = down_at(row.x + row.width / 2, row.y);
    let result = dialog.handle_event(&ev);

    // @step Then the work unit MENU-009 is selected
    assert!(
        matches!(
            dialog.take_pending_action(),
            Some(Action::SelectWorkUnit(ref id)) if id == "MENU-009"
        ),
        "clicking a result row must select that work unit"
    );
    // @step And the dialog is closed
    assert!(result.is_consumed(), "a committing click is consumed");
}

// ─────────────────────────────────────────────────────────────────────
// R4/R5 — hit-testing & non-activatable cells
// ─────────────────────────────────────────────────────────────────────

/// Scenario: A click outside the dialog rect is ignored
#[test]
fn a_click_outside_the_dialog_rect_is_ignored() {
    // @step Given the "Start New Agent?" dialog is open
    let mut dialog = CreateSessionDialog::new(None, None);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();
    let frame = layout
        .dialog_rect
        .expect("the dialog frame rect must be cached after render");

    // @step When the user left-clicks a cell outside the dialog's rendered rect
    // The top border row is the first row of the frame; one row ABOVE it
    // is outside the frame (and off any control).
    let outside_row = frame.y.saturating_sub(1);
    let ev = down_at(frame.x + frame.width / 2, outside_row);
    let result = dialog.handle_event(&ev);

    // @step Then no action is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "a click outside the frame must not emit any action"
    );
    // @step And the dialog stays open
    assert!(
        !result.is_consumed(),
        "a click outside the frame must be Ignored so it bubbles"
    );
}

/// Scenario: A click in the gap between two buttons does nothing
#[test]
fn a_click_in_the_gap_between_two_buttons_does_nothing() {
    // @step Given the "Start New Agent?" dialog is open
    let mut dialog = CreateSessionDialog::new(None, None);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();

    // @step When the user left-clicks a cell in the gap between two buttons
    let col = gap_col(layout, 0, 1);
    let ev = down_at(col, layout.controls[0].y);
    let result = dialog.handle_event(&ev);

    // @step Then no action is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "a click in the gap must not emit any action"
    );
    // @step And the dialog stays open
    assert!(
        !result.is_consumed(),
        "a gap click must be Ignored so it bubbles"
    );
    // @step And no button is committed
    assert_eq!(
        dialog.selected_option(),
        CreateSessionOption::Yes,
        "a gap click must not move the highlight"
    );
}

/// Scenario: A click on the title, description or footer does nothing
#[test]
fn a_click_on_the_title_description_or_footer_does_nothing() {
    // @step Given the "Start New Agent?" dialog is open
    let mut dialog = CreateSessionDialog::new(None, None);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();
    let frame = layout
        .dialog_rect
        .expect("the dialog frame rect must be cached after render");
    let buttons_y = layout.controls[0].y;

    // @step When the user left-clicks a cell on the title row, a description cell, or a footer cell
    // Title row (inside the frame, not a control):
    let title_ev = down_at(frame.x + 4, frame.y + 1);
    assert!(
        !dialog.handle_event(&title_ev).is_consumed(),
        "title click must be Ignored"
    );
    // Description row (between the title and the button row):
    let desc_ev = down_at(frame.x + 4, buttons_y - 1);
    assert!(
        !dialog.handle_event(&desc_ev).is_consumed(),
        "description click must be Ignored"
    );
    // Footer row (below the last control, inside the frame):
    let footer_ev = down_at(frame.x + 4, frame.y + frame.height - 2);
    assert!(
        !dialog.handle_event(&footer_ev).is_consumed(),
        "footer click must be Ignored"
    );

    // @step Then no action is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "non-control clicks must not emit any action"
    );
    // @step And the dialog stays open
    assert_eq!(
        dialog.selected_option(),
        CreateSessionOption::Yes,
        "non-control clicks must not move the highlight"
    );
}

/// Scenario: Wheel navigation behavior is unchanged
#[test]
fn wheel_navigation_behavior_is_unchanged() {
    // @step Given the "Start New Agent?" dialog is open with "Yes" highlighted
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    render_80x24(&mut dialog);
    let layout = dialog.last_layout();
    let inside = down_inside(layout);

    // @step When the user scrolls right then scrolls left
    let scroll_right = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollRight,
        column: inside.column,
        row: inside.row,
        modifiers: KeyModifiers::NONE,
    });
    assert!(dialog.handle_event(&scroll_right).is_consumed());
    assert_eq!(
        dialog.selected_option(),
        CreateSessionOption::Isolated,
        "ScrollRight must move the highlight one step"
    );
    let scroll_left = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollLeft,
        column: inside.column,
        row: inside.row,
        modifiers: KeyModifiers::NONE,
    });
    assert!(dialog.handle_event(&scroll_left).is_consumed());

    // @step Then "Yes" is highlighted again
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    // @step And no action is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "wheel events must never commit"
    );
}

/// A point inside the dialog frame (for wheel events).
fn down_inside(layout: &LastLayout) -> MouseEvent {
    let frame = layout
        .dialog_rect
        .expect("the dialog frame rect must be cached after render");
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: frame.x + frame.width / 2,
        row: layout.controls[0].y,
        modifiers: KeyModifiers::NONE,
    }
}

// Silence the unused-import lint for types used only in the RED R2/R3
// tests before the accessors exist (kept so the test file compiles as a
// whole once the implementation lands).
#[allow(dead_code)]
fn _type_anchor(
    _r: Rect,
    _buf: Buffer,
    _outcome: ConfirmDialogOutcome,
    _outcome2: SessionWorktreesDialogOutcome,
) {
}
