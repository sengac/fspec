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

use codelet_test_helpers::OrFail;
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
async fn fresh_background_session() -> Result<
    (
        std::path::PathBuf,
        Arc<SessionManager>,
        Arc<BackgroundSession>,
    ),
    codelet_test_helpers::TestError,
> {
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().or_fail("shared temp subdirectory")?;
    // Hermetic: seed the offline model cache + fake anthropic key so
    // `create_session` works without ambient credentials or network.
    let cache_dir = data_dir.as_path().join("cache");
    std::fs::create_dir_all(&cache_dir).or_fail("create cache dir")?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).or_fail("write models.json")?;
    // RPC-423 precedent (cont009): reset stores BEFORE setting data
    // directory so init_session_store() points at the fresh temp dir.
    codelet_core::persistence::reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.clone()).or_fail("set the data directory")?;
    std::env::set_var("ANTHROPIC_API_KEY", "cont008-fake-key");
    let manager = SessionManager::new();
    // BUG-437: sessions are created in the manager's start workspace.
    manager
        .set_default_project(data_dir.clone())
        .or_fail("set the start workspace")?;
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle
        .create_session_checked(None)
        .or_fail("the handle creates the session")?;
    let session = manager
        .get_session(&sid.value)
        .or_fail("session must exist after create_session")?;
    Ok((data_dir, manager, session))
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

    fn events(&self) -> Result<Vec<StreamEvent>, codelet_test_helpers::TestError> {
        Ok(self.events.lock().or_fail("expected a value")?.clone())
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        // No error channel here: a poisoned recorder is logged, and the
        // test's assertions on the recorded events then fail.
        match self.events.lock() {
            Ok(mut events) => events.push(event),
            Err(error) => tracing::error!("recording output: events mutex poisoned: {error}"),
        }
    }
}

/// A CLI session for an explicitly selected provider that needs no
/// credentials, whatever this machine has credentialed (BUG-548).
fn fresh_cli_session() -> Result<Session, codelet_test_helpers::TestError> {
    Ok(Session::from_provider_manager(
        codelet_providers::ProviderManager::for_testing(
            codelet_providers::ProviderType::Claude,
            None,
            None,
        ),
    ))
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
        menu: Rect::new(0, 0, 0, 0),
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
fn last_scrollback_text(
    app: &App,
    session: &SessionId,
) -> Result<String, codelet_test_helpers::TestError> {
    let ctx = app
        .agent_view_store()
        .session_context_for(session)
        .or_fail("session context must exist")?;
    Ok(ctx
        .scrollback
        .chunks()
        .last()
        .and_then(|c| c.source.as_ref())
        .map(|s| s.text.clone())
        .unwrap_or_default())
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

fn read_source(rel: &str) -> Result<String, codelet_test_helpers::TestError> {
    let path = format!("{}/../{}", env!("CARGO_MANIFEST_DIR"), rel);
    Ok(
        // Follows file-backed `mod` declarations so split modules (e.g.
        // stream_loop.rs + stream_loop/) are read as one source.
        codelet_test_helpers::source_composition::read_tree_or_file(&path)
            .map_err(|e| format!("failed to read {path}: {e}"))?,
    )
}

// ============================================================================
// Scenario: Goal-satisfied teardown emits the dedicated goal-cleared snapshot
// ============================================================================

// ============================================================================
// Scenario: Goal-satisfied snapshot writes the chrome goal state back through
// the background output
// ============================================================================

// ============================================================================
// Scenario: A goal replaced mid-turn survives the goal-satisfied write-back
// ============================================================================

// ============================================================================
// Scenario: A satisfied goal is not resurrected on the next dispatched user
// message
// ============================================================================

// ============================================================================
// Scenario: The dispatch sync never re-applies a chrome goal the engine
// already consumed
// ============================================================================

// ============================================================================
// Scenario: The TUI drops the goal indicator when the engine clears the goal
// ============================================================================

// ============================================================================
// Scenario: Bare /goal reports the live nudge and rejection counters
// ============================================================================

// ============================================================================
// Scenario: Escalation keeps the goal indicator on the bar
// ============================================================================

// ============================================================================
// Scenario: Wire chunk and twin serializers carry the goal-cleared flag and
// rejection count
// ============================================================================

#[path = "cont008_goal_back_sync_test/escalation_keeps_the_goal.rs"]
mod escalation_keeps_the_goal;
#[path = "cont008_goal_back_sync_test/goal_satisfied_teardown_emits.rs"]
mod goal_satisfied_teardown_emits;
#[path = "cont008_goal_back_sync_test/the_dispatch_sync_never.rs"]
mod the_dispatch_sync_never;
