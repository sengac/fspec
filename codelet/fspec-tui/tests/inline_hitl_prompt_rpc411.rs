// Feature: spec/features/inline-hitl-prompt.feature
//! RPC-411 — inline HITL prompt acceptance tests: rendering, selection
//! wrap, multi-question accumulation, Esc-cancel, slot lifecycle,
//! options-mode key/paste swallowing, freeform / Other mode (shared
//! composer input, empty-submit hint, draft consumption), and the
//! modal-removal source-shape lock.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::store::agent_view::hitl_state::HitlPromptState;
use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, FspecBackend, SessionContext, ViewMode,
};
use codelet_rpc_types::{
    HitlAnswer, HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId,
    SessionState, StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn opt(label: &str, description: &str) -> HitlOption {
    HitlOption {
        label: label.to_string(),
        description: description.to_string(),
    }
}

fn question(id: &str, header: &str, text: &str, options: Vec<HitlOption>) -> HitlQuestion {
    HitlQuestion {
        id: id.to_string(),
        header: header.to_string(),
        question: text.to_string(),
        options,
    }
}

fn one_question_request(id: &str, header: &str, text: &str, opts: Vec<HitlOption>) -> HitlRequest {
    HitlRequest {
        questions: vec![question(id, header, text, opts)],
    }
}

fn yes_no_request(id: &str) -> HitlRequest {
    one_question_request(
        id,
        "Header",
        "Question?",
        vec![opt("Yes", "desc-Yes"), opt("No", "desc-No")],
    )
}

// ─────────────────────────────────────────────────────────────────────────
// View-level harness (rpc406 pattern)
// ─────────────────────────────────────────────────────────────────────────

fn view_harness(sessions: &[&str]) -> (AgentView, AgentViewStore) {
    let (tx, _rx) = unbounded_channel();
    let mut store = AgentViewStore::default();
    for s in sessions {
        store.append_session(SessionContext::new(sid(s)));
    }
    store.focus_session_index(0);
    (AgentView::new(tx), store)
}

/// Render one frame on a w x h TestBackend; return every terminal row
/// as a String, the raw buffer, and the input-area Rect.
fn render_frame(
    w: u16,
    h: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> (Vec<String>, Buffer, Rect) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), store))
        .unwrap();
    let buf = term.backend().buffer().clone();
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    let input_area = view.last_input_area.expect("input area recorded by render");
    (rows, buf, input_area)
}

/// Terminal x coordinate of the first char of `needle` inside `row`.
fn char_x(row: &str, needle: &str) -> u16 {
    let pos = row
        .find(needle)
        .unwrap_or_else(|| panic!("`{needle}` not found in row: {row}"));
    row[..pos].chars().count() as u16
}

// ─────────────────────────────────────────────────────────────────────────
// App-level harness (rpc406 pattern)
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app(sessions: &[&str]) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for s in sessions {
        app.dispatch(Action::SessionCreated(sid(s)));
    }
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Paint one App frame so the AgentView caches the focused session's
/// HITL slot (`last_hitl`) for key routing.
fn render_app(app: &mut App) {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
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

/// Read the authoritative HITL slot (RPC-411 store/agent_view/hitl_state.rs).
fn hitl_slot<'a>(app: &'a App, session: &SessionId) -> Option<&'a HitlPromptState> {
    app.agent_view_store().hitl_prompt_for(session)
}

// ─────────────────────────────────────────────────────────────────────────
// Rendering — TS InputTransition.tsx:427-463 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Options question renders inline with radios, colors, Other entry, and footer
#[test]
fn options_question_renders_inline_with_radios_colors_other_entry_and_footer() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with one question "Header" / "Question?" and options "Deploy — push to prod" and "Skip — do nothing"
    store.set_hitl_prompt(
        sid("s-1"),
        one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Deploy", "push to prod"), opt("Skip", "do nothing")],
        ),
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's header row shows "⏸ Header: Question?" with a magenta "⏸ " glyph, a bold header, and no "[1/1]" progress marker
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Header: Question?"),
        "header row missing; got: {r0}"
    );
    assert!(
        !r0.contains("[1/1]"),
        "single-question request must show no [n/m] marker; got: {r0}"
    );
    let pause_x = char_x(r0, "⏸");
    assert_eq!(
        buf[(pause_x, input.y)].fg,
        Color::Magenta,
        "⏸ glyph must be magenta"
    );
    let header_x = char_x(r0, "Header");
    assert!(
        buf[(header_x, input.y)].modifier.contains(Modifier::BOLD),
        "header must be bold"
    );

    // @step And an option row shows " ● Deploy" with the radio and label green and " — push to prod" dim
    let y1 = input.y + 1;
    let r1 = &rows[y1 as usize];
    assert!(
        r1.contains(" ● Deploy — push to prod"),
        "selected option row missing; got: {r1}"
    );
    let radio_x = char_x(r1, "●");
    assert_eq!(buf[(radio_x, y1)].fg, Color::Green, "selected radio green");
    let label_x = char_x(r1, "Deploy");
    assert_eq!(buf[(label_x, y1)].fg, Color::Green, "selected label green");
    let desc_x = char_x(r1, "— push to prod");
    assert!(
        buf[(desc_x, y1)].modifier.contains(Modifier::DIM),
        "description must be dim"
    );

    // @step And an option row shows " ○ Skip" with the radio and label white
    let y2 = input.y + 2;
    let r2 = &rows[y2 as usize];
    assert!(r2.contains(" ○ Skip"), "unselected row missing; got: {r2}");
    let skip_radio_x = char_x(r2, "○");
    assert_eq!(
        buf[(skip_radio_x, y2)].fg,
        Color::White,
        "unselected radio white"
    );
    let skip_x = char_x(r2, "Skip");
    assert_eq!(buf[(skip_x, y2)].fg, Color::White, "unselected label white");

    // @step And the row below the real options shows " ○ Other..." with the label dim and italic
    let y3 = input.y + 3;
    let r3 = &rows[y3 as usize];
    assert!(r3.contains(" ○ Other..."), "Other row missing; got: {r3}");
    let other_x = char_x(r3, "Other...");
    assert!(
        buf[(other_x, y3)].modifier.contains(Modifier::DIM),
        "Other... label must be dim"
    );
    assert!(
        buf[(other_x, y3)].modifier.contains(Modifier::ITALIC),
        "Other... label must be italic"
    );

    // @step And the footer row shows " (↑/↓ Navigate | Enter Select | Esc Cancel)" in dim
    let y4 = input.y + 4;
    let r4 = &rows[y4 as usize];
    assert!(
        r4.contains(" (↑/↓ Navigate | Enter Select | Esc Cancel)"),
        "footer row missing; got: {r4}"
    );
    let footer_x = char_x(r4, "(↑/↓ Navigate");
    assert!(
        buf[(footer_x, y4)].modifier.contains(Modifier::DIM),
        "footer must be dim"
    );

    // @step And no modal layer is mounted on the compositor
    // (view-level harness has no compositor; the App-level lifecycle
    // scenario locks the no-modal invariant — here we lock that the
    // prompt paints inside the input area, not a popup.)
    assert!(input.height >= 5, "inline prompt occupies the input area");
}

// Scenario: Multi-question request shows a magenta question progress marker
#[test]
fn multi_question_request_shows_a_magenta_question_progress_marker() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with three options questions
    store.set_hitl_prompt(
        sid("s-1"),
        HitlRequest {
            questions: (1..=3)
                .map(|i| {
                    question(
                        &format!("q-{i}"),
                        &format!("Header-{i}"),
                        &format!("Question-{i}?"),
                        vec![opt("Yes", "y"), opt("No", "n")],
                    )
                })
                .collect(),
        },
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's header row shows "⏸ [1/3] Header-1: Question-1?"
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ [1/3] Header-1: Question-1?"),
        "multi-question header missing; got: {r0}"
    );

    // @step And the "[1/3] " progress marker is magenta
    let marker_x = char_x(r0, "[1/3]");
    assert_eq!(
        buf[(marker_x, input.y)].fg,
        Color::Magenta,
        "[1/3] marker must be magenta"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Selection wrap — useHitlInput.ts:210-222 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Up and Down wrap the selection across options and the virtual Other entry
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn up_and_down_wrap_the_selection_across_options_and_the_virtual_other_entry() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with one question with options "Deploy" and "Skip"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Deploy", "d"), opt("Skip", "s")],
        ),
    });
    render_app(&mut app);

    // @step When the user presses Down three times
    for _ in 0..3 {
        app.handle_event(&key_event(KeyCode::Down));
        drain_pending(&mut app).await;
    }

    // @step Then the HITL selection for s-1 is back on option 0
    assert_eq!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .selected_option,
        0,
        "3 Downs over 2 options + Other must wrap back to 0"
    );

    // @step When the user presses Up
    app.handle_event(&key_event(KeyCode::Up));
    drain_pending(&mut app).await;

    // @step Then the HITL selection for s-1 is on the virtual "Other..." entry at index 2
    assert_eq!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .selected_option,
        2,
        "Up from 0 must wrap to the virtual Other... index"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Multi-question accumulation — useHitlInput.ts:134-151 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Answers accumulate across questions and submit together after the last question
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answers_accumulate_across_questions_and_submit_together_after_the_last_question() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with two options questions "q-1" with options "Yes" and "No" and "q-2" with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: HitlRequest {
            questions: vec![
                question("q-1", "H1", "Q1?", vec![opt("Yes", "y"), opt("No", "n")]),
                question("q-2", "H2", "Q2?", vec![opt("Yes", "y"), opt("No", "n")]),
            ],
        },
    });
    render_app(&mut app);

    // @step When the user presses Enter on question 1 with "Yes" selected
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response has not been called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "nothing may be sent before the last question is answered"
    );

    // @step And the prompt advances to question 2 with the selection reset to 0
    let slot = hitl_slot(&app, &sid("s-1")).expect("slot still active");
    assert_eq!(slot.question_index, 1, "must advance to question 2");
    assert_eq!(slot.selected_option, 0, "selection must reset to 0");

    // @step When the user presses Down then Enter on question 2
    render_app(&mut app);
    app.handle_event(&key_event(KeyCode::Down));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled false and answers [{id q-1, selected [Yes]}, {id q-2, selected [No]}]
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(!response.cancelled, "submit must carry cancelled:false");
    assert_eq!(
        response.answers,
        vec![
            HitlAnswer {
                id: "q-1".to_string(),
                selected: vec!["Yes".to_string()],
                other: None,
            },
            HitlAnswer {
                id: "q-2".to_string(),
                selected: vec!["No".to_string()],
                other: None,
            },
        ]
    );

    // @step And the HITL prompt for s-1 is cleared
    assert!(
        hitl_slot(&app, &sid("s-1")).is_none(),
        "slot must clear after the final submit"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Esc/cancel correctness — the stranding-bug fix
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Esc on an options question cancels the whole request through the backend
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_an_options_question_cancels_the_whole_request_through_the_backend() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.navigator_mut().agent.input.set_value("keep this draft");

    // @step And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: yes_no_request("q-1"),
    });
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled true and empty answers
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(response.cancelled, "Esc must send cancelled:true");
    assert!(response.answers.is_empty(), "cancel carries no answers");

    // @step And the HITL prompt for s-1 is cleared
    assert!(
        hitl_slot(&app, &sid("s-1")).is_none(),
        "no path may leave the slot set after cancelling"
    );

    // @step And the composer draft is left untouched
    assert_eq!(
        app.navigator().agent.input.value(),
        "keep this draft",
        "Esc-cancel must not clear the remaining composer draft"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Slot lifecycle — HITL wins over pause, Running clears
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Paused chunk stores a HITL slot that wins over the pause slot and Running clears it
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paused_chunk_stores_a_hitl_slot_that_wins_over_the_pause_slot_and_running_clears_it() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(
        sid("s-1"),
        Some(PauseState {
            kind: PauseKind::Confirm,
            prompt: "p".to_string(),
            tool_call_id: None,
        }),
    );

    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    let request = yes_no_request("q-1");
    mock.script_hitl_request(sid("s-1"), Some(request.clone()));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));

    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || hitl_slot(&app, &sid("s-1")).is_some(),
        "HITL slot set from the Paused chunk",
    )
    .await;

    // @step Then the HITL prompt slot for s-1 holds the fetched request
    let slot = hitl_slot(&app, &sid("s-1")).expect("slot set");
    assert_eq!(slot.request, request, "slot must hold the wire request");

    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "HITL wins on tie — no pause slot may be set"
    );

    // @step And no compositor layer is mounted for the HITL prompt
    assert_eq!(
        app.compositor().len(),
        0,
        "the inline prompt must never mount a modal layer; got ids: {:?}",
        app.compositor().layer_ids()
    );

    // @step When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Running,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the HITL prompt slot for s-1 is empty
    assert!(
        hitl_slot(&app, &sid("s-1")).is_none(),
        "Running must clear the stale HITL prompt"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Removed invented behaviors + paste routing (options mode)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Hotkeys, Tab, scroll, and paste do nothing on an options question
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hotkeys_tab_scroll_and_paste_do_nothing_on_an_options_question() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the composer input contains the draft "agent draft"
    app.navigator_mut().agent.input.set_value("agent draft");

    // @step And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: yes_no_request("q-1"),
    });
    render_app(&mut app);
    let before = hitl_slot(&app, &sid("s-1"))
        .expect("slot active")
        .selected_option;

    // @step When the user types the characters "abc" and presses Tab
    for c in "abc".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
        drain_pending(&mut app).await;
    }
    app.handle_event(&key_event(KeyCode::Tab));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is NEVER called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "no hotkeys / Tab-cycle: letters and Tab must never select or submit"
    );

    // @step And the HITL selection for s-1 is unchanged
    assert_eq!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .selected_option,
        before,
        "letters and Tab must not move the selection"
    );

    // @step When a paste of "clipboard\ncontents" arrives
    let _ = app.handle_paste("clipboard\ncontents");
    drain_pending(&mut app).await;

    // @step Then the composer input still contains exactly "agent draft"
    assert_eq!(
        app.navigator().agent.input.value(),
        "agent draft",
        "options-mode paste must be swallowed, never reach the composer"
    );

    // @step And the HITL prompt for s-1 is still active
    assert!(
        hitl_slot(&app, &sid("s-1")).is_some(),
        "swallowed keys must not dismiss the prompt"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape — modal removal lock (mirrors RPC-406 pause-modal lock)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: The HITL modal is deleted and no construction site remains
#[test]
fn the_hitl_modal_is_deleted_and_no_construction_site_remains() {
    // @step Given the codelet-fspec-tui crate sources
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the file codelet/fspec-tui/src/components/hitl_dialog.rs does not exist
    assert!(
        !src.join("components").join("hitl_dialog.rs").exists(),
        "components/hitl_dialog.rs must be deleted"
    );

    // @step And the crate sources never mention HITL_DIALOG_ID or HitlDialog
    fn rust_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    for f in &files {
        let body = std::fs::read_to_string(f).expect("read source file");
        assert!(
            !body.contains("HITL_DIALOG_ID"),
            "{} still mentions HITL_DIALOG_ID",
            f.display()
        );
        assert!(
            !body.contains("HitlDialog"),
            "{} still mentions HitlDialog",
            f.display()
        );
    }

    // @step And components/mod.rs declares no OpenHitlDialog action variant
    let components =
        std::fs::read_to_string(src.join("components").join("mod.rs")).expect("read mod.rs");
    assert!(
        !components.contains("OpenHitlDialog"),
        "Action::OpenHitlDialog must be replaced by Action::HitlPromptFetched"
    );

    // @step And the files hitl_state.rs, hitl_keys.rs, and hitl_prompt.rs each stay under 300 lines
    let files = [
        src.join("store").join("agent_view").join("hitl_state.rs"),
        src.join("views").join("agent").join("hitl_keys.rs"),
        src.join("views").join("agent").join("hitl_prompt.rs"),
    ];
    for f in files {
        let body =
            std::fs::read_to_string(&f).unwrap_or_else(|_| panic!("{} must exist", f.display()));
        let n = body.lines().count();
        assert!(n < 300, "{} has {n} lines, must be < 300", f.display());
    }
}

/// Paint one App frame and return the rendered rows + buffer (used by
/// the freeform/Other-mode scenarios to assert painted content).
fn render_app_frame(app: &mut App) -> (Vec<String>, Buffer) {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    let rows: Vec<String> = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    (rows, buf)
}

/// Drive the App into Other mode on a fresh one-question options
/// prompt: navigate the selection onto the virtual "Other..." entry
/// (index = options.len()) and press Enter.
async fn enter_other_mode(app: &mut App) {
    render_app(app);
    app.handle_event(&key_event(KeyCode::Up));
    drain_pending(app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(app).await;
    render_app(app);
}

// ─────────────────────────────────────────────────────────────────────────
// Other... freeform mode — TOOL-018 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Enter on Other activates freeform mode with the shared input, empty-submit hint, and submit
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_on_other_activates_freeform_mode_with_the_shared_input_empty_submit_hint_and_submit()
{
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with one question "q-1" with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    });

    // @step When the user navigates the selection to "Other..." and presses Enter
    enter_other_mode(&mut app).await;

    // @step Then the prompt is in Other mode and the header hint shows " (Enter Submit | Esc Back to options)" in dim
    assert!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .other_active,
        "Enter on Other... must activate Other mode"
    );
    let (rows, buf) = render_app_frame(&mut app);
    let header_y = (0..rows.len())
        .find(|&y| rows[y].contains("⏸ Header: Question?"))
        .expect("HITL header row rendered");
    let hr = &rows[header_y];
    assert!(
        hr.contains(" (Enter Submit | Esc Back to options)"),
        "Other-mode header hint missing; got: {hr}"
    );
    let hint_x = char_x(hr, "(Enter Submit | Esc Back to options)");
    assert!(
        buf[(hint_x, header_y as u16)]
            .modifier
            .contains(Modifier::DIM),
        "header hint must be dim"
    );

    // @step And the shared composer input renders with the placeholder "Type your answer..."
    assert!(
        rows.iter().any(|r| r.contains("Type your answer...")),
        "shared input placeholder missing"
    );

    // @step When the user presses Enter with the shared input empty
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then the yellow hint "  ⚠ Please type a response or press Esc to go back" is shown
    assert!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .show_empty_hint,
        "empty Enter must set show_empty_hint"
    );
    let (rows, buf) = render_app_frame(&mut app);
    let hint_y = (0..rows.len())
        .find(|&y| rows[y].contains("⚠ Please type a response or press Esc to go back"))
        .expect("empty-submit hint row rendered");
    let warn_x = char_x(&rows[hint_y], "⚠");
    assert_eq!(
        buf[(warn_x, hint_y as u16)].fg,
        Color::Yellow,
        "empty-submit hint must be yellow"
    );

    // @step And backend.send_hitl_response has not been called
    assert_eq!(mock.send_hitl_response_calls(), 0);

    // @step When the user types "ship it"
    for c in "ship it".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
        drain_pending(&mut app).await;
    }

    // @step Then the empty-submit hint is cleared
    assert!(
        !hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .show_empty_hint,
        "typing must clear the empty-submit hint"
    );

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled false and one answer {id q-1, selected [], other "ship it"}
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(!response.cancelled);
    assert_eq!(
        response.answers,
        vec![HitlAnswer {
            id: "q-1".to_string(),
            selected: vec![],
            other: Some("ship it".to_string()),
        }]
    );

    // @step And the shared composer input is empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "captured answer must clear the shared input"
    );

    // @step And the HITL prompt for s-1 is cleared
    assert!(hitl_slot(&app, &sid("s-1")).is_none());
}

// Scenario: Esc in Other mode returns to the options list without sending anything
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_in_other_mode_returns_to_the_options_list_without_sending_anything() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt in Other mode with the empty-submit hint showing and "half an answer" in the shared input
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    });
    enter_other_mode(&mut app).await;
    app.handle_event(&key_event(KeyCode::Enter)); // empty submit → hint
    drain_pending(&mut app).await;
    assert!(hitl_slot(&app, &sid("s-1")).expect("slot").show_empty_hint);
    app.navigator_mut().agent.input.set_value("half an answer");
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is NEVER called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "Esc in Other mode is local only — nothing may be sent"
    );

    // @step And the prompt is back in options mode with the empty-submit hint cleared
    let slot = hitl_slot(&app, &sid("s-1")).expect("slot still active");
    assert!(!slot.other_active, "must return to options mode");
    assert!(!slot.show_empty_hint, "hint must be cleared");

    // @step And the shared composer input is empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "Esc from Other mode must clear the shared input value"
    );

    // @step And the HITL prompt for s-1 is still active
    assert!(hitl_slot(&app, &sid("s-1")).is_some());
}

// ─────────────────────────────────────────────────────────────────────────
// Freeform question — shared composer draft semantics
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Freeform question consumes the composer draft as the answer
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn freeform_question_consumes_the_composer_draft_as_the_answer() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the composer input contains the draft "deploy the fix"
    app.navigator_mut().agent.input.set_value("deploy the fix");

    // @step And session s-1 has an inline HITL prompt with one freeform question "q-1" without options
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request("q-1", "Header", "Question?", vec![]),
    });

    // @step When the agent view renders a frame
    let (rows, buf) = render_app_frame(&mut app);

    // @step Then the header hint shows " (Enter Submit | Esc Cancel)" in dim
    let header_y = (0..rows.len())
        .find(|&y| rows[y].contains("⏸ Header: Question?"))
        .expect("HITL header row rendered");
    let hr = &rows[header_y];
    assert!(
        hr.contains(" (Enter Submit | Esc Cancel)"),
        "plain-freeform header hint missing; got: {hr}"
    );
    let hint_x = char_x(hr, "(Enter Submit | Esc Cancel)");
    assert!(
        buf[(hint_x, header_y as u16)]
            .modifier
            .contains(Modifier::DIM),
        "header hint must be dim"
    );

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled false and one answer {id q-1, selected [], other "deploy the fix"}
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(!response.cancelled);
    assert_eq!(
        response.answers,
        vec![HitlAnswer {
            id: "q-1".to_string(),
            selected: vec![],
            other: Some("deploy the fix".to_string()),
        }]
    );

    // @step And the shared composer input is empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "the consumed draft must be cleared after capture"
    );
}

// Scenario: Composer draft and cursor survive an options-only HITL round-trip
#[test]
fn composer_draft_and_cursor_survive_an_options_only_hitl_round_trip() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    view.input.set_value("deploy the fix");
    assert_eq!(view.input.cursor(), (0, 14), "cursor parked after 'fix'");

    // @step And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    store.set_hitl_prompt(
        sid("s-1"),
        one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    );

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the rendered input area does not contain the text "deploy the fix"
    for y in input.y..input.y + input.height {
        assert!(
            !rows[y as usize].contains("deploy the fix"),
            "draft chars must not paint during an options question; row {y}: {}",
            rows[y as usize]
        );
    }

    // @step When the HITL prompt for s-1 is cleared as if the user answered
    store.clear_hitl_prompt(&sid("s-1"));

    // @step And the agent view renders another frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);
    assert!(
        rows[input.y as usize].contains("deploy the fix"),
        "draft repainted after the round-trip; got: {}",
        rows[input.y as usize]
    );

    // @step Then the input buffer contains exactly "deploy the fix"
    assert_eq!(view.input.value(), "deploy the fix");

    // @step And the cursor is at the end of "deploy the fix"
    assert_eq!(
        view.input.cursor(),
        (0, 14),
        "cursor untouched by the options-only round-trip"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Paste routing — freeform/Other mode
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Paste in Other mode inserts into the shared input
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paste_in_other_mode_inserts_into_the_shared_input() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt in Other mode with an empty shared input
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    });
    enter_other_mode(&mut app).await;
    assert!(hitl_slot(&app, &sid("s-1")).expect("slot").other_active);
    assert_eq!(app.navigator().agent.input.value(), "");

    // @step When a paste of "pasted answer" arrives
    let _ = app.handle_paste("pasted answer");
    drain_pending(&mut app).await;

    // @step Then the shared composer input contains "pasted answer"
    assert_eq!(
        app.navigator().agent.input.value(),
        "pasted answer",
        "Other-mode paste must fall through to the shared MultiLineInput"
    );

    // @step And backend.send_hitl_response is NEVER called
    assert_eq!(mock.send_hitl_response_calls(), 0);
}
