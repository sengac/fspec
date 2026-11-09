//! Feature: spec/features/rust-tui-compact-real-compaction.feature
//!
//! RPC-418 — Behavioural tests for `SessionManagerHandle::compact_session`.
//! Each `#[tokio::test]` maps 1:1 to a Gherkin scenario in the feature file.
//!
//! The tests construct a fresh `SessionManager`, create a session via the
//! trait's `create_session` bridge (which uses `NoopSessionManagerHooks` so no
//! agent loop is spawned), seed `session.inner.lock().await.messages` with
//! hand-crafted rig messages + token counts, then drive the REAL sync trait
//! method `compact_session` through `SessionManagerHandle`.
//!
//! They exercise the real handle (NOT the NAPI `session_compact` path).
//!
//! multi-thread runtime is REQUIRED: `compact_session` bridges sync->async via
//! `tokio::task::block_in_place`, which panics on a single-thread runtime.
//!
//! These tests are written BEFORE the implementation and MUST FAIL against the
//! current 1:1 no-op stub in `handle_impl.rs` (red phase): the stub never
//! clears the conversation, never injects the compaction instruction, never
//! sends "Continue", never errors on an empty session, and always returns
//! `compacted_tokens == original_tokens` with ratio 1.0.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "fixtures/offline_catalog.rs"]
mod offline_catalog;

use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, SessionStatus};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_sessions::session_manager::SessionManagerHooks;
use codelet_sessions::SessionManager;
use codelet_tools::McpInjection;
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;
use tokio::sync::{mpsc, Mutex};

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`) so a parallel test in this binary
/// cannot swap the pointer out from under another test's `SessionManager::new()`
/// (which eagerly loads `<data_dir>/default-model.json`). Mirrors the
/// `DATA_DIR_GUARD` pattern from `rpc081_restore_session_messages.rs`.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Keeps each session's input channels open without running an agent loop,
/// so the queued manual-compaction request has a live receiver (as the real
/// agent loop provides) while the test inspects the session directly.
#[derive(Default)]
struct RetainedLoop(StdMutex<Vec<(mpsc::Receiver<PromptInput>, mpsc::Receiver<McpInjection>)>>);

impl SessionManagerHooks for RetainedLoop {
    fn spawn_agent_loop(
        &self,
        _: Arc<BackgroundSession>,
        input: mpsc::Receiver<PromptInput>,
        mcp: mpsc::Receiver<McpInjection>,
    ) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((input, mcp));
    }

    fn spawn_scheduler(&self, _: String, _: tokio::runtime::Handle) {}
    fn ensure_scheduler_running_for_loop(&self, _: String, _: tokio::runtime::Handle) {}
    fn spawn_footer_poller(&self, _: String, _: String, _: Option<String>) {}
    fn stop_footer_poller(&self, _: &str) {}
    fn cleanup_session_loops(&self, _: uuid::Uuid) {}
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
/// `RetainedLoop` hooks keep its input channel open without an agent loop.
async fn fresh_session(manager: &SessionManager) -> SessionId {
    manager.set_hooks(Arc::new(RetainedLoop::default()));
    offline_catalog::seed_offline_catalog().expect("seed the offline model catalog");
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager;
    handle
        .create_session_checked(None)
        .expect("create_session must succeed")
}

/// Seed the session's inner conversation with several user/assistant messages
/// and a non-zero input-token count, mirroring a populated conversation.
/// Returns the number of messages present after seeding.
async fn seed_populated(manager: &SessionManager, sid: &SessionId) -> usize {
    let session = manager.get_session(&sid.value).expect("session must exist");
    let mut inner = session.inner.lock().await;
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Please refactor the auth module")),
    });
    inner.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text("Sure, reading the files now")),
    });
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Also add rate limiting")),
    });
    inner.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text("Done, here is the plan")),
    });
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Looks good, ship it")),
    });
    inner.token_tracker.input_tokens = 5000;
    inner.token_tracker.output_tokens = 1000;
    inner.messages.len()
}

// ============================================================================
// Scenario: Compacting a populated session runs the compactor agent and
// replaces the conversation with its summary (BUG-516: agent-loop test)
// ============================================================================

// ============================================================================
// Scenario: Compacting an empty session returns an error and leaves it
// untouched
// ============================================================================

// ============================================================================
// Scenario: Compacting a populated session reports real token counts
// ============================================================================

// ============================================================================
// Scenario: Compacting an unknown session id returns a not-found error
// ============================================================================

#[path = "rpc418_compact_session/compact_populated_clears_conversation.rs"]
mod compact_populated_clears_conversation;

// CMPCT-039: compression-ratio-clamping owns its own file in this target.
#[path = "rpc418_compact_session/compression_ratio_clamping.rs"]
mod compression_ratio_clamping;
