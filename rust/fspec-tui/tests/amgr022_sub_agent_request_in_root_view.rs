//! Feature: spec/features/a-sub-agents-request-shows-in-the-root-view.feature
//!
//! AMGR-022: a sub-agent's question or approval held by the user is drawn
//! in the view of the session at the top of its chain, headed with who is
//! asking, and answered there with the request's id; every view on the
//! chain above the sub-agent shows a waiting row, and its agent-bar chip
//! the waiting marker, until the request is answered or cancelled.

use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    ApprovalChoice, HitlOption, HitlQuestion, HitlRequest, PauseKind, PauseState, PendingRequest,
    RequestHolder, RequestKind, RequestRoute, RequestStatus, SessionId, SessionState, StreamChunk,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

mod common;
use common::{MockBackend, TaskOutcome};

const WIDTH: u16 = 110;
const HEIGHT: u16 = 24;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn route(id: &str, kind: RequestKind, chain: &[&str], holder: RequestHolder) -> RequestRoute {
    RequestRoute {
        request_id: id.into(),
        origin: sid(chain[0]),
        kind,
        chain: chain.iter().map(|s| sid(s)).collect(),
        holder,
    }
}

fn question(route: RequestRoute) -> PendingRequest<HitlRequest> {
    PendingRequest {
        route,
        payload: HitlRequest {
            questions: vec![HitlQuestion {
                id: "auth_library".into(),
                header: "Auth".into(),
                question: "Which auth library?".into(),
                options: vec![
                    HitlOption {
                        label: "jose".into(),
                        description: "Use jose.".into(),
                    },
                    HitlOption {
                        label: "jsonwebtoken".into(),
                        description: "Use jsonwebtoken.".into(),
                    },
                ],
            }],
        },
    }
}

fn approval(route: RequestRoute) -> PendingRequest<PauseState> {
    PendingRequest {
        route,
        payload: PauseState {
            kind: PauseKind::Triple,
            prompt: "Bash: Command requires approval".into(),
            details: None,
            tool_call_id: None,
        },
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

async fn drain(app: &mut App) -> TestResult {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

/// Open `sessions` in order (slots #1, #2, ...), each Idle; the first is
/// a bot when `bot_root`.
async fn open(
    mock: &Arc<MockBackend>,
    sessions: &[&str],
    bot_root: bool,
) -> Result<App, codelet_test_helpers::TestError> {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for session in sessions {
        mock.script_pending_pause(sid(session), None);
        mock.script_pending_hitl(sid(session), None);
        app.dispatch(Action::SessionCreated(sid(session)));
        drain(&mut app).await?;
        state(&mut app, session, SessionState::Idle);
    }
    if bot_root {
        app.agent_view_store_mut()
            .mark_bot_session(sid(sessions[0]));
    }
    app.navigator_mut().active_view = ViewMode::Agent;
    drain(&mut app).await?;
    Ok(app)
}

fn state(app: &mut App, session: &str, state: SessionState) {
    app.dispatch(Action::ChunkReceived(
        sid(session),
        StreamChunk::SessionStateChange { state },
    ));
}

/// The event as the TUI gets it: one copy on every session of the chain.
async fn announce(app: &mut App, route: &RequestRoute, status: RequestStatus) -> TestResult {
    for member in route.chain.clone() {
        app.dispatch(Action::ChunkReceived(
            member,
            StreamChunk::InteractionRequestChanged {
                route: route.clone(),
                status: status.clone(),
            },
        ));
    }
    drain(app).await
}

/// Focus the session in `slot` (1-based) and render the agent view.
fn view(app: &mut App, slot: usize) -> Buffer {
    app.agent_view_store_mut().focus_session_index(slot - 1);
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    buf
}

fn text(buf: &Buffer) -> String {
    let area = buf.area;
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The style of the cell at the end of the first `needle` on screen.
fn last_cell_of(buf: &Buffer, needle: &str) -> Option<Color> {
    let area = buf.area;
    for y in area.y..area.y + area.height {
        let row: Vec<String> = (area.x..area.x + area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        let line: String = row.concat();
        if let Some(byte) = line.find(needle) {
            let column = line[..byte].chars().count() + needle.chars().count() - 1;
            return Some(buf[(area.x + column as u16, y)].fg);
        }
    }
    None
}

fn shows(buf: &Buffer, needle: &str) -> Result<(), String> {
    let screen = text(buf);
    if screen.contains(needle) {
        Ok(())
    } else {
        Err(format!("expected {needle:?} on screen:\n{screen}"))
    }
}

fn lacks(buf: &Buffer, needle: &str) -> Result<(), String> {
    let screen = text(buf);
    if screen.contains(needle) {
        Err(format!("did not expect {needle:?} on screen:\n{screen}"))
    } else {
        Ok(())
    }
}

/// Scenario: The root view shows a question escalated to the user, headed with who asks, and answers it
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_root_view_shows_an_escalated_question_headed_with_who_asks() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = open(&mock, &["b-1", "a-2"], true).await?;

    // @step Given bot #1 spawned sub-agent #2 and #2's question "Which auth library?" is escalated to the user
    let held_by_bot = route(
        "req-q-1",
        RequestKind::Question,
        &["a-2", "b-1"],
        RequestHolder::Session(sid("b-1")),
    );
    announce(&mut app, &held_by_bot, RequestStatus::Pending).await?;
    let escalated = route(
        "req-q-1",
        RequestKind::Question,
        &["a-2", "b-1"],
        RequestHolder::User,
    );
    mock.script_pending_hitl(sid("a-2"), Some(question(escalated.clone())));
    announce(&mut app, &escalated, RequestStatus::Pending).await?;

    // @step When the user looks at bot #1's view
    let buf = view(&mut app, 1);

    // @step Then it shows "sub-agent #2 of bot #1 asks" above the question
    shows(&buf, "sub-agent #2 of bot #1 asks")?;
    shows(&buf, "Which auth library?")?;
    let screen = text(&buf);
    let header = screen
        .find("sub-agent #2 of bot #1 asks")
        .ok_or("no header")?;
    let asked = screen.find("Which auth library?").ok_or("no question")?;
    assert!(
        header < asked,
        "the header is above the question:\n{screen}"
    );

    // @step When the user picks jose there
    app.handle_event(&key(KeyCode::Enter));
    drain(&mut app).await?;

    // @step Then send_hitl_response goes to sub-agent #2 naming the request id
    assert_eq!(
        mock.reply_request_ids(),
        vec![("send_hitl_response", sid("a-2"), "req-q-1".to_string())]
    );
    Ok(())
}

/// Scenario: A view whose session holds a sub-agent's approval shows a waiting row and its chip a waiting marker
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_holder_view_shows_a_waiting_row_and_its_chip_a_marker() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = open(&mock, &["a-1", "a-2"], false).await?;

    // @step Given agent #1 holds sub-agent #2's approval
    let held = route(
        "req-p-1",
        RequestKind::Approval,
        &["a-2", "a-1"],
        RequestHolder::Session(sid("a-1")),
    );
    announce(&mut app, &held, RequestStatus::Pending).await?;

    // @step When the user looks at agent #1's view
    let buf = view(&mut app, 1);

    // @step Then it shows "Waiting: sub-agent #2 asks for approval; agent #1 holds it" and no prompt
    shows(
        &buf,
        "Waiting: sub-agent #2 asks for approval; agent #1 holds it",
    )?;
    lacks(&buf, "Allow Once")?;
    assert_eq!(
        mock.get_pause_state_calls(),
        0,
        "a session-held request is not read for the user"
    );

    // @step And agent #1's chip reads "#1 ●?" with the marker in yellow
    let other_view = view(&mut app, 2);
    shows(&other_view, "#1 ●?")?;
    assert_eq!(last_cell_of(&other_view, "#1 ●?"), Some(Color::Yellow));
    lacks(&other_view, "#2 ●?")?;
    Ok(())
}

/// Scenario: An approval passed up a three-level chain to the user shows in the root view and is answered there
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_approval_passed_up_three_levels_is_answered_in_the_root_view() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = open(&mock, &["b-1", "a-2", "a-3"], true).await?;

    // @step Given bot #1 spawned sub-agent #2, #2 spawned sub-agent #3, and #3's approval is passed up to the user
    let chain = ["a-3", "a-2", "b-1"];
    let held = route(
        "req-p-3",
        RequestKind::Approval,
        &chain,
        RequestHolder::Session(sid("a-2")),
    );
    announce(&mut app, &held, RequestStatus::Pending).await?;
    let passed_up = route(
        "req-p-3",
        RequestKind::Approval,
        &chain,
        RequestHolder::User,
    );
    mock.script_pending_pause(sid("a-3"), Some(approval(passed_up.clone())));
    announce(&mut app, &passed_up, RequestStatus::Pending).await?;

    // @step When the user looks at bot #1's view and at #2's view
    let middle = view(&mut app, 2);
    let root = view(&mut app, 1);

    // @step Then bot #1's view shows "sub-agent #3 of sub-agent #2 of bot #1 asks" above the approval
    shows(&root, "sub-agent #3 of sub-agent #2 of bot #1 asks")?;
    shows(&root, "Allow Once")?;

    // @step And #2's view shows "Waiting: sub-agent #3 asks for approval; the user holds it"
    shows(
        &middle,
        "Waiting: sub-agent #3 asks for approval; the user holds it",
    )?;

    // @step When the user presses Enter in bot #1's view
    app.handle_event(&key(KeyCode::Enter));
    drain(&mut app).await?;

    // @step Then pause_triple approve goes to sub-agent #3 naming the request id
    assert_eq!(
        mock.pause_triple_log(),
        vec![(sid("a-3"), ApprovalChoice::Approve)]
    );
    assert_eq!(
        mock.reply_request_ids(),
        vec![("pause_triple", sid("a-3"), "req-p-3".to_string())]
    );
    Ok(())
}

/// Scenario: An answered request leaves no prompt, waiting row or marker
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_answered_request_leaves_no_prompt_row_or_marker() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = open(&mock, &["a-1", "a-2"], false).await?;

    // @step Given agent #1's view shows a waiting row for sub-agent #2's request
    let held = route(
        "req-p-4",
        RequestKind::Approval,
        &["a-2", "a-1"],
        RequestHolder::Session(sid("a-1")),
    );
    mock.script_pending_pause(sid("a-2"), Some(approval(held.clone())));
    announce(&mut app, &held, RequestStatus::Pending).await?;
    state(&mut app, "a-2", SessionState::Paused);
    drain(&mut app).await?;
    shows(&view(&mut app, 2), "Allow Once")?;
    shows(
        &view(&mut app, 1),
        "Waiting: sub-agent #2 asks for approval; agent #1 holds it",
    )?;

    // @step When the Answered event for that request arrives
    let by = RequestStatus::Answered {
        by: RequestHolder::Session(sid("a-1")),
    };
    announce(&mut app, &held, by).await?;

    // @step Then no view shows a prompt or a waiting row for it and no chip shows the waiting marker
    let holder_view = view(&mut app, 1);
    lacks(&holder_view, "Waiting:")?;
    let origin_view = view(&mut app, 2);
    lacks(&origin_view, "Allow Once")?;
    lacks(&origin_view, "●?")?;
    Ok(())
}

/// Scenario: The user answers an escalated question from the sub-agent's own tab
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_user_answers_from_the_sub_agents_tab() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = open(&mock, &["b-1", "a-2"], true).await?;

    // @step Given sub-agent #2's question is escalated to the user and bot #1's view shows it
    let escalated = route(
        "req-q-5",
        RequestKind::Question,
        &["a-2", "b-1"],
        RequestHolder::User,
    );
    mock.script_pending_hitl(sid("a-2"), Some(question(escalated.clone())));
    announce(&mut app, &escalated, RequestStatus::Pending).await?;
    shows(&view(&mut app, 1), "sub-agent #2 of bot #1 asks")?;

    // @step When the user answers it from #2's own tab
    let own_tab = view(&mut app, 2);
    shows(&own_tab, "Which auth library?")?;
    lacks(&own_tab, "sub-agent #2 of bot #1 asks")?;
    app.handle_event(&key(KeyCode::Enter));
    drain(&mut app).await?;

    // @step Then the reply goes to sub-agent #2 naming the request id
    assert_eq!(
        mock.reply_request_ids(),
        vec![("send_hitl_response", sid("a-2"), "req-q-5".to_string())]
    );

    // @step And once the Answered event arrives bot #1's view no longer shows it
    announce(
        &mut app,
        &escalated,
        RequestStatus::Answered {
            by: RequestHolder::User,
        },
    )
    .await?;
    let root = view(&mut app, 1);
    lacks(&root, "sub-agent #2")?;
    lacks(&root, "Which auth library?")?;
    Ok(())
}

/// Scenario: The root's own approval is shown before a relayed question
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_roots_own_approval_comes_before_a_relayed_question() -> TestResult {
    let mock = Arc::new(MockBackend::new());
    let mut app = open(&mock, &["b-1", "a-2"], true).await?;

    // @step Given bot #1 has its own approval pending and sub-agent #2's question is held by the user
    let own = route(
        "req-p-6",
        RequestKind::Approval,
        &["b-1"],
        RequestHolder::User,
    );
    mock.script_pending_pause(sid("b-1"), Some(approval(own.clone())));
    announce(&mut app, &own, RequestStatus::Pending).await?;
    state(&mut app, "b-1", SessionState::Paused);
    let relayed = route(
        "req-q-6",
        RequestKind::Question,
        &["a-2", "b-1"],
        RequestHolder::User,
    );
    mock.script_pending_hitl(sid("a-2"), Some(question(relayed.clone())));
    announce(&mut app, &relayed, RequestStatus::Pending).await?;

    // @step When the user looks at bot #1's view
    let root = view(&mut app, 1);

    // @step Then it shows bot #1's own approval
    shows(&root, "Allow Once")?;
    lacks(&root, "Which auth library?")?;

    // @step When the user answers that approval
    app.handle_event(&key(KeyCode::Enter));
    drain(&mut app).await?;
    assert_eq!(
        mock.pause_triple_log(),
        vec![(sid("b-1"), ApprovalChoice::Approve)]
    );

    // @step Then bot #1's view shows sub-agent #2's question headed "sub-agent #2 of bot #1 asks"
    let root = view(&mut app, 1);
    shows(&root, "sub-agent #2 of bot #1 asks")?;
    shows(&root, "Which auth library?")?;
    Ok(())
}
