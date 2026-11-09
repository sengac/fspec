//! Reusable fixtures for codelet-fspec-tui integration tests (RPC-008
//! architecture note Q-FIX-1).
//!
//! Fixture module — supports multiple feature files (no single Feature:
//! header). Used by integration tests for fspec-tui-embedded-backend,
//! fspec-tui-ws-backend, and fspec-tui-app-shell.
//!
//! RPC-065: also exposes the `harness` sub-module which provides the
//! reusable `AppTestHarness` consumed by `behaviour_parity_rpc065.rs`.
//!
//! These fixtures construct REAL services — real `WorkUnitsWatcher` over a
//! tempdir, real `SharedFspecService`, real `bind_and_serve` rpc-server
//! when needed — so integration tests exercise actual production code
//! paths rather than mocks. The only "mock" allowed is `MockBackend`
//! (added in a later test) since the FspecBackend trait surface is the
//! NEW code under test in this card and it has both real impls plus a
//! controlled in-memory mock for App-level tests.
//!
//! Per the dev-dependency policy locked in architecture note Q-DEV-CORE-1,
//! `codelet-core` is permitted in `[dev-dependencies]` so fixtures can
//! reach `codelet_core::work_units::WorkUnitsWatcher`. Production
//! `[dependencies]` of rust/fspec-tui must NOT contain codelet-core —
//! `tests/source_shape.rs` enforces that asymmetry.

#![allow(dead_code)]

use codelet_test_helpers::OrFail;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use tempfile::TempDir;
mod transcript;
use tokio::task::JoinHandle;
pub mod backend_state_mock;
pub mod call_gates; // BUG-575
pub mod checkpoint_repo; // BUG-355
mod custom_model_backend; // BUG-325 // BUG-466
pub mod model_selection_mock; // BUG-576
use backend_state_mock::BackendState;
mod blocklist_mock;
pub mod exec_control_mock;
mod focus_backend;
mod provider_settings_backend;
pub mod task_outcome; // BUG-487: moved from codelet-test-helpers
pub mod workspace_mock; // RPC-445 // TOOL-027
pub use task_outcome::TaskOutcome;

/// BUG-351: the subscriber tasks `App::bootstrap` spawns. ONE source for
/// every `subscriber_task_count()` assertion: work_units, chunks, logs,
/// status_changes, session_created (RPC-385), checkpoints_progress
/// (TUI-109), git_state (BUG-182), focus, session_removed (RPC-434),
/// session_cleared (RPC-437), workspace work units (RPC-445) and workspace
/// git state (RPC-454).
pub const BOOTSTRAP_SUBSCRIBER_TASKS: usize = 12;

// Shared fixtures (seeded service, WS server, source readers).
mod fixtures;
// Shared by every test binary; each uses only some of the fixtures.
#[allow(unused_imports)]
pub use fixtures::*;

// ─────────────────────────────────────────────────────────────────────────
// Fixture (3) per Q-FIX-1: MockBackend (extended in RPC-009 with scripted
// create_session/send_input/interrupt + per-call counters + chunks_tx
// publisher per architecture note [10]).
// ─────────────────────────────────────────────────────────────────────────

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::FspecBackend;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CheckpointCounts, CompactionResult, ExecStdinRequest,
    FspecResult, GitState, HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo,
    LogRecord, ModelEntry, ModelInfo, PauseState, PendingRequest, ProviderCredentialInfo,
    ProviderCredentialInput, ProviderInfo, RequestHolder, RequestKind, RequestRoute, SessionId,
    SessionInfo, SessionStatus, StreamChunk, TestConnectionResult, ThinkingLevel, WorkUnitContext,
    WorkUnitInfo, WorkspaceInfo,
};
use tokio::sync::broadcast;

/// BUG-184: scripted checkpoint-file-diff payload — `(work_unit_id,
/// checkpoint_name, file_path, diff_text)`.
type ScriptedCheckpointFileDiff = (String, String, String, Option<String>);

/// In-memory FspecBackend with seedable data and per-channel
/// broadcast::Sender handles tests use to push synthetic events. Used by
/// the App-level integration tests where a real WS server or service is
/// overkill: the goal is to exercise the App's wiring against
/// `Arc<dyn FspecBackend>`, not the transport itself.
#[derive(Default)]
pub struct MockBackend {
    /// Scripted workspaces (`workspace_mock.rs`).
    pub workspaces: workspace_mock::WorkspaceMock,
    /// BUG-575: hold individual calls and release them in any order.
    pub gates: call_gates::CallGates,
    /// BUG-601: per-session turned-off blocklist rules (`blocklist_mock.rs`).
    blocklist: blocklist_mock::BlocklistMock,
    /// BUG-576: every `set_session_model` call and scripted failures.
    pub model_selections: model_selection_mock::ModelSelectionMock,
    core: mock_core::MockCore,
    controls: mock_controls::MockControls,
    orchestration: mock_orchestration::MockOrchestration,
    /// Every `control_exec_stdin` call and a scripted failure.
    exec_control: exec_control_mock::ExecControlMock,
    /// The session continue / goal state this fake backend stores (the
    /// FspecBackend defaults refuse) and a scripted listing failure.
    backend_state: BackendState,
}

mod mock_controls;
mod mock_controls_default;
mod mock_core;
mod mock_core_default;
mod mock_orchestration;
use mock_controls::MockControls;
use mock_core::MockCore;

mod mock_scripting_1;
mod mock_scripting_1_b;
mod mock_scripting_2;
mod mock_scripting_2_b;
mod mock_scripting_3;
mod mock_scripting_3_b;
mod mock_scripting_4;
mod mock_scripting_4_b;
#[allow(unused_imports)]
pub use mock_scripting_4_b::{hitl_route, pause_route}; // AMGR-020
mod mock_scripting_5;
mod mock_scripting_5_b;
mod mock_scripting_6;
mod mock_scripting_6_b;

mod mock_delegation;
mod mock_methods_1;
mod mock_methods_1_b;
mod mock_methods_2;
mod mock_methods_2_b;
mod mock_methods_3;
mod mock_methods_3_b;
mod mock_methods_4;
mod mock_methods_4_b;
mod mock_methods_5;
mod mock_methods_5_b;
mod mock_methods_6;
mod mock_methods_6_b;
mod mock_methods_7;
mod mock_methods_7_b;

// ─────────────────────────────────────────────────────────────────────────
// Fixture (4) per Q-FIX-1: test_app — App + 80x24 TestBackend pair
// ─────────────────────────────────────────────────────────────────────────

use codelet_fspec_tui::App;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Construct an [`App`] alongside an 80x24 [`Terminal<TestBackend>`]
/// suitable for App-level integration tests.
pub fn test_app(
    backend: Arc<dyn FspecBackend>,
) -> Result<(App, Terminal<TestBackend>), codelet_test_helpers::TestError> {
    let app = App::new(backend);
    let terminal_backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(terminal_backend).or_fail("Terminal::new(TestBackend)")?;
    Ok((app, terminal))
}

// ─────────────────────────────────────────────────────────────────────────
// Fixture (5) per Q-FIX-1: render_one_frame
// ─────────────────────────────────────────────────────────────────────────

use ratatui::buffer::Buffer;

/// Drive a single render cycle of `app` against `terminal` and return
/// a clone of the resulting [`Buffer`] for snapshotting.
pub fn render_one_frame(
    terminal: &mut Terminal<TestBackend>,
    app: &mut App,
) -> Result<Buffer, codelet_test_helpers::TestError> {
    terminal
        .draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })
        .or_fail("Terminal::draw")?;
    Ok(terminal.backend().buffer().clone())
}

/// Convert a [`Buffer`] into a Vec<String> of row text — one entry per
/// row, suitable for `insta::assert_yaml_snapshot!`.
pub fn buffer_to_rows(buf: &Buffer) -> Vec<String> {
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::with_capacity(buf.area.width as usize);
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

// ─────────────────────────────────────────────────────────────────────────
// RPC-065 — AppTestHarness sub-module
// ─────────────────────────────────────────────────────────────────────────

pub mod harness;

/// MODEL-011: the options of a model offering Off and the efforts low,
/// medium and high, for tests of the thinking dialog's generic behaviour.
pub fn off_low_medium_high_options() -> codelet_rpc_types::ThinkingOptions {
    codelet_rpc_types::ThinkingOptions::Levels {
        model: "test/model".to_string(),
        choices: vec![
            codelet_rpc_types::ThinkingChoice::Off,
            codelet_rpc_types::ThinkingChoice::Effort("low".to_string()),
            codelet_rpc_types::ThinkingChoice::Effort("medium".to_string()),
            codelet_rpc_types::ThinkingChoice::Effort("high".to_string()),
        ],
        takes_off: true,
    }
}

/// TUI-126: the clipboard and URL-launcher seams for copy and open tests.
pub mod clipboard;
