// Feature: spec/features/exec-stdin-prompt.feature
//! TOOL-022 P2 — TUI inline exec-stdin prompt acceptance tests.
//!
//! Covers the 12 P2 scenarios of spec/features/exec-stdin-prompt.feature
//! that live at the TUI layer (the backend-level ones — newline append,
//! clean unknown-session error, detector round-trip, no-status-flip,
//! cooldown — are covered by rust/sessions/tests/exec_stdin_prompt_p2.rs
//! and the tools inline detector tests).
//!
//! Harness mirrors `inline_hitl_prompt_rpc411.rs`:
//! - view-level: view_harness + render_frame on a TestBackend + char_x
//! - app-level: fresh_app + drain_pending + wait_until, MockBackend
//!   script_exec_stdin_request / write_exec_stdin_log / set_*_error

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, FspecBackend, SessionContext, ViewMode,
};
use codelet_rpc_types::{
    ExecStdinRequest, HitlRequest, HitlQuestion, PauseKind, PauseState, SessionId,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
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

fn exec_request(exec_id: &str, command: &str, quiet_seconds: i64) -> ExecStdinRequest {
    ExecStdinRequest {
        exec_session_id: exec_id.to_string(),
        command: command.to_string(),
        quiet_seconds,
        ts_ms: 1_700_000_000_000,
    }
}

fn freeform_hitl_request() -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "q-1".to_string(),
            header: "Header".to_string(),
            question: "Question?".to_string(),
            options: vec![],
        }],
    }
}

// ─────────────────────────────────────────────────────────────────────────
// View-level harness
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

/// Render one frame on a w x h TestBackend; return the rows, the raw
/// buffer, and the input-area Rect.
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
// App-level harness
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
/// exec-stdin slot (`last_exec_stdin`) for key routing.
fn render_app(app: &mut App) {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}

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

/// Read the authoritative exec-stdin slot (store/agent_view/exec_stdin_state.rs).
fn exec_stdin_slot<'a>(app: &'a App, session: &SessionId) -> Option<&'a ExecStdinRequest> {
    app.agent_view_store().exec_stdin_for(session)
}

/// Paint one App frame into a 160x30 TestBackend buffer (mux-grid size).
fn render_app_wide(app: &mut App) -> (ratatui::buffer::Buffer, Rect) {
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 30)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    (
        term.backend().buffer().clone(),
        ratatui::layout::Rect::new(0, 0, 160, 30),
    )
}

/// The text painted inside a pane rect (rows x cols joined by '\n').
fn pane_text(buf: &ratatui::buffer::Buffer, pane: Rect) -> String {
    (0..pane.height)
        .map(|i| {
            (pane.x..pane.x + pane.width)
                .map(|x| buf[(x, pane.y + i)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: TUI renders the exec-stdin prompt in the composer slot
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: TUI renders the exec-stdin prompt in the composer slot
#[test]
fn tui_renders_the_exec_stdin_prompt_in_the_composer_slot() {
    // @step Given an agent session has a pending exec-stdin request for command "git commit" quiet for 5 seconds
    let (mut view, mut store) = view_harness(&["s-1"]);
    store.set_exec_stdin(
        sid("s-1"),
        exec_request("exec-abc", "git commit", 5),
    );

    // @step And the agent view is focused on that agent session
    // (view_harness focuses index 0; s-1 is the only session)

    // @step When the agent view paints the input area
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area shows the exec-stdin prompt line with a magenta keyboard glyph
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⌨ git commit"),
        "header row must start with the magenta keyboard glyph + command; got: {r0}"
    );
    let glyph_x = char_x(r0, "⌨");
    assert_eq!(
        buf[(glyph_x, input.y)].fg,
        Color::Magenta,
        "⌨ glyph must be magenta"
    );

    // @step And the prompt line shows the command display and how long it has been quiet
    assert!(
        r0.contains("git commit"),
        "command display missing from header; got: {r0}"
    );
    assert!(
        r0.contains("has been quiet for 5s"),
        "quiet-duration tail missing from header; got: {r0}"
    );

    // @step And the prompt line shows no command output content
    // The header spans are exactly: glyph + command + quiet tail. No
    // output-content span exists (the request carries no hint/content
    // field — see rule [11] "mask secrets by elimination").
    assert!(
        r0.contains("waiting for input?"),
        "header must end with the waiting-for-input tail; got: {r0}"
    );
    assert!(
        !r0.contains("Command still running"),
        "P1 steering line must NOT leak into the header; got: {r0}"
    );

    // @step And the input area shows the shared input with placeholder "Type to send to the command…"
    let placeholder = "Type to send to the command…";
    let input_rows: Vec<String> = (input.y..input.y + input.height)
        .map(|y| rows[y as usize].clone())
        .collect();
    assert!(
        input_rows.iter().any(|r| r.contains(placeholder)),
        "shared input placeholder missing; rows: {input_rows:?}"
    );

    // @step And the input area shows the dim footer "(Enter Send | Esc Dismiss)"
    let footer_y = input.y + input.height - 1;
    let r_footer = &rows[footer_y as usize];
    assert!(
        r_footer.contains("(Enter Send | Esc Dismiss)"),
        "dim footer missing; got: {r_footer}"
    );

    // @step And the exec-stdin prompt is painted in the composer slot ahead of the pause slot
    // Precedence (input_area.rs): HITL > exec-stdin > pause > composer.
    // With ONLY a pause slot + an exec-stdin slot, the exec-stdin
    // overlay must win (no ⏸ painted).
    let (mut view2, mut store2) = view_harness(&["s-2"]);
    store2.set_pause_state(
        sid("s-2"),
        PauseState {
            kind: PauseKind::Confirm,
            prompt: "approve?".to_string(),
            tool_call_id: None,
        },
    );
    store2.set_exec_stdin(sid("s-2"), exec_request("exec-x", "cmd-x", 3));
    let (rows2, _buf2, input2) = render_frame(100, 14, &mut store2, &mut view2);
    let input_rows2: Vec<String> = (input2.y..input2.y + input2.height)
        .map(|y| rows2[y as usize].clone())
        .collect();
    assert!(
        input_rows2.iter().any(|r| r.contains("⌨ cmd-x")),
        "exec-stdin overlay must beat the pause slot; rows: {input_rows2:?}"
    );
    assert!(
        !input_rows2.iter().any(|r| r.contains("⏸ approve?")),
        "pause slot must not paint while exec-stdin occupies the composer; rows: {input_rows2:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: HITL prompt takes precedence over the exec-stdin prompt
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: HITL prompt takes precedence over the exec-stdin prompt
#[test]
fn hitl_prompt_takes_precedence_over_the_exec_stdin_prompt() {
    // @step Given an agent session has a pending exec-stdin request
    let (mut view, mut store) = view_harness(&["s-1"]);
    store.set_exec_stdin(sid("s-1"), exec_request("exec-abc", "git commit", 5));

    // @step And an agent session has a pending HITL request
    store.set_hitl_prompt(sid("s-1"), freeform_hitl_request());

    // @step When the agent view paints the input area
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area shows the HITL prompt
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Header: Question?"),
        "HITL header must paint in the composer slot; got: {r0}"
    );

    // @step And the input area does not show the exec-stdin prompt line
    let input_rows: Vec<String> = (input.y..input.y + input.height)
        .map(|y| rows[y as usize].clone())
        .collect();
    assert!(
        !input_rows.iter().any(|r| r.contains("has been quiet for")),
        "exec-stdin header must not paint while HITL occupies the slot; rows: {input_rows:?}"
    );
    assert!(
        !input_rows.iter().any(|r| r.contains("Type to send to the command")),
        "exec-stdin shared-input placeholder must not paint; rows: {input_rows:?}"
    );
    assert!(
        !input_rows.iter().any(|r| r.contains("(Enter Send | Esc Dismiss)")),
        "exec-stdin footer must not paint; rows: {input_rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter submits the typed text to the exec session stdin and
// clears the prompt
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter submits the typed text to the exec session stdin and clears the prompt
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_submits_the_typed_text_to_the_exec_session_stdin_and_clears_the_prompt() {
    // @step Given an agent session has a pending exec-stdin request for exec session "exec-abc"
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::ExecStdinPromptFetched {
        agent_session: sid("s-1"),
        request: exec_request("exec-abc", "git commit", 5),
    });
    render_app(&mut app);
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "slot must be populated before typing"
    );

    // @step And the user types "y" into the shared input
    app.handle_event(&key_event(KeyCode::Char('y')));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.input.value(),
        "y",
        "typed char must land in the shared input"
    );

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then the backend write_exec_stdin is called with text "y" for exec session "exec-abc"
    wait_until(
        || mock.write_exec_stdin_calls() == 1,
        "write_exec_stdin called once",
    )
    .await;
    let log = mock.write_exec_stdin_log();
    assert_eq!(log.len(), 1, "exactly one write_exec_stdin call expected");
    let (agent_session, exec_session, text) = &log[0];
    assert_eq!(agent_session, &sid("s-1"), "agent session must match");
    assert_eq!(exec_session, "exec-abc", "exec session id must match");
    assert_eq!(text, "y", "typed text must be submitted verbatim");

    // @step And the exec-stdin prompt slot is cleared for the agent session
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "slot must clear after a successful submit"
    );

    // @step And the shared input is cleared after submit
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "shared input must be cleared after submit"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc dismisses the exec-stdin prompt without cancelling anything
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Esc dismisses the exec-stdin prompt without cancelling anything
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_dismisses_the_exec_stdin_prompt_without_cancelling_anything() {
    // @step Given an agent session has a pending exec-stdin request
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::ExecStdinPromptFetched {
        agent_session: sid("s-1"),
        request: exec_request("exec-abc", "git commit", 5),
    });
    render_app(&mut app);
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "slot must be populated before Esc"
    );

    // @step And the shared input holds the draft "draft-text"
    app.navigator_mut().agent.input.set_value("draft-text");
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then the exec-stdin prompt slot is cleared for the agent session
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "Esc must clear the exec-stdin slot"
    );

    // @step And the agent session status remains running
    // Pure overlay: NO backend call may have been made (no status flip,
    // no cancel, no write). The only backend surface is write_exec_stdin.
    assert_eq!(
        mock.write_exec_stdin_calls(),
        0,
        "Esc must not send anything to the backend"
    );

    // @step And the shared input draft is preserved
    assert_eq!(
        app.navigator().agent.input.value(),
        "draft-text",
        "Esc must preserve the composer draft"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: No exec-stdin prompt while no exec session is waiting
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: No exec-stdin prompt while no exec session is waiting
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_exec_stdin_prompt_while_no_exec_session_is_waiting() {
    // @step Given an agent session with no pending exec-stdin request
    // (two sessions; s-2 is focused after SessionCreated ordering, s-1
    // carries NO exec-stdin request — the default backend answer)
    let (mut app, mock) = fresh_app(&["s-1", "s-2"]);

    // @step When the agent view probes the backend for the exec-stdin request
    // (probe runs on focus switch — SessionPrev moves focus s-2 → s-1
    // and re-probes s-1)
    app.dispatch(Action::SessionPrev);
    drain_pending(&mut app).await;

    // @step Then the backend returns no request
    wait_until(
        || mock.get_exec_stdin_request_calls() >= 1,
        "get_exec_stdin_request probed at least once",
    )
    .await;
    assert_eq!(
        mock.get_exec_stdin_request_calls(),
        1,
        "exactly one probe expected"
    );

    // @step And the input area shows the plain composer
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "no slot may be populated when the backend returns None"
    );
    let (rows, _buf) = render_app_frame(&mut app);
    assert!(
        rows.iter().any(|r| r.contains("Type a message...")),
        "plain composer placeholder must paint; rows: {rows:?}"
    );
    assert!(
        !rows.iter().any(|r| r.contains("has been quiet for")),
        "no exec-stdin header may paint; rows: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Slot is cleared when the exec session no longer exists
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Slot is cleared when the exec session no longer exists
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slot_is_cleared_when_the_exec_session_no_longer_exists() {
    // @step Given an agent session has a pending exec-stdin request for exec session "exec-gone"
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::ExecStdinPromptFetched {
        agent_session: sid("s-1"),
        request: exec_request("exec-gone", "git commit", 10),
    });
    render_app(&mut app);
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "slot must be populated before the re-probe"
    );

    // @step And exec session "exec-gone" has exited
    // (the backend's get_exec_stdin_request now returns None — the
    // request is gone from the BackgroundSession)
    mock.script_exec_stdin_request(sid("s-1"), None);

    // @step When the agent view re-probes the backend on focus
    // (a Paused chunk for s-1 re-issues the 3-way probe
    //  get_pause_state + get_hitl_request + get_exec_stdin_request —
    //  the same re-probe path used on focus; backend returns None)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        codelet_rpc_types::StreamChunk::SessionStateChange {
            state: codelet_rpc_types::SessionState::Paused,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the backend returns no request
    wait_until(
        || mock.get_exec_stdin_request_calls() >= 1,
        "re-probe hit the backend",
    )
    .await;

    // @step And the exec-stdin prompt slot is cleared for the agent session
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "slot must clear when the re-probe returns None"
    );
    let (rows, _buf) = render_app_frame(&mut app);
    assert!(
        !rows.iter().any(|r| r.contains("has been quiet for")),
        "exec-stdin header must not paint after the slot cleared; rows: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Ghost panes do not render the exec-stdin prompt
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Ghost panes do not render the exec-stdin prompt
#[tokio::test]
async fn ghost_panes_do_not_render_the_exec_stdin_prompt() {
    // @step Given agent session A has a pending exec-stdin request
    let (mut app, _mock) = fresh_app(&["s-1", "s-2"]);
    app.dispatch(Action::ExecStdinPromptFetched {
        agent_session: sid("s-1"),
        request: exec_request("exec-a", "cmd-a", 5),
    });
    drain_pending(&mut app).await;

    // @step And agent session B is focused while agent session A is a ghost pane
    // (mux board + agent + agent; B = the second agent pane, focused)
    app.dispatch(Action::InputSubmitted(
        "/mux board agent agent".to_string(),
    ));
    drain_pending(&mut app).await;
    let rects = app.navigator().mux.pane_rects();
    let b_pane = *rects.get(2).expect("pane 2 (agent B) rect must exist");
    let a_pane = *rects.get(1).expect("pane 1 (agent A) rect must exist");
    // Focus pane B (agent 2) via the production focus path.
    app.navigator_mut().mux.set_focus(2);

    // @step When the agent view paints all panes
    let (buf, _area) = render_app_wide(&mut app);

    // @step Then the focused pane input area shows no exec-stdin prompt line
    let b_text = pane_text(&buf, b_pane);
    assert!(
        !b_text.contains("has been quiet for"),
        "focused pane B (no pending request) must not paint the exec-stdin prompt; pane: {b_text}"
    );

    // @step And only the focused agent pane renders an exec-stdin prompt when it has one
    // Ghost pane A carries a pending request but is unfocused — the
    // ghost input row paints the draft only, never the overlay.
    let a_text = pane_text(&buf, a_pane);
    assert!(
        !a_text.contains("has been quiet for"),
        "ghost pane A must NOT paint the exec-stdin prompt (focused-pane-only); pane: {a_text}"
    );
    assert!(
        !a_text.contains("Type to send to the command"),
        "ghost pane A must not paint the exec-stdin shared input; pane: {a_text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Reducer edge: HITL occupying the slot swallows a fresh exec-stdin fetch
// (rule [9] guard in handle_exec_stdin_prompt_fetched)
// ─────────────────────────────────────────────────────────────────────────

/// While a HITL prompt occupies the composer slot for a session, an
/// arriving ExecStdinPromptFetched must NOT populate the exec-stdin
/// slot (it clears instead) — the overlay must never show behind HITL.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_occupying_the_slot_swallow_a_fresh_exec_stdin_fetch() {
    // @step Given an agent session has a pending HITL request
    let (mut app, _mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: freeform_hitl_request(),
    });
    render_app(&mut app);
    assert!(
        app.agent_view_store()
            .hitl_prompt_for(&sid("s-1"))
            .is_some(),
        "HITL slot must be populated"
    );

    // @step When an exec-stdin request for the same session arrives
    app.dispatch(Action::ExecStdinPromptFetched {
        agent_session: sid("s-1"),
        request: exec_request("exec-abc", "git commit", 5),
    });

    // @step Then the exec-stdin slot is NOT populated (HITL wins)
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "exec-stdin slot must stay empty while HITL occupies the composer"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Submit-failure edge: a failed write keeps the slot (tracing only)
// ─────────────────────────────────────────────────────────────────────────

/// On `write_exec_stdin` failure the slot is KEPT (so a re-probe can
/// re-fire) and the error is logged via tracing — never a scrollback
/// notice (rule [3]: "the error is logged via tracing — never a
/// scrollback notice").
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_submit_keeps_the_slot_and_sends_nothing_else() {
    // @step Given an agent session has a pending exec-stdin request for exec session "exec-abc"
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::ExecStdinPromptFetched {
        agent_session: sid("s-1"),
        request: exec_request("exec-abc", "git commit", 5),
    });
    render_app(&mut app);

    // @step And the backend's write_exec_stdin is scripted to fail
    mock.set_write_exec_stdin_error("boom".to_string());

    // @step And the user types "x" and presses Enter
    app.handle_event(&key_event(KeyCode::Char('x')));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then the backend write_exec_stdin was attempted
    wait_until(
        || mock.write_exec_stdin_calls() == 1,
        "write_exec_stdin attempted",
    )
    .await;

    // @step And the exec-stdin prompt slot is KEPT (re-probe will re-fire)
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "failed submit must keep the slot so the user can retry"
    );

    // @step And no scrollback notice was appended (only tracing logged)
    // The session scrollback must carry no text mentioning the error.
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context");
    let leaked = ctx.scrollback.chunks().iter().any(|chunk| {
        chunk.lines.iter().any(|line| {
            line.spans.iter().any(|span| span.content.contains("boom"))
        })
    });
    assert!(
        !leaked,
        "error text must not leak into scrollback (tracing only)"
    );
}
