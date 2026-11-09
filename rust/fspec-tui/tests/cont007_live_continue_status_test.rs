//! Feature: spec/features/live-continue-status-indicator.feature
//!
//! CONT-007: live status-bar updates for the auto-continue / goal nudge
//! counter. A new state-only `StreamChunk::ContinueStateUpdate` is pushed
//! by the engine at every counter transition and folded into the TUI
//! chrome cache so the footer paints the REAL `nudges_used` instead of a
//! literal 0, while the per-nudge chat print is removed from the
//! transcript (CliOutput keeps the stdout line).
//!
//! Test surfaces (cont005/cont006 precedent):
//! - rpc-types serde round-trip + SessionContext state-only behavior,
//! - behavioral tests on the REAL production helpers
//!   (`continue_state_event`, `apply_finish_with_summary`) with a
//!   RecordingOutput callback sink (vi.fn()-style, no module mocks),
//! - App + MockBackend chunk-dispatch tests asserting chrome cache and
//!   `paint_footer` buffer output,
//! - source-shape pins (rpc082/083 precedent) for the stream-loop
//!   emission sites, twin mappings and JSON serializers that cannot be
//!   driven without a live provider stream.

use codelet_test_helpers::OrFail;
use std::sync::Mutex;

use codelet_cli::interactive::continue_state::continue_state_event;
use codelet_cli::interactive::done_early_exit::apply_finish_with_summary;
use codelet_cli::interactive::output::{ContinueStateReason, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_fspec_tui::store::agent_view::chrome_state::ContinueLiveState;
use codelet_fspec_tui::views::agent::chrome_paint::{paint_footer, ChromeAreas};
use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{ContinueStateInfo, SessionId, StreamChunk};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::sync::Arc;

mod common;
use common::MockBackend;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Recording StreamOutput — callback event sink so tests can assert on the
/// emitted events (vi.fn()-style sink, not a module replacement).
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
fn fresh_session() -> Result<Session, codelet_test_helpers::TestError> {
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

/// Paint the SessionFooter for `session` into a 60x1 buffer and return the
/// painted row text.
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
    // Collapse space runs: the double-width ⏩/🎯 glyphs leave a blank
    // continuation cell in the buffer that reads back as an extra space.
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

/// Strip `//` line comments so source-shape pins cannot false-positive on
/// commentary (cont009 precedent).
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
// Scenario: ContinueStateUpdate chunk round-trips and stays out of the
// transcript
// ============================================================================

// ============================================================================
// Scenario: Engine snapshot reports the effective goal budget
// ============================================================================

// ============================================================================
// Scenario: Accepted done() teardown emits a reset counter state from the
// shared teardown
// ============================================================================

// ============================================================================
// Scenario: Footer paints the real nudge counter from a live update
// ============================================================================

// ============================================================================
// Scenario: Slash dispatch drops the stale live counter
// ============================================================================

// ============================================================================
// Scenario: Emission sites, twin mappings, and chat-print removal are pinned
// in the sources
// ============================================================================

#[path = "cont007_live_continue_status_test/continue_state_update_chunk.rs"]
mod continue_state_update_chunk;
#[path = "cont007_live_continue_status_test/slash_dispatch_drops_the.rs"]
mod slash_dispatch_drops_the;
