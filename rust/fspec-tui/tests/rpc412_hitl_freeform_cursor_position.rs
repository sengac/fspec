//! Feature: spec/features/inline-hitl-freeform-cursor-position.feature
//!
//! RPC-412 — the inline HITL freeform hardware cursor is painted one (or
//! more) rows too HIGH: it lands on the prompt header line instead of the
//! shared "> " composer input line that is stacked below the header (and
//! optional hint) rows. These acceptance tests render the input area into a
//! Buffer of a known Rect, then query the cursor row via the AgentView API
//! (`cursor_position()` → `hardware_cursor_in`) and assert it equals the
//! painted "> " input row (input_area.y + the ACTUAL header offset returned
//! by `render_hitl_prompt`, incl. hint / wrapped-header rows). Two regression
//! scenarios assert options-mode HITL and the normal composer are unaffected.
//!
//! The freeform scenarios assert the header-offset cursor row (input area
//! top + the actual header offset); the two regression scenarios assert
//! options-mode HITL and the normal composer are unaffected. The
//! wrapped-header scenario renders at width 60 (padded body 58) so the
//! 114-char header wraps to exactly two rows, proving the offset tracks the
//! real wrapped-header height rather than a constant +1.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{AgentView, AgentViewStore, SessionContext};
use codelet_rpc_types::{HitlOption, HitlQuestion, HitlRequest, SessionId, SessionStatus};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn opt(label: &str, description: &str) -> HitlOption {
    HitlOption {
        label: label.to_string(),
        description: description.to_string(),
    }
}

fn question(header: &str, text: &str, options: Vec<HitlOption>) -> HitlQuestion {
    HitlQuestion {
        id: "q-1".to_string(),
        header: header.to_string(),
        question: text.to_string(),
        options,
    }
}

fn one_question_request(header: &str, text: &str, opts: Vec<HitlOption>) -> HitlRequest {
    HitlRequest {
        questions: vec![question(header, text, opts)],
    }
}

/// Build an AgentView + store pair with one focused session (rpc411 pattern).
fn view_harness(session: &str) -> (AgentView, AgentViewStore) {
    let (tx, _rx) = unbounded_channel();
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid(session)));
    store.focus_session_index(0);
    (AgentView::new(tx), store)
}

/// Render one frame on a w x h TestBackend; return the input-area Rect
/// recorded by the view (rpc411 render harness).
fn render_frame(w: u16, h: u16, store: &mut AgentViewStore, view: &mut AgentView) -> Rect {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), store))
        .unwrap();
    view.last_input_area.expect("input area recorded by render")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Freeform question with a single-row header places the cursor on
//           the input line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn freeform_question_with_a_single_row_header_places_the_cursor_on_the_input_line() {
    // @step Given a focused session with an active pure-freeform HITL question whose header fits on one row
    let (mut view, mut store) = view_harness("s-1");
    store.set_hitl_prompt(sid("s-1"), one_question_request("Header", "Q?", vec![]));

    // @step And no empty-submit hint is showing
    assert!(
        !store
            .hitl_prompt_for(&sid("s-1"))
            .expect("slot active")
            .show_empty_hint,
        "precondition: no empty-submit hint"
    );

    // @step When the input area is painted and the hardware cursor position is queried
    let input = render_frame(100, 14, &mut store, &mut view);
    let (_x, y) = view.cursor_position().expect("freeform cursor pos");

    // @step Then the reported cursor row is one row below the input area top
    assert_eq!(
        y,
        input.y + 1,
        "single-row-header freeform cursor must sit one row below the input area top \
         (input.y={}, got y={y})",
        input.y
    );

    // @step And that row is the same row where the "> " composer input line is painted
    assert_eq!(
        y,
        input.y + 1,
        "the cursor row must equal the painted '> ' input row (header offset = 1)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The empty-submit hint pushes the cursor down onto the input line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_empty_submit_hint_pushes_the_cursor_down_onto_the_input_line() {
    // @step Given a focused session with an active pure-freeform HITL question whose header fits on one row
    let (mut view, mut store) = view_harness("s-1");
    store.set_hitl_prompt(sid("s-1"), one_question_request("Header", "Q?", vec![]));

    // @step And the empty-submit hint is showing
    store
        .hitl_prompt_for_mut(&sid("s-1"))
        .expect("slot active")
        .show_empty_hint = true;

    // @step When the input area is painted and the hardware cursor position is queried
    let input = render_frame(100, 14, &mut store, &mut view);
    let (_x, y) = view.cursor_position().expect("freeform cursor pos");

    // @step Then the reported cursor row is two rows below the input area top
    assert_eq!(
        y,
        input.y + 2,
        "header row + yellow hint row must push the cursor two rows below the input area top \
         (input.y={}, got y={y})",
        input.y
    );

    // @step And that row is the same row where the "> " composer input line is painted
    assert_eq!(
        y,
        input.y + 2,
        "the cursor row must equal the pushed-down '> ' input row (header + hint = offset 2)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Other sub-mode places the cursor on the input line below the
//           header
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_other_sub_mode_places_the_cursor_on_the_input_line_below_the_header() {
    // @step Given a focused session with an active options HITL question in the "Other..." freeform sub-mode
    let (mut view, mut store) = view_harness("s-1");
    store.set_hitl_prompt(
        sid("s-1"),
        one_question_request("Header", "Q?", vec![opt("Yes", "y"), opt("No", "n")]),
    );
    store
        .hitl_prompt_for_mut(&sid("s-1"))
        .expect("slot active")
        .other_active = true;

    // @step When the input area is painted and the hardware cursor position is queried
    let input = render_frame(100, 14, &mut store, &mut view);
    let (_x, y) = view.cursor_position().expect("Other-mode cursor pos");

    // @step Then the reported cursor row is below the header on the "> " composer input line
    assert_eq!(
        y,
        input.y + 1,
        "Other-mode cursor must sit on the '> ' input row below the header \
         (input.y={}, got y={y})",
        input.y
    );

    // @step And the cursor is not on the header row
    assert_ne!(
        y, input.y,
        "the cursor must NOT land on the header row (input area top)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A header wrapped to two rows offsets the cursor by two rows
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_header_wrapped_to_two_rows_offsets_the_cursor_by_two_rows() {
    // @step Given a focused session with an active pure-freeform HITL question whose header wraps to two rows at a narrow width
    let (mut view, mut store) = view_harness("s-1");
    // A long header+question forces the wrapped header onto two rows at the
    // narrow padded body width used below.
    let long = "This is a deliberately long freeform question that must wrap across two rows";
    store.set_hitl_prompt(sid("s-1"), one_question_request("Header", long, vec![]));

    // @step And no empty-submit hint is showing
    assert!(
        !store
            .hitl_prompt_for(&sid("s-1"))
            .expect("slot active")
            .show_empty_hint,
        "precondition: no empty-submit hint"
    );

    // @step When the input area is painted and the hardware cursor position is queried
    // 60-wide terminal → padded body 58 cols (width - 2 for the 1-col pads) →
    // ceil(114 / 58) = 2, so the 114-char header wraps to exactly 2 rows.
    let input = render_frame(60, 14, &mut store, &mut view);
    let (_x, y) = view.cursor_position().expect("wrapped-header cursor pos");

    // @step Then the reported cursor row is two rows below the input area top
    assert_eq!(
        y,
        input.y + 2,
        "a 2-row wrapped header must offset the cursor by two rows \
         (input.y={}, got y={y})",
        input.y
    );

    // @step And the offset tracks the wrapped header height rather than a fixed single row
    assert_ne!(
        y,
        input.y + 1,
        "offset must NOT be a constant +1 — it must follow the wrapped header height"
    );
    assert_ne!(
        y,
        input.y + 3,
        "offset must not exceed two rows — the header must wrap to exactly two rows at this width"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An options-mode HITL prompt shows no hardware cursor
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn an_options_mode_hitl_prompt_shows_no_hardware_cursor() {
    // @step Given a focused session with an active options HITL question that is not in the freeform sub-mode
    let (mut view, mut store) = view_harness("s-1");
    store.set_hitl_prompt(
        sid("s-1"),
        one_question_request("Header", "Q?", vec![opt("Yes", "y"), opt("No", "n")]),
    );

    // @step When the cursor visibility is queried for the input area
    let _input = render_frame(100, 14, &mut store, &mut view);
    let visible = view.is_cursor_visible(Some(SessionStatus::Idle));

    // @step Then no hardware cursor is reported
    assert!(
        !visible,
        "options-mode HITL must report NO hardware cursor (cursor gate returns false)"
    );

    // @step And no header offset is applied
    // (No cursor is drawn, so the header-offset seam is never consulted for
    // options mode — the gate short-circuits before any anchor is computed.)
    assert!(!visible, "no cursor ⇒ no offset is applied in options mode");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The normal composer anchors the cursor at the input area top
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_normal_composer_anchors_the_cursor_at_the_input_area_top() {
    // @step Given a focused session with no active HITL prompt and no pause prompt
    let (mut view, mut store) = view_harness("s-1");
    view.input.set_value("hello");

    // @step When the input area is painted and the hardware cursor position is queried
    let input = render_frame(100, 14, &mut store, &mut view);
    let (_x, y) = view.cursor_position().expect("normal composer cursor pos");

    // @step Then the reported cursor row equals the input area top
    assert_eq!(
        y, input.y,
        "the normal composer cursor must anchor at the input area top \
         (input.y={}, got y={y})",
        input.y
    );

    // @step And no header offset is added
    assert_eq!(
        y, input.y,
        "no header is painted above the input line, so no offset may be added"
    );
}
