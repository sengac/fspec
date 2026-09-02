#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/goal-state-back-sync.feature
//!
//! CONT-008: goal state back-sync to chrome. Engine-side goal transitions
//! (satisfied via accepted done()) must propagate OUT of the inner CLI
//! Session: the shared teardown emits a dedicated goal-cleared signal
//! (`ContinueStateReason::GoalSatisfied` → wire `goalCleared: true`), the
//! BackgroundOutput twins write the chrome `BackgroundSession.goal_state`
//! back through ONE shared helper, a generation stamp makes the chrome→inner
//! dispatch sync direction-safe (no satisfied-goal resurrection), and the
//! TUI drops its 🎯 cache + renders REAL counters for bare `/goal`.
//!
//! Test surfaces (cont007/cont009 precedent):
//! - behavioral tests on the REAL production helpers
//!   (`apply_finish_with_summary`, agent-loop `BackgroundOutput`, the
//!   shared `sync_completion_contract_for_user_turn`) against a real
//!   `BackgroundSession` (SessionManager fixture, Noop hooks),
//! - App + MockBackend chunk-dispatch tests asserting the chrome goal
//!   cache, `paint_footer` buffer output and bare `/goal` scrollback,
//! - serde round-trip + backward-compat defaults for the widened
//!   `ContinueStateInfo`,
//! - source-shape pins (rpc082/083 precedent) for the napi twin mapping,
//!   both JSON serializers and the escalation arm's no-emission rule.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::done_early_exit::apply_finish_with_summary;
use codelet_cli::interactive::goal::{build_goal_blocked_message, raise_goal_escalation};
use codelet_cli::interactive::output::{
    ContinueStateEvent, ContinueStateReason, StreamEvent, StreamOutput,
};
use codelet_cli::session::system_reminders::count_system_reminders_by_type;
use codelet_cli::session::{Session, SystemReminderType};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_fspec_tui::views::agent::chrome_paint::{paint_footer, ChromeAreas};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{ContinueStateInfo, SessionId, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use codelet_tools::tool_pause::{set_pause_handler, PauseHandler, PauseResponse};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// PROV-132/cont009 precedent: serialize tests that swap the process-global
/// data directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Trimmed offline models.dev catalog (anthropic/openai/google), shared with
/// the sessions-crate PROV-101 / cont009 tests. Seeding it into the temp data
/// dir's cache keeps registry validation fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge
/// (Noop hooks — no agent loop spawned). Mirrors
/// rust/sessions/tests/cont009_completion_contract_sync.rs.
async fn fresh_background_session() -> (
    tempfile::TempDir,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
) {
    let data_dir = tempfile::tempdir().expect("tempdir");
    // Hermetic: seed the offline model cache + fake anthropic key so
    // `create_session` works without ambient credentials or network.
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models.json");
    // RPC-423 precedent (cont009): reset stores BEFORE setting data
    // directory so init_session_store() points at the fresh temp dir.
    codelet_core::persistence::reset_stores_for_tests();
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    std::env::set_var("ANTHROPIC_API_KEY", "cont008-fake-key");
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist after create_session");
    (data_dir, manager, session)
}

/// Recording StreamOutput — callback event sink (vi.fn()-style, no module
/// replacement). cont007 precedent.
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn events(&self) -> Vec<StreamEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Build a CLI Session without depending on a single-credential environment
/// (cont006/cont007 precedent).
fn fresh_cli_session() -> Session {
    for name in ["claude", "openai", "gemini", "codex", "zai"] {
        if let Ok(pm) = codelet_providers::ProviderManager::with_provider(name) {
            return Session::from_provider_manager(pm);
        }
    }
    Session::new(None).expect("failed to create test session")
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

/// Build a live counter snapshot ContinueStateEvent for direct emission
/// through a BackgroundOutput twin.
fn snapshot(reason: ContinueStateReason, goal_active: bool) -> ContinueStateEvent {
    ContinueStateEvent {
        enabled: false,
        budget: 10,
        nudges_used: 0,
        goal_active,
        effective_budget: if goal_active { 15 } else { 10 },
        done_rejections: 0,
        reason,
    }
}

/// Paint the SessionFooter into a 60x1 buffer and return the collapsed row
/// text (cont007 precedent — double-width glyphs leave a blank cell).
fn painted_footer(app: &App, session: &SessionId) -> String {
    let areas = ChromeAreas {
        header: Rect::new(0, 0, 0, 0),
        role: Rect::new(0, 0, 0, 0),
        scrollback: Rect::new(0, 0, 0, 0),
        footer: Rect::new(0, 0, 60, 1),
        input: Rect::new(0, 0, 0, 0),
    };
    let mut buf = Buffer::empty(areas.footer);
    paint_footer(&areas, &mut buf, app.agent_view_store(), Some(session));
    let mut text = String::new();
    for x in 0..60u16 {
        text.push_str(buf[(x, 0)].symbol());
    }
    let mut collapsed = String::new();
    let mut prev_space = false;
    for c in text.chars() {
        if c == ' ' {
            if !prev_space {
                collapsed.push(c);
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    collapsed
}

/// Last scrollback chunk text for a session (bare /goal output assertions).
fn last_scrollback_text(app: &App, session: &SessionId) -> String {
    let ctx = app
        .agent_view_store()
        .session_context_for(session)
        .expect("session context must exist");
    ctx.scrollback
        .chunks()
        .last()
        .and_then(|c| c.source.as_ref())
        .map(|s| s.text.clone())
        .unwrap_or_default()
}

/// Strip `//` line comments so source-shape pins cannot false-positive on
/// commentary (cont007/cont009 precedent).
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut in_str = false;
            let bytes = line.as_bytes();
            let mut i = 0;
            while i + 1 < bytes.len() {
                if bytes[i] == b'"' {
                    in_str = !in_str;
                }
                if !in_str && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                    return &line[..i];
                }
                i += 1;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_source(rel: &str) -> String {
    let path = format!("{}/../{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

// ============================================================================
// Scenario: Goal-satisfied teardown emits the dedicated goal-cleared snapshot
// ============================================================================

#[test]
fn goal_satisfied_teardown_emits_the_dedicated_goal_cleared_snapshot() {
    // @step Given a session with an active goal and 4 nudges used
    let session_id = uuid::Uuid::new_v4();
    let mut session = fresh_cli_session();
    session.continue_enabled = true;
    session.continue_budget = 10;
    session.continue_nudges_used = 4;
    session.set_goal("make tests pass", None);
    let output = RecordingOutput::new();

    // @step When the shared teardown runs for an accepted done() summary
    apply_finish_with_summary(&mut session, session_id, "all tests pass", &output);

    // @step Then the satisfied goal is announced before the counter snapshot
    let events = output.events();
    let status_pos = events
        .iter()
        .position(
            |e| matches!(e, StreamEvent::Status(s) if s.starts_with("\u{1F3AF} goal satisfied:")),
        )
        .expect("goal teardown must announce the satisfied goal");
    let state_pos = events
        .iter()
        .position(|e| matches!(e, StreamEvent::ContinueState(_)))
        .expect("teardown must emit a ContinueState event");
    assert!(
        state_pos > status_pos,
        "ContinueState must be emitted AFTER the announcement"
    );

    // @step And the snapshot carries the goal-satisfied reason with goalActive false and nudgesUsed 0
    match &events[state_pos] {
        StreamEvent::ContinueState(cs) => {
            assert_eq!(
                cs.reason,
                ContinueStateReason::GoalSatisfied,
                "goal teardown must mark the snapshot with the DEDICATED goal-cleared reason"
            );
            assert!(!cs.goal_active, "goal must be cleared before the emission");
            assert_eq!(cs.nudges_used, 0, "teardown must reset the nudge counter");
        }
        _ => unreachable!(),
    }

    // @step And running the teardown without a goal emits the done-accepted reason instead
    let mut plain = fresh_cli_session();
    plain.continue_enabled = true;
    plain.continue_nudges_used = 2;
    let plain_output = RecordingOutput::new();
    apply_finish_with_summary(&mut plain, uuid::Uuid::new_v4(), "done", &plain_output);
    assert!(
        plain_output.events().iter().any(|e| matches!(
            e,
            StreamEvent::ContinueState(cs)
                if cs.reason == ContinueStateReason::DoneAccepted && cs.nudges_used == 0
        )),
        "non-goal teardown must keep the DoneAccepted reason (goalCleared false on the wire)"
    );
}

// ============================================================================
// Scenario: Goal-satisfied snapshot writes the chrome goal state back through
// the background output
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn goal_satisfied_snapshot_writes_the_chrome_goal_state_back() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await;
    session.set_goal_state(Some(("ship the fix".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }
    let output = BackgroundOutput::with_provider(session.clone(), "test".to_string());
    let mut rx = session.subscribe_to_stream();

    // @step When the background output maps a goal-satisfied counter snapshot
    output.emit(StreamEvent::ContinueState(snapshot(
        ContinueStateReason::GoalSatisfied,
        false,
    )));

    // @step Then the chrome goal state reads as no goal
    assert!(
        session.get_goal_state().is_none(),
        "GoalSatisfied must write the chrome goal state back to None"
    );

    // @step And the pushed chunk carries goalCleared true
    let chunk = rx.try_recv().expect("a chunk must have been pushed");
    match chunk {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(
                continue_state.goal_cleared,
                "the wire chunk must carry goalCleared true for the TUI cache drop"
            );
            assert!(!continue_state.goal_active);
        }
        other => panic!("expected ContinueStateUpdate, got {other:?}"),
    }

    // @step And mapping a non-goal done-accepted snapshot leaves a synced chrome goal untouched
    session.set_goal_state(Some(("second goal".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }
    output.emit(StreamEvent::ContinueState(snapshot(
        ContinueStateReason::DoneAccepted,
        false,
    )));
    assert_eq!(
        session.get_goal_state(),
        Some(("second goal".to_string(), None)),
        "a DoneAccepted (goalCleared false) snapshot must NEVER wipe the chrome goal"
    );
}

// ============================================================================
// Scenario: A goal replaced mid-turn survives the goal-satisfied write-back
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_goal_replaced_mid_turn_survives_the_goal_satisfied_write_back() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await;
    session.set_goal_state(Some(("goal A".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step And the user has since replaced the chrome goal mid-turn
    session.set_goal_state(Some(("goal B".to_string(), Some("true".to_string()))));

    // @step When the background output maps a goal-satisfied counter snapshot
    let output = BackgroundOutput::with_provider(session.clone(), "test".to_string());
    output.emit(StreamEvent::ContinueState(snapshot(
        ContinueStateReason::GoalSatisfied,
        false,
    )));

    // @step Then the chrome goal state still holds the replacement goal
    assert_eq!(
        session.get_goal_state(),
        Some(("goal B".to_string(), Some("true".to_string()))),
        "a mid-turn replacement (generation moved) must WIN over the write-back"
    );

    // @step And the next dispatch sync applies the replacement goal to the inner session
    let mut inner = session.inner.lock().await;
    session.sync_completion_contract_for_user_turn(&mut inner);
    let goal = inner.goal.as_ref().expect("inner goal must be set");
    assert_eq!(goal.text, "goal B");
    assert_eq!(goal.verify.as_deref(), Some("true"));
}

// ============================================================================
// Scenario: A satisfied goal is not resurrected on the next dispatched user
// message
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_satisfied_goal_is_not_resurrected_on_the_next_dispatched_user_message() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await;
    session.set_goal_state(Some(("make tests pass".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }

    // @step And the engine accepted done() for the goal and the background output performed the write-back
    let output = BackgroundOutput::with_provider(session.clone(), "test".to_string());
    {
        let mut inner = session.inner.lock().await;
        // The REAL teardown against the REAL twin: apply_finish_with_summary
        // emits the GoalSatisfied snapshot through BackgroundOutput, which
        // performs the chrome write-back.
        apply_finish_with_summary(&mut inner, session.id, "all tests pass", &output);
        assert!(inner.goal.is_none(), "teardown must clear the inner goal");
    }
    assert!(
        session.get_goal_state().is_none(),
        "write-back must clear the chrome goal state"
    );

    // @step When the dispatch-site sync helper runs for the next real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session has no goal
    let inner = session.inner.lock().await;
    assert!(
        inner.goal.is_none(),
        "RESURRECTION: the dispatch sync must not re-apply the satisfied goal"
    );

    // @step And the done() registry reports the session as disarmed with no goal spec
    assert!(
        !codelet_tools::is_continue_armed(session.id),
        "registry must stay disarmed after the satisfied goal"
    );
    assert!(
        codelet_tools::get_session_goal(session.id).is_none(),
        "registry must hold no goal spec after the satisfied goal"
    );

    // @step And no CompletionContract reminder is re-injected into the inner session messages
    assert_eq!(
        count_system_reminders_by_type(&inner.messages, SystemReminderType::CompletionContract),
        0,
        "no CompletionContract reminder may be re-injected for a satisfied goal"
    );
}

// ============================================================================
// Scenario: The dispatch sync never re-applies a chrome goal the engine
// already consumed
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_dispatch_sync_never_reapplies_a_chrome_goal_the_engine_already_consumed() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await;
    session.set_goal_state(Some(("stale goal".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }

    // @step And the engine cleared the inner goal without the chrome write-back landing
    {
        let mut inner = session.inner.lock().await;
        inner.clear_goal();
        codelet_tools::set_session_goal(session.id, None);
    }

    // @step When the dispatch-site sync helper runs for the next real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session still has no goal
    {
        let inner = session.inner.lock().await;
        assert!(
            inner.goal.is_none(),
            "defense-in-depth: the guard must hold even when the push channel is lost \
             (chrome generation unchanged since the last sync)"
        );
        assert!(
            !codelet_tools::is_continue_armed(session.id),
            "registry must not be re-armed by the stale chrome goal"
        );
    }

    // @step And setting a new chrome goal afterwards is applied by the following dispatch sync
    session.set_goal_state(Some(("fresh goal".to_string(), None)));
    let mut inner = session.inner.lock().await;
    session.sync_completion_contract_for_user_turn(&mut inner);
    assert_eq!(
        inner.goal.as_ref().map(|g| g.text.as_str()),
        Some("fresh goal"),
        "a genuinely NEW user /goal (generation bumped) must be applied"
    );
    assert!(
        codelet_tools::is_continue_armed(session.id),
        "the new goal must arm the registry"
    );
}

// ============================================================================
// Scenario: The TUI drops the goal indicator when the engine clears the goal
// ============================================================================

#[test]
fn the_tui_drops_the_goal_indicator_when_the_engine_clears_the_goal() {
    // @step Given a TUI session whose footer shows the goal indicator from a live counter snapshot
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::InputSubmitted("/continue 10".to_string()));
    app.dispatch(Action::InputSubmitted("/goal ship it".to_string()));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 1,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));
    let before = painted_footer(&app, &sid("s-1"));
    assert!(
        before.contains("\u{1F3AF} goal (1/15)"),
        "precondition: footer must show the goal indicator; painted: {before:?}"
    );

    // @step When a counter snapshot chunk with goalCleared true is dispatched for the session
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 0,
            goal_active: false,
            effective_budget: 10,
            goal_cleared: true,
            done_rejections: 0,
        }),
    ));

    // @step Then the cached goal state for the session is cleared
    assert!(
        app.agent_view_store().goal_state_for(&sid("s-1")).is_none(),
        "goalCleared must clear the TUI goal cache"
    );

    // @step And the painted footer no longer shows the goal indicator
    let after = painted_footer(&app, &sid("s-1"));
    assert!(
        !after.contains("\u{1F3AF}"),
        "footer must drop the goal indicator; painted: {after:?}"
    );
    assert!(
        after.contains("\u{23E9} auto-continue (0/10)"),
        "with the toggle on the footer falls back to the auto-continue indicator; painted: {after:?}"
    );

    // @step And bare /goal reports that no goal is set
    app.dispatch(Action::InputSubmitted("/goal".to_string()));
    let text = last_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.contains("no goal set"),
        "bare /goal after the engine clear must report no goal; got: {text:?}"
    );
}

// ============================================================================
// Scenario: Bare /goal reports the live nudge and rejection counters
// ============================================================================

#[test]
fn bare_goal_reports_the_live_nudge_and_rejection_counters() {
    // @step Given a TUI session with an active goal whose live snapshot reports 3 nudges used and 2 rejections
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::InputSubmitted("/goal ship it".to_string()));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: false,
            budget: 10,
            nudges_used: 3,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 2,
        }),
    ));

    // @step When the user enters "/goal"
    app.dispatch(Action::InputSubmitted("/goal".to_string()));

    // @step Then the state output shows nudges used 3 and rejections 2
    let text = last_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.contains("nudges used: 3, rejections: 2"),
        "bare /goal must show the LIVE counters; got: {text:?}"
    );
    assert!(
        text.contains("budget: 15"),
        "bare /goal must show the effective goal budget; got: {text:?}"
    );

    // @step And the state output no longer hard-codes zero counters
    assert!(
        !text.contains("nudges used: 0, rejections: 0"),
        "the hard-coded zeros must be gone from the Show display; got: {text:?}"
    );
}

// ============================================================================
// Scenario: Escalation keeps the goal indicator on the bar
// ============================================================================

#[test]
fn escalation_keeps_the_goal_indicator_on_the_bar() {
    // @step Given a TUI session whose footer shows the goal indicator from a live counter snapshot
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::InputSubmitted("/goal ship it".to_string()));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: false,
            budget: 10,
            nudges_used: 5,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 4,
        }),
    ));

    // @step When the engine raises a goal escalation through a registered pause handler
    // Drive the REAL escalation path the Escalate arm runs (status line
    // first, then the per-session HITL pause) with a stub pause handler
    // standing in for the TUI/NAPI surface (tool_pause registry).
    let engine_session_id = uuid::Uuid::new_v4();
    let mut engine_session = fresh_cli_session();
    engine_session.set_goal("ship it", None);
    codelet_tools::set_session_goal(
        engine_session_id,
        Some(codelet_tools::GoalSpec {
            text: "ship it".to_string(),
            verify: None,
        }),
    );
    let sink = RecordingOutput::new();
    let handler_invoked = Arc::new(AtomicBool::new(false));
    let handler_flag = Arc::clone(&handler_invoked);
    let handler: PauseHandler = Arc::new(move |request| {
        assert_eq!(
            request.tool_name, "goal",
            "escalation must pause under the goal tool name"
        );
        handler_flag.store(true, Ordering::SeqCst);
        PauseResponse::Resumed
    });
    set_pause_handler(engine_session_id, Some(handler));
    let message = build_goal_blocked_message();
    sink.emit_status(&message);
    let resolution = raise_goal_escalation(engine_session_id, &message);
    set_pause_handler(engine_session_id, None);

    // @step Then the painted footer still shows the goal indicator
    let text = painted_footer(&app, &sid("s-1"));
    assert!(
        text.contains("\u{1F3AF} goal (5/15)"),
        "escalation leaves the goal active — the bar must keep showing it; painted: {text:?}"
    );

    // @step And the escalation path emits no goal-cleared signal in the engine sources
    let stream_loop = strip_line_comments(&read_source("cli/src/interactive/stream_loop.rs"));
    let arm_start = stream_loop
        .find("ContinueDecision::Escalate")
        .expect("stream_loop.rs must have the Escalate arm");
    let arm_end = stream_loop[arm_start..]
        .find("ContinueDecision::Nudge")
        .map(|i| arm_start + i)
        .expect("the Nudge arm must follow the Escalate arm");
    let escalate_arm = &stream_loop[arm_start..arm_end];
    assert!(
        !escalate_arm.contains("emit_continue_state") && !escalate_arm.contains("GoalSatisfied"),
        "the Escalate arm must not emit a goal-cleared (or any counter) snapshot — \
         the goal stays active by design"
    );

    // @step And the pause handler is invoked and no continue-state event reaches the output sink
    assert!(
        handler_invoked.load(Ordering::SeqCst),
        "raise_goal_escalation must dispatch through the registered pause handler"
    );
    assert_eq!(
        resolution,
        PauseResponse::Resumed,
        "the stub handler resumes the escalation"
    );
    let sink_events = sink.events();
    assert!(
        sink_events
            .iter()
            .any(|e| matches!(e, StreamEvent::Status(s) if s == &message)),
        "the blocked status line stays in chat (terminal event)"
    );
    assert!(
        !sink_events
            .iter()
            .any(|e| matches!(e, StreamEvent::ContinueState(_))),
        "escalation must push no ContinueState (or goal-cleared) event to the sink"
    );

    // @step And the inner session goal remains set after the escalation
    assert!(
        engine_session.goal.is_some(),
        "escalation leaves the goal armed on the inner session"
    );
    assert_eq!(
        codelet_tools::get_session_goal(engine_session_id).map(|g| g.text),
        Some("ship it".to_string()),
        "escalation leaves the goal armed in the done() registry"
    );
    codelet_tools::set_session_goal(engine_session_id, None);
}

// ============================================================================
// Scenario: Wire chunk and twin serializers carry the goal-cleared flag and
// rejection count
// ============================================================================

#[test]
fn wire_chunk_and_twin_serializers_carry_the_goal_cleared_flag_and_rejection_count() {
    // @step Given a ContinueStateUpdate chunk with goalCleared true and doneRejections 2
    let chunk = StreamChunk::continue_state_update(ContinueStateInfo {
        enabled: true,
        budget: 10,
        nudges_used: 0,
        goal_active: false,
        effective_budget: 10,
        goal_cleared: true,
        done_rejections: 2,
    });

    // @step When the chunk is serialized to JSON and deserialized back
    let json = serde_json::to_string(&chunk).expect("serialize ContinueStateUpdate");
    let back: StreamChunk = serde_json::from_str(&json).expect("deserialize ContinueStateUpdate");

    // @step Then the JSON payload uses the camelCase field names goalCleared and doneRejections
    assert!(
        json.contains("goalCleared") && json.contains("doneRejections"),
        "wire JSON must be camelCase; got: {json}"
    );
    match back {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(continue_state.goal_cleared);
            assert_eq!(continue_state.done_rejections, 2);
        }
        other => panic!("expected ContinueStateUpdate after round-trip, got {other:?}"),
    }

    // @step And a payload without the new fields still deserializes with false and zero defaults
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse to Value");
    let info = value
        .pointer_mut("/ContinueStateUpdate/continueState")
        .expect("externally-tagged payload must carry the continueState object")
        .as_object_mut()
        .expect("continueState must be an object");
    info.remove("goalCleared");
    info.remove("doneRejections");
    let legacy_json = serde_json::to_string(&value).expect("re-serialize legacy payload");
    let legacy: StreamChunk =
        serde_json::from_str(&legacy_json).expect("legacy payload must still deserialize");
    match legacy {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(
                !continue_state.goal_cleared,
                "missing goalCleared must default to false"
            );
            assert_eq!(
                continue_state.done_rejections, 0,
                "missing doneRejections must default to 0"
            );
        }
        other => panic!("expected ContinueStateUpdate from legacy payload, got {other:?}"),
    }

    // @step And both background twins and both twin JSON serializers carry the new fields and the shared write-back call
    let agent_loop_twin = strip_line_comments(&read_source("agent-loop/src/background_output.rs"));
    let napi_twin = strip_line_comments(&read_source("napi/src/agent_loop.rs"));
    for (name, twin) in [("agent-loop", &agent_loop_twin), ("napi", &napi_twin)] {
        assert!(
            twin.contains("clear_goal_state_if_unchanged_since_sync"),
            "{name} twin must call the SHARED BackgroundSession write-back helper \
             (CONT-009 twin-parity rule)"
        );
        assert!(
            twin.contains("goal_cleared") && twin.contains("done_rejections"),
            "{name} twin must map the new fields into ContinueStateInfo"
        );
    }
    let agent_loop_json = strip_line_comments(&read_source("agent-loop/src/stream_chunk_json.rs"));
    let napi_json = strip_line_comments(&read_source("napi/src/types.rs"));
    for (name, json_src) in [("agent-loop", &agent_loop_json), ("napi", &napi_json)] {
        assert!(
            json_src.contains("goalCleared") && json_src.contains("doneRejections"),
            "{name} stream_chunk_to_json_value must serialize the new fields"
        );
    }
}
