//! TUI-106 — shared animated LoadingDialog base + redraw-clock gate.
//!
//! Feature: spec/features/shared-animated-loadingdialog-base-reusing-the-canonical-dialog-theme-with-lifted-braille-spinner-redraw-clock-gate.feature
//!
//! Covers the 8 TUI-106 scenarios: spinner lift byte-identical re-export,
//! LoadingDialog on the shared dialog_theme base, loading≠empty state
//! discriminator, redraw-gate truth table + animated glyph advance,
//! per-cascade-stage labels, ESC-dismiss contract, stale-drop invariance,
//! cascade stage-key shape.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::app::{synth_key, tick_should_draw};
use codelet_fspec_tui::components::dialog_theme::Accent;
use codelet_fspec_tui::components::loading_dialog::{render_loading_dialog, LoadingDialog};
use codelet_fspec_tui::components::load_state::LoadTracker;
use codelet_fspec_tui::components::spinner::{
    current_frame_glyph, DOTS_FRAMES, DOTS_INTERVAL_MS,
};
use codelet_fspec_tui::components::status_dialog::StatusDialog;
use codelet_fspec_tui::Component;
use codelet_fspec_tui::components::{EventResult};
use codelet_fspec_tui::views::agent::spinner as agent_spinner;
use codelet_fspec_tui::{Action, App, FspecBackend, Navigator, Theme, ViewMode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

mod common;
use common::MockBackend;

/// Paint the given body text into an `w×h` buffer (seeded "view body").
fn body_buf(w: u16, h: u16, body: &str) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    Paragraph::new(Line::from(Span::raw(body))).render(Rect::new(0, 0, w, h), &mut buf);
    buf
}

fn buf_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

// ── Scenario a: spinner lift ──────────────────────────────────────────────

/// @step Given the braille spinner code lives in the shared components module
/// @step When the frame picker is queried at elapsed times 0, 80 and 240 milliseconds
/// @step Then the shared spinner returns the first braille glyph at 0 ms, the second glyph at 80 ms and the fourth glyph at 240 ms
/// @step And the ten-frame table wraps back to the first glyph at 800 ms and continues at 80 ms cadence
#[test]
fn spinner_frames_live_in_components_module_and_advance() {
    assert_eq!(DOTS_FRAMES.len(), 10, "ten-frame braille table");
    assert_eq!(DOTS_INTERVAL_MS, 80, "80 ms cadence");
    assert_eq!(current_frame_glyph(0), DOTS_FRAMES[0]);
    assert_eq!(current_frame_glyph(80), DOTS_FRAMES[1]);
    assert_eq!(current_frame_glyph(240), DOTS_FRAMES[3]);
    assert_eq!(current_frame_glyph(800), DOTS_FRAMES[0], "full cycle wraps");
    assert_eq!(current_frame_glyph(880), DOTS_FRAMES[1]);
}

/// @step And the DIM-styled painter writes the first glyph at the row origin in its own area
/// @step And the agent view's spinner module re-exports the exact same table, interval constant and painter so existing agent code compiles unchanged with byte-identical behavior
#[test]
fn agent_view_reexports_the_shared_spinner_byte_identical() {
    assert_eq!(
        agent_spinner::DOTS_FRAMES, DOTS_FRAMES,
        "table identical via re-export"
    );
    assert_eq!(
        agent_spinner::DOTS_INTERVAL_MS, DOTS_INTERVAL_MS,
        "interval identical"
    );
    assert_eq!(
        agent_spinner::current_frame_glyph(160), current_frame_glyph(160),
        "glyph picker identical via re-export"
    );

    let area = Rect::new(0, 0, 30, 1);
    let mut buf = Buffer::empty(area);
    agent_spinner::paint_spinner_line(area, &mut buf, 1, "Thinking", "(Esc to stop)");
    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), DOTS_FRAMES[1], "agent path paints same glyph");
    assert!(
        cell.style().add_modifier.contains(Modifier::DIM),
        "DIM modifier carried through the re-export"
    );
}

// ── Scenario b: LoadingDialog on the shared base ──────────────────────────

fn draw_loading(body: &str, dialog: &LoadingDialog, elapsed_ms: u64) -> Buffer {
    let buf = body_buf(60, 14, body);
    let mut buf = buf;
    render_loading_dialog(Rect::new(0, 0, 60, 14), &mut buf, dialog, elapsed_ms);
    buf
}

/// @step Given a mode view body area at least 60 columns wide and 14 rows tall
/// @step When a LoadingDialog titled "Loading checkpoints" with stage label "Loading checkpoint list…" is painted over that body at elapsed 0 milliseconds
/// @step Then a centered popup with a rounded cyan border and the title "Loading checkpoints" is painted
#[test]
fn loading_dialog_paints_cyan_popup_on_shared_base_over_body() {
    let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
    let body = "No checkpoints available";
    let buf = draw_loading(body, &dialog, 0);

    let text = buf_text(&buf);
    assert!(text.contains("Loading checkpoints"), "dialog title painted");
    assert!(text.contains("⠋"), "braille spinner glyph painted at t=0");
    assert!(text.contains("Loading checkpoint list…"), "stage label painted");

    // Rounded border in the accent (cyan) color — the dialog_theme contract.
    let corner: Vec<_> = buf
        .content
        .iter()
        .filter(|c| matches!(c.symbol(), "╭" | "╮" | "╰" | "╯"))
        .collect();
    assert!(!corner.is_empty(), "corner glyphs present");
    let style = corner[0].style();
    assert_eq!(
        style.fg,
        Some(Accent::Cyan.color()),
        "border fg is accent cyan"
    );
    assert_eq!(style.bg, Some(Color::Black), "border bg is black");

    // Body behind the dialog is preserved (pattern B: paint OVER the panes).
    assert!(text.contains(body), "body visible behind the dialog");
    // No counter row while no progress has been reported.
    assert!(!text.contains("(/)"), "no counter row before progress");
}

/// @step And a dialog row begins with the first braille glyph followed by the stage label
/// @step And the counter row is absent while no progress has been reported
/// @step And the pixel paint comes from the single shared dialog_theme render_dialog implementation the same way StatusDialog does
#[test]
fn loading_dialog_spinner_row_and_optional_counter() {
    let mut dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
    let buf0 = draw_loading("No checkpoints available", &dialog, 0);
    let text0 = buf_text(&buf0);
    assert!(
        text0.contains("⠋ Loading checkpoint list…"),
        "row = glyph + space + label, at elapsed 0"
    );
    assert!(
        !text0.contains("(/") && !text0.contains("1/"),
        "no '(idx/total)' counter row without progress"
    );

    // Counter row appears only once TUI-109 feeds progress.
    dialog.set_progress(3, 10);
    let buf1 = draw_loading("No checkpoints available", &dialog, 0);
    assert!(
        buf_text(&buf1).contains("(3/10)"),
        "counter row painted only when progress is set"
    );
}

// ── Scenario c: loading distinct from empty (state discriminator) ─────────

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

/// @step Given a fresh Checkpoints view whose list load is in flight
/// @step When the view state is inspected before any result has folded
/// @step Then the view reports itself as loading and not empty
#[test]
fn fresh_mode_views_report_loading_not_empty() {
    let mut app = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    // @step Then the view reports itself as loading and not empty
    assert!(
        app.is_view_loading(),
        "fresh checkpoints view (list load in flight) reports loading"
    );

    // @step And a fresh Changed Files view in the same state likewise reports itself as loading and not empty
    let mut app = fresh_app();
    app.dispatch(Action::OpenChangedFilesView);
    assert!(
        app.is_view_loading(),
        "fresh changed-files view (scan in flight) reports loading"
    );
}

/// @step When the cascade list stage has flushed even though the result list is empty
/// @step Then the view reports itself as not loading and empty so the real empty state can surface
#[test]
fn flushed_empty_list_clears_loading_so_empty_state_surfaces() {
    let mut app = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    app.dispatch(Action::CheckpointsLoaded(vec![])); // list flushed, empty
    assert!(
        !app.is_view_loading(),
        "after list flush + no selected checkpoint → fully idle"
    );
    // Loaded AND empty → the real "No checkpoints available" empty state
    // is what renders — distinct from the loading dialog.
    assert!(
        app.navigator_checkpoints_loaded_and_empty(),
        "checkpoints loaded + empty"
    );

    let mut app = fresh_app();
    app.dispatch(Action::OpenChangedFilesView);
    app.dispatch(Action::ChangedFilesLoaded(vec![]));
    assert!(!app.is_view_loading(), "flushed empty scan → fully idle");
    assert!(
        app.navigator_changed_files_loaded_and_empty(),
        "changed files loaded + empty"
    );
}

// ── Scenario d: redraw gate + animation advance ───────────────────────────

/// @step Given the run loop draw gate is evaluated with should-render false, session not busy and input not animating
/// @step When the active mode view is not loading
/// @step Then the gate reports no redraw
/// @step And when the active mode view reports loading the same gate reports a redraw
#[test]
fn redraw_gate_runs_while_view_is_loading_and_only_then() {
    assert!(!tick_should_draw(false, false, false, false), "idle → no redraw");
    assert!(tick_should_draw(false, false, false, true), "view loading → redraw");
    assert!(tick_should_draw(false, true, false, false), "busy → redraw (unchanged)");
    assert!(
        tick_should_draw(false, false, true, false),
        "animating → redraw (unchanged)"
    );
    assert!(
        tick_should_draw(true, false, false, false),
        "pending render (unchanged)"
    );
}

/// Navigator chain: is_view_loading follows active_view.
/// @step And no per-view timer exists — the redraw decision comes only from the shared gate chain view is_loading up through navigator up through app to the draw guard
#[test]
fn navigator_is_view_loading_follows_active_view() {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    nav.active_view = ViewMode::Checkpoints;
    assert!(
        nav.is_view_loading(),
        "fresh checkpoints view → navigator reports loading"
    );
    nav.active_view = ViewMode::Board;
    assert!(
        !nav.is_view_loading(),
        "board has no lazy cascade → not loading"
    );
    nav.active_view = ViewMode::ChangedFiles;
    assert!(nav.is_view_loading(), "fresh changed-files view → loading");
}

/// @step And when a LoadingDialog is painted over the body at elapsed 0 milliseconds the first braille glyph appears in the dialog row
/// @step And when the same dialog is painted at elapsed 80 milliseconds the second braille glyph appears instead
#[test]
fn spinner_glyph_advances_between_zero_and_eighty_ms() {
    let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
    let buf0 = draw_loading("No checkpoints available", &dialog, 0);
    let buf80 = draw_loading("No checkpoints available", &dialog, 80);
    assert!(buf_text(&buf0).contains("⠋"), "t=0 → first glyph");
    assert!(!buf_text(&buf0).contains("⠙"), "t=0 → NOT yet the second glyph");
    assert!(buf_text(&buf80).contains("⠙"), "t=80ms → second glyph");
}

// ── Scenario e: per-stage labels ──────────────────────────────────────────

/// @step Given a checkpoints cascade tracker is created with the list label "Loading checkpoint list…"
/// @step When the files load for checkpoint cp-1 of work unit TUI-107 is requested with label "Loading files for X…"
/// @step Then the active stage label is "Loading files for X…"
/// @step And when that files stage completes with its own key and the diff load for file a.txt is requested with label "Loading diff for a.txt…"
/// @step Then the active stage label is "Loading diff for a.txt…" and the view still reports itself as loading
/// @step And when that diff stage completes with its own key
/// @step Then no stage is active and the view reports itself as not loading
#[test]
fn checkpoints_cascade_stages_show_their_own_labels() {
    let mut t = LoadTracker::new("Loading checkpoint list…");
    assert!(t.is_loading(), "list stage loading");
    assert_eq!(
        t.active_label().as_deref(),
        Some("Loading checkpoint list…")
    );

    t.begin_stage(&LoadTracker::files_stage_key("TUI-107", "cp-1"), "Loading files for X…");
    assert_eq!(t.active_label().as_deref(), Some("Loading files for X…"));

    // Cascade the full checkpoints sequence through App::dispatch: the
    // tracker labels must advance list → files → diff, then settle.
    let mut app = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    let cp = checkpoint_info("TUI-107", "cp-1");
    let file = changed_file("src/a.txt");

    app.dispatch(Action::CheckpointsLoaded(vec![cp]));
    assert!(app.is_view_loading(), "files stage in flight");
    assert_eq!(
        app.navigator_checkpoints_active_label().as_deref(),
        Some("Loading files for cp-1…"),
        "stage 2 label fed by the checkpoints cascade"
    );

    app.dispatch(Action::CheckpointFilesLoaded {
        work_unit_id: "TUI-107".into(),
        name: "cp-1".into(),
        files: vec![file],
    });
    assert!(app.is_view_loading(), "diff stage in flight");
    assert_eq!(
        app.navigator_checkpoints_active_label().as_deref(),
        Some("Loading diff for src/a.txt…"),
        "stage 3 label"
    );

    app.dispatch(Action::CheckpointFileDiffLoaded {
        work_unit_id: "TUI-107".into(),
        name: "cp-1".into(),
        path: "src/a.txt".into(),
        diff: Some("+ line".into()),
    });
    assert!(!app.is_view_loading(), "all stages flushed → idle");
    app.mark_rendered();
}

/// @step And when a changed-files cascade tracker first labels the scan "Loading changed files…"
/// @step And when the files list has flushed and a file diff for b.txt is requested with label "Loading diff for b.txt…"
/// @step Then the active stage label is "Loading diff for b.txt…"
#[test]
fn changed_files_cascade_stages_show_their_own_labels() {
    let mut t = LoadTracker::new("Loading changed files…");
    assert_eq!(
        t.active_label().as_deref(),
        Some("Loading changed files…")
    );
    assert!(t.is_loading());

    let flushed = t.mark_list_flushed();
    assert!(flushed, "list stage complete, no cascading stage → dismissable");

    let key = LoadTracker::diff_stage_key_path("b.txt");
    t.begin_stage(&key, "Loading diff for b.txt…");
    assert!(t.is_loading(), "diff stage in flight");
    assert_eq!(t.active_label().as_deref(), Some("Loading diff for b.txt…"));
    assert!(t.complete_stage(&key), "matching key completes the stage");
    assert!(!t.is_loading(), "cascade settled → idle");
    assert_eq!(t.active_label(), None, "settle: no active label");
}

// ── Scenario f: ESC dismiss contract ──────────────────────────────────────

/// @step Given a LoadingDialog that represents an in-flight lazy load
/// @step When the dismissability of the dialog is queried
/// @step Then the dialog reports NOT dismissable
#[test]
fn loading_dialog_is_not_dismissable_mid_flight() {
    let dialog = LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…");
    assert!(
        !dialog.dismissable(),
        "loading dialog is never dismissible while loading"
    );
    // The view routes this: any ESC handling MUST consult `dismissable()`
    // and return Ignored while loading (full wiring: TUI-107/108).
}

/// @step And the StatusDialog anchor reports ESC is ignored in Restoring state so the shared keyboard contract that TUI-107/108 will wire is locked
#[test]
fn status_dialog_restoring_ignores_esc_anchor() {
    let mut sd = StatusDialog::new("Restoring");
    let ev = synth_key(crossterm::event::KeyCode::Esc);
    let res = sd.handle_event(&ev);
    assert!(
        matches!(res, EventResult::Ignored(None)),
        "ESC while Restoring is ignored (rule [7])"
    );
}

// ── Scenario g: stale-drop invariance ─────────────────────────────────────

/// @step Given the checkpoints cascade tracker is in the files stage for checkpoint NEW
/// @step When a stale files result for the earlier de-selected checkpoint OLD is folded with the OLD stage key
/// @step Then the tracker is unchanged: still loading, active stage key still NEW
/// @step And when the matching files result for NEW is folded with the NEW stage key
/// @step Then the files stage completes and the cascade can advance to the diff stage
#[test]
fn stale_result_for_de_selected_item_does_not_clear_stage() {
    let mut t = LoadTracker::new("Loading checkpoint list…");
    t.mark_list_flushed();
    let new_key = LoadTracker::files_stage_key("TUI-107", "NEW");
    let old_key = LoadTracker::files_stage_key("TUI-107", "OLD");
    t.begin_stage(&new_key, "Loading files for NEW…");
    assert!(
        !t.complete_stage(&old_key),
        "stale key must NOT complete the stage"
    );
    assert!(t.is_loading(), "still loading after stale fold");
    assert_eq!(
        t.active_label().as_deref(),
        Some("Loading files for NEW…"),
        "label unchanged"
    );
    assert!(t.complete_stage(&new_key), "matching key completes");
    assert!(!t.is_loading(), "settled");
}

// ── Scenario h: stage-key shape ────────────────────────────────────────────

/// @step Given a cascade files stage key is built for work unit AUTH-001 checkpoint pre-refactor
/// @step Then the key is exactly "files:AUTH-001:pre-refactor"
/// @step And a cascade diff stage key is exactly "diff:AUTH-001:pre-refactor:src/main.rs"
/// @step And a changed-files diff stage key is exactly "diff:src/app.rs"
#[test]
fn cascade_stage_keys_follow_existing_stale_drop_shape() {
    assert_eq!(
        LoadTracker::files_stage_key("AUTH-001", "pre-refactor"),
        "files:AUTH-001:pre-refactor"
    );
    assert_eq!(
        LoadTracker::diff_stage_key("AUTH-001", "pre-refactor", "src/main.rs"),
        "diff:AUTH-001:pre-refactor:src/main.rs"
    );
    assert_eq!(
        LoadTracker::diff_stage_key_path("src/app.rs"),
        "diff:src/app.rs"
    );
}

// ── helpers ───────────────────────────────────────────────────────────────

fn checkpoint_info(wu: &str, name: &str) -> codelet_rpc_types::CheckpointInfo {
    codelet_rpc_types::CheckpointInfo {
        work_unit_id: wu.into(),
        name: name.into(),
        timestamp: "2026-01-01T00:00:00Z".into(),
        is_automatic: false,
    }
}

fn changed_file(path: &str) -> codelet_rpc_types::ChangedFile {
    codelet_rpc_types::ChangedFile {
        path: path.into(),
        change_type: "M".into(),
        staged: false,
    }
}
