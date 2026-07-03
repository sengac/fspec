// Feature: spec/features/inline-tool-approval-pause-prompt.feature
//! RPC-406 — inline tool-approval pause prompt acceptance tests.
//!
//! Covers the TS-parity inline prompt that replaces the RPC-053
//! PauseDialog modal: rendering (TestBackend buffer assertions), key
//! routing (Esc DENIES), chunk-driven store-slot lifecycle, draft
//! preservation, and the modal-removal source-shape lock.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, FspecBackend, SessionContext, ViewMode,
};
use codelet_rpc_types::{
    ApprovalChoice, HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, SessionId,
    SessionState, SessionStatus, StreamChunk,
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

fn triple_state(prompt: &str, details: Option<&str>) -> PauseState {
    PauseState {
        kind: PauseKind::Triple,
        prompt: prompt.to_string(),
        tool_call_id: details.map(str::to_string),
    }
}

fn confirm_state(prompt: &str, details: Option<&str>) -> PauseState {
    PauseState {
        kind: PauseKind::Confirm,
        prompt: prompt.to_string(),
        tool_call_id: details.map(str::to_string),
    }
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// View-level harness (rpc404/rpc405 pattern)
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
// App-level harness (rpc053 pattern + rendered frame to seed key cache)
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

/// Paint one App frame into a scratch buffer so the AgentView caches
/// the focused session's pause slot for key routing.
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

// RPC-410: multi-question wire shape (single-question fixture).
fn hitl_request(id: &str, labels: &[&str]) -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: id.to_string(),
            header: "Apply the proposed change?".to_string(),
            question: "Continue?".to_string(),
            options: labels
                .iter()
                .map(|l| HitlOption {
                    label: (*l).to_string(),
                    description: format!("desc-{l}"),
                })
                .collect(),
        }],
    }
}

fn pause_dialog_layer_mounted(app: &App) -> bool {
    app.compositor()
        .layer_ids()
        .iter()
        .any(|id| id == "pause-dialog")
}

// ─────────────────────────────────────────────────────────────────────────
// Rendering — TS InputTransition.tsx:467-533 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Triple pause prompt renders inline with options, colors, and hint
#[test]
fn triple_pause_prompt_renders_inline_with_options_colors_and_hint() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Triple pause slot with prompt "Read: Access to .env requires approval" and details ".env"
    store.set_pause_state(
        sid("s-1"),
        triple_state("Read: Access to .env requires approval", Some(".env")),
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's first row shows "⏸ Read: Access to .env requires approval (.env)"
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Read: Access to .env requires approval (.env)"),
        "triple prompt row missing; got: {r0}"
    );

    // @step And the "⏸ Read" prefix is cyan and the "(.env)" details are dim
    let pause_x = char_x(r0, "⏸");
    assert_eq!(
        buf[(pause_x, input.y)].fg,
        Color::Cyan,
        "⏸ glyph must be cyan"
    );
    let read_x = char_x(r0, "Read:");
    assert_eq!(
        buf[(read_x, input.y)].fg,
        Color::Cyan,
        "tool name must be cyan"
    );
    let details_x = char_x(r0, "(.env)");
    assert!(
        buf[(details_x, input.y)].modifier.contains(Modifier::DIM),
        "details must be dim"
    );

    // @step And the input area's second row shows "[Allow Once] [Allow Session] [Deny] (←/→ Navigate | Enter Select | Esc Deny)"
    let y1 = input.y + 1;
    let r1 = &rows[y1 as usize];
    assert!(
        r1.contains("[Allow Once] [Allow Session] [Deny] (←/→ Navigate | Enter Select | Esc Deny)"),
        "options row missing; got: {r1}"
    );

    // @step And "[Allow Once]" is green, "[Allow Session]" is blue, and "[Deny]" is red
    let once_x = char_x(r1, "[Allow Once]");
    let session_x = char_x(r1, "[Allow Session]");
    let deny_x = char_x(r1, "[Deny]");
    assert_eq!(buf[(once_x, y1)].fg, Color::Green, "[Allow Once] green");
    assert_eq!(buf[(session_x, y1)].fg, Color::Blue, "[Allow Session] blue");
    assert_eq!(buf[(deny_x, y1)].fg, Color::Red, "[Deny] red");

    // @step And "[Allow Once]" is rendered inverse because the selection defaults to 0
    assert!(
        buf[(once_x, y1)].modifier.contains(Modifier::REVERSED),
        "[Allow Once] must be inverse when selection is 0"
    );
    assert!(
        !buf[(session_x, y1)].modifier.contains(Modifier::REVERSED),
        "[Allow Session] must NOT be inverse"
    );
    assert!(
        !buf[(deny_x, y1)].modifier.contains(Modifier::REVERSED),
        "[Deny] must NOT be inverse"
    );

    // @step And the navigation hint is dim
    let hint_x = char_x(r1, "(←/→ Navigate");
    assert!(
        buf[(hint_x, y1)].modifier.contains(Modifier::DIM),
        "hint must be dim"
    );
}

// Scenario: Confirm pause prompt renders yellow header, dim details line, and Y/N row
#[test]
fn confirm_pause_prompt_renders_yellow_header_dim_details_and_yn_row() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot with prompt "Bash: Run rm -rf build?" and details "rm -rf build"
    store.set_pause_state(
        sid("s-1"),
        confirm_state("Bash: Run rm -rf build?", Some("rm -rf build")),
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's first row shows "⏸ Bash: Run rm -rf build?" in yellow
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Bash: Run rm -rf build?"),
        "confirm header missing; got: {r0}"
    );
    let pause_x = char_x(r0, "⏸");
    assert_eq!(
        buf[(pause_x, input.y)].fg,
        Color::Yellow,
        "confirm header must be yellow"
    );

    // @step And the input area's second row shows the dim details line "rm -rf build"
    let y1 = input.y + 1;
    let r1 = &rows[y1 as usize];
    assert!(
        r1.contains("rm -rf build"),
        "details line missing; got: {r1}"
    );
    let details_x = char_x(r1, "rm -rf build");
    assert!(
        buf[(details_x, y1)].modifier.contains(Modifier::DIM),
        "details line must be dim"
    );

    // @step And the input area's third row shows "[Y] Approve [N] Deny (Esc to cancel)"
    let y2 = input.y + 2;
    let r2 = &rows[y2 as usize];
    assert!(
        r2.contains("[Y] Approve [N] Deny (Esc to cancel)"),
        "Y/N row missing; got: {r2}"
    );

    // @step And "[Y] Approve" is green, "[N] Deny" is red, and "(Esc to cancel)" is dim
    let approve_x = char_x(r2, "[Y] Approve");
    let deny_x = char_x(r2, "[N] Deny");
    let esc_x = char_x(r2, "(Esc to cancel)");
    assert_eq!(buf[(approve_x, y2)].fg, Color::Green, "[Y] Approve green");
    assert_eq!(buf[(deny_x, y2)].fg, Color::Red, "[N] Deny red");
    assert!(
        buf[(esc_x, y2)].modifier.contains(Modifier::DIM),
        "(Esc to cancel) must be dim"
    );
}

// Scenario: Confirm pause prompt without details omits the details line
#[test]
fn confirm_pause_prompt_without_details_omits_the_details_line() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot with prompt "Bash: Run build?" and no details
    store.set_pause_state(sid("s-1"), confirm_state("Bash: Run build?", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area is 2 rows tall
    assert_eq!(input.height, 2, "confirm without details is 2 rows");

    // @step And the input area's second row shows "[Y] Approve [N] Deny (Esc to cancel)"
    let r1 = &rows[(input.y + 1) as usize];
    assert!(
        r1.contains("[Y] Approve [N] Deny (Esc to cancel)"),
        "Y/N row missing; got: {r1}"
    );
}

// Scenario: Pause prompt replaces the draft and paints no hardware cursor
#[test]
fn pause_prompt_replaces_the_draft_and_paints_no_hardware_cursor() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix"
    view.input.set_value("deploy the fix");

    // @step And session s-1 has a Triple pause slot with prompt "Read: approval" and no details
    store.set_pause_state(sid("s-1"), triple_state("Read: approval", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the rendered input area does not contain the text "deploy the fix"
    for y in input.y..input.y + input.height {
        assert!(
            !rows[y as usize].contains("deploy the fix"),
            "draft chars must not be painted while paused; row {y}: {}",
            rows[y as usize]
        );
    }

    // @step And the rendered input area contains "⏸ Read"
    assert!(
        rows[input.y as usize].contains("⏸ Read"),
        "prompt must be painted; got: {}",
        rows[input.y as usize]
    );

    // @step And the cursor is not visible while the pause prompt is showing
    assert!(
        !view.is_cursor_visible(Some(SessionStatus::Paused)),
        "hardware cursor must not be painted inside the pause prompt"
    );
}

// Scenario: Prompt renders only when the paused session is focused
#[test]
fn prompt_renders_only_when_the_paused_session_is_focused() {
    // @step Given the agent view is rendered with open sessions s-1 and s-2 and s-1 focused
    let (mut view, mut store) = view_harness(&["s-1", "s-2"]);
    assert_eq!(store.current_session(), Some(&sid("s-1")));

    // @step And session s-2 has a Triple pause slot with prompt "Read: approval" and no details
    store.set_pause_state(sid("s-2"), triple_state("Read: approval", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area shows the normal input, not a pause prompt
    for y in input.y..input.y + input.height {
        assert!(
            !rows[y as usize].contains("⏸"),
            "no pause glyph for a non-focused session's pause; row: {}",
            rows[y as usize]
        );
    }

    // @step When focus switches to session s-2 and the agent view renders a frame
    store.focus_session_index(1);
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's first row shows "⏸ Read: approval"
    assert!(
        rows[input.y as usize].contains("⏸ Read: approval"),
        "focused paused session must show its prompt; got: {}",
        rows[input.y as usize]
    );
}

// Scenario: Long triple prompt header wraps instead of clipping
#[test]
fn long_triple_prompt_header_wraps_instead_of_clipping() {
    // @step Given the agent view is rendered on a narrow terminal with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Triple pause slot whose prompt and details exceed the input-area width
    let prompt = "Read: Access to /home/user/project/secrets/.env requires approval";
    let details = "/home/user/project/secrets/.env";
    store.set_pause_state(sid("s-1"), triple_state(prompt, Some(details)));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(44, 16, &mut store, &mut view);

    // @step Then the prompt header wraps across two or more input-area rows
    assert!(
        input.height >= 3,
        "input area must grow to hold a wrapped header + options row; height = {}",
        input.height
    );
    let header_rows = input.height - 1;
    assert!(
        header_rows >= 2,
        "header must occupy 2+ rows; got {header_rows}"
    );

    // @step And no header characters are lost to clipping
    // The padded body starts at x=1 and is 42 cols wide (1-col pad each
    // side). Interior wrapped rows are full slices, so concatenating the
    // body cells of every header row and trimming the tail must
    // reproduce the full header text.
    let header_text = format!("⏸ {prompt} ({details})");
    let body: String = (input.y..input.y + header_rows)
        .map(|y| {
            let row = &rows[y as usize];
            row.chars().skip(1).take(42).collect::<String>()
        })
        .collect();
    assert_eq!(
        body.trim_end(),
        header_text,
        "wrapped header must preserve every character"
    );

    // @step And the options row "[Allow Once] [Allow Session] [Deny]" and the hint still render below the wrapped header
    let opts_row = &rows[(input.y + header_rows) as usize];
    assert!(
        opts_row.contains("[Allow Once] [Allow Session] [Deny]"),
        "options row must render below the wrapped header; got: {opts_row}"
    );
    assert!(
        opts_row.contains("(←/→"),
        "hint must start rendering after the options; got: {opts_row}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Keys — TS AgentView.tsx:4521-4607 parity, Esc DENIES
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Left and Right cycle the triple selection with wraparound
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn left_and_right_cycle_the_triple_selection_with_wraparound() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user presses Right twice
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the triple pause selection for s-1 is 2
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        2
    );

    // @step When the user presses Right once more
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the triple pause selection for s-1 is 0
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        0
    );

    // @step When the user presses Left
    app.handle_event(&key_event(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the triple pause selection for s-1 is 2
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        2
    );
}

// Scenario: Enter sends the selected approval choice and resets the selection
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_sends_the_selected_approval_choice_and_resets_the_selection() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user presses Right then Enter
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::ApproveSession)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), ApprovalChoice::ApproveSession));

    // @step And the pause slot for s-1 is cleared
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "pause slot must be cleared after Enter"
    );

    // @step And the triple pause selection for s-1 is reset to 0
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        0
    );
}

// Scenario: Esc on a triple prompt denies instead of resuming
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_a_triple_prompt_denies_instead_of_resuming() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: Access to .env requires approval", Some(".env")),
    });
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::Deny)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), ApprovalChoice::Deny));

    // @step And backend.pause_resume is NEVER called
    assert_eq!(
        mock.pause_resume_calls(),
        0,
        "Esc must DENY, never resume (security fix)"
    );

    // @step And the pause slot for s-1 is cleared
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
}

// Scenario: Y approves a confirm prompt
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn y_approves_a_confirm_prompt() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: confirm_state("Bash: Run rm -rf build?", None),
    });
    render_app(&mut app);

    // @step When the user presses the character "y"
    app.handle_event(&key_event(KeyCode::Char('y')));
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, true)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), true));
}

// Scenario: N denies a confirm prompt
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn n_denies_a_confirm_prompt() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: confirm_state("Bash: Run rm -rf build?", None),
    });
    render_app(&mut app);

    // @step When the user presses the character "n"
    app.handle_event(&key_event(KeyCode::Char('n')));
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, false)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), false));
}

// Scenario: Esc on a confirm prompt denies
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_a_confirm_prompt_denies() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: confirm_state("Bash: Run rm -rf build?", None),
    });
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, false)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    assert_eq!(mock.pause_confirm_log()[0], (sid("s-1"), false));

    // @step And backend.pause_resume is NEVER called
    assert_eq!(
        mock.pause_resume_calls(),
        0,
        "Esc must deny the confirm prompt, never resume"
    );
}

// Scenario: Printable keys are swallowed while the pause prompt is showing
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn printable_keys_are_swallowed_while_the_pause_prompt_is_showing() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And the input buffer contains the draft "hello"
    app.navigator_mut().agent.input.set_value("hello");

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user types the characters "abc"
    for c in "abc".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
        drain_pending(&mut app).await;
    }

    // @step Then the input buffer still contains exactly "hello"
    assert_eq!(
        app.navigator_mut().agent.input.value(),
        "hello",
        "printable keys must never reach the MultiLineInput while paused"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Dispatch and store — chunk-driven slot lifecycle
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Paused chunk with pause state stores a per-session slot without mounting a modal
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paused_chunk_with_pause_state_stores_a_per_session_slot_without_mounting_a_modal() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return Some Triple PauseState for s-1
    let state = triple_state("Read: Access to .env requires approval", Some(".env"));
    mock.script_pause_state(sid("s-1"), Some(state.clone()));

    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

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
        || {
            app.agent_view_store()
                .pause_state_for(&sid("s-1"))
                .is_some()
        },
        "pause slot set",
    )
    .await;

    // @step Then the AgentViewStore pause slot for s-1 holds the fetched PauseState
    let slot = app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .expect("slot set");
    assert_eq!(slot.kind, PauseKind::Triple);
    assert_eq!(slot.prompt, "Read: Access to .env requires approval");
    assert_eq!(slot.tool_call_id.as_deref(), Some(".env"));

    // @step And no compositor layer with id "pause-dialog" is mounted
    assert!(
        !pause_dialog_layer_mounted(&app),
        "the modal PauseDialog must never mount for pause states"
    );
}

// Scenario: Stale Paused chunk with no pause state sets no slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_paused_chunk_with_no_pause_state_sets_no_slot() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);

    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));

    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore pause slot for s-1 is empty
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "stale Paused chunk must not set a slot"
    );

    // @step And no compositor layer with id "pause-dialog" is mounted
    assert!(!pause_dialog_layer_mounted(&app));
}

// Scenario: HITL wins on tie and no pause slot is set
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_wins_on_tie_and_no_pause_slot_is_set() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(sid("s-1"), Some(confirm_state("p", None)));

    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    mock.script_hitl_request(sid("s-1"), Some(hitl_request("q-1", &["Yes", "No"])));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));

    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Compositor contains a layer with id HITL_DIALOG_ID
    // RPC-411: the HitlDialog modal was deleted — HITL now wins into
    // the per-session inline HITL slot (no compositor layer).
    wait_until(
        || {
            app.agent_view_store()
                .hitl_prompt_for(&sid("s-1"))
                .is_some()
        },
        "HITL slot wins on tie",
    )
    .await;

    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "HITL wins on tie — no pause slot"
    );
}

// Scenario: Running chunk clears the pause slot and resets the selection
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn running_chunk_clears_the_pause_slot_and_resets_the_selection() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot with selection 2
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    app.dispatch(Action::PausePromptNav {
        session_id: sid("s-1"),
        delta: 1,
    });
    app.dispatch(Action::PausePromptNav {
        session_id: sid("s-1"),
        delta: 1,
    });
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        2
    );

    // @step When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Running,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());

    // @step And the triple pause selection for s-1 is reset to 0
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        0
    );
}

// Scenario: Prompt keys route to the paused session in a multi-session app
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_keys_route_to_the_paused_session_in_a_multi_session_app() {
    // @step Given an App with a MockBackend and open sessions s-1 and s-2 with s-2 focused
    let (mut app, mock) = fresh_app(&["s-1", "s-2"]);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-2")));

    // @step And session s-2 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-2"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-2, ApprovalChoice::Approve)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-2"), ApprovalChoice::Approve));

    // @step And backend.pause_triple is NOT called for s-1
    assert!(!log.iter().any(|(s, _)| s == &sid("s-1")));
}

// ─────────────────────────────────────────────────────────────────────────
// Draft preservation — MultiLineInput parity (RPC-404/405 geometry)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Draft text and cursor survive the pause round-trip
#[test]
fn draft_text_and_cursor_survive_the_pause_round_trip() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    view.input.set_value("deploy the fix");
    assert_eq!(view.input.cursor(), (0, 14), "cursor parked after 'fix'");

    // @step And session s-1 has a Triple pause slot
    store.set_pause_state(sid("s-1"), triple_state("Read: approval", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);
    assert!(
        rows[input.y as usize].contains("⏸ Read"),
        "prompt painted during the pause"
    );

    // @step And the pause slot for s-1 is cleared as if the user denied
    store.clear_pause_state(&sid("s-1"));

    // @step And the agent view renders another frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input buffer contains exactly "deploy the fix"
    assert_eq!(view.input.value(), "deploy the fix");

    // @step And the cursor is at the end of "deploy the fix"
    assert_eq!(
        view.input.cursor(),
        (0, 14),
        "cursor untouched by the pause"
    );

    // @step And the rendered input area shows the draft text again
    assert!(
        rows[input.y as usize].contains("deploy the fix"),
        "draft repainted after the pause; got: {}",
        rows[input.y as usize]
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape — modal removal + Esc-deny lock
// ─────────────────────────────────────────────────────────────────────────

// Scenario: The pause modal is deleted and resume is unreachable from the prompt
#[test]
fn the_pause_modal_is_deleted_and_resume_is_unreachable_from_the_prompt() {
    // @step Given the codelet-fspec-tui crate sources
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the file codelet/fspec-tui/src/components/pause_dialog.rs does not exist
    assert!(
        !src.join("components").join("pause_dialog.rs").exists(),
        "components/pause_dialog.rs must be deleted"
    );

    // @step And components/mod.rs declares no OpenPauseDialog action variant
    let components =
        std::fs::read_to_string(src.join("components").join("mod.rs")).expect("read mod.rs");
    assert!(
        !components.contains("OpenPauseDialog"),
        "Action::OpenPauseDialog must be removed"
    );
    assert!(
        !components.contains("pub mod pause_dialog"),
        "pub mod pause_dialog must be removed"
    );

    // @step And views/agent/pause_keys.rs and app/dispatch_pause_hitl.rs never construct Action::PauseResumed from a pause-prompt key path
    let pause_keys = std::fs::read_to_string(src.join("views").join("agent").join("pause_keys.rs"))
        .expect("pause_keys.rs exists");
    assert!(
        !pause_keys.contains("PauseResumed"),
        "the pause-prompt key path must never emit Action::PauseResumed"
    );
    let dph = std::fs::read_to_string(src.join("app").join("dispatch_pause_hitl.rs"))
        .expect("dispatch_pause_hitl.rs exists");
    assert!(
        !dph.contains("send(Action::PauseResumed"),
        "dispatch_pause_hitl.rs must not construct/send Action::PauseResumed"
    );

    // @step And the files pause_prompt.rs, pause_keys.rs, input_area.rs, and store/agent_view/pause_state.rs each stay under 300 lines
    let files = [
        src.join("views").join("agent").join("pause_prompt.rs"),
        src.join("views").join("agent").join("pause_keys.rs"),
        src.join("views").join("agent").join("input_area.rs"),
        src.join("store").join("agent_view").join("pause_state.rs"),
    ];
    for f in files {
        let body =
            std::fs::read_to_string(&f).unwrap_or_else(|_| panic!("{} must exist", f.display()));
        let n = body.lines().count();
        assert!(n < 300, "{} has {n} lines, must be < 300", f.display());
    }
}
