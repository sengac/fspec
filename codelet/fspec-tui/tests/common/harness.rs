//! RPC-065 — Reusable AppTestHarness for behaviour-parity tests.
//!
//! Feature: spec/features/agent-view-behaviour-parity-matrix.feature
//!
//! Wraps an `App` + `Arc<MockBackend>` (the existing 2876-LoC double in
//! `tests/common/mod.rs`) and exposes ergonomic helpers so the parity
//! matrix tests in `behaviour_parity_rpc065.rs` never need to re-roll
//! the seed_chunks / scrollback_text / drain_pending / wait_until
//! boilerplate that's duplicated across slash_clear_rpc046.rs,
//! slash_compact_rpc047.rs, slash_role_rpc063.rs, slash_thinking_rpc048.rs,
//! slash_resume_rpc049.rs, slash_detach_rpc050.rs, and friends.
//!
//! ## Lifecycle
//!
//! ```text
//! let mut h = AppTestHarness::new();      // session s-1 seeded + focused
//! h.dispatch_slash(SlashCommandAction::Help);
//! assert!(h.compositor_contains("help-dialog"));
//! ```
//!
//! ## Design constraints
//!
//! 1. Wraps `MockBackend` (NOT `StubSessionManagerHandle`) — every
//!    counter the matrix asserts is already exposed there.
//! 2. Lives in `tests/common/harness.rs` so no production-code change
//!    is required.
//! 3. `drain_pending().await` re-uses the well-tested pattern from
//!    `slash_clear_rpc046.rs::drain_pending`: pop spawned tasks, then
//!    re-dispatch any queued actions, then pop spawned tasks again,
//!    bounded by a 1-second timeout.
//! 4. Every helper is sync EXCEPT `drain_pending` and `wait_until` —
//!    matches the cadence of the existing per-card tests.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, RenderedChunk, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Line;
use tokio::time::timeout;

use super::MockBackend;

/// Convenience constructor for the seed SessionId used by `new()`.
pub fn seed_sid() -> SessionId {
    SessionId::new("s-1")
}

/// Builder + helper façade over an `App` + `Arc<MockBackend>` for the
/// behaviour-parity test suite.
pub struct AppTestHarness {
    pub app: App,
    pub mock: Arc<MockBackend>,
}

impl AppTestHarness {
    // ── Constructors ───────────────────────────────────────────────────

    /// Default constructor — fresh App wired to a fresh MockBackend
    /// with a single seeded SessionId `"s-1"` already focused AND the
    /// navigator switched into [`ViewMode::Agent`] so keyboard events
    /// (Shift+Arrow, Ctrl+R, Ctrl+C, …) reach the AgentView's
    /// dispatch path instead of the BoardView.
    pub fn new() -> Self {
        let mock = Arc::new(MockBackend::new());
        let backend: Arc<dyn FspecBackend> = mock.clone();
        let mut app = App::new(backend);
        app.dispatch(Action::SessionCreated(seed_sid()));
        // Switch to the AgentView so keyboard events route through
        // the agent's dispatch.rs (handle_event) rather than the
        // BoardView. Mirrors what `Action::EnterWorkUnit` /
        // `Action::OpenAgentView` do at runtime.
        app.dispatch(Action::OpenAgentView(Some(seed_sid())));
        Self { app, mock }
    }

    /// Constructor with no seeded session — for tests that need
    /// "no current session" semantics.
    pub fn empty() -> Self {
        let mock = Arc::new(MockBackend::new());
        let backend: Arc<dyn FspecBackend> = mock.clone();
        let app = App::new(backend);
        Self { app, mock }
    }

    // ── Session topology helpers ───────────────────────────────────────

    /// Add a session to the App (extra to the seed). Convenience over
    /// `app.dispatch(Action::SessionCreated(id))`.
    pub fn add_session(&mut self, id: SessionId) {
        self.app.dispatch(Action::SessionCreated(id));
    }

    /// Push `count` raw scrollback chunks into the SessionContext for
    /// `id`. Mirrors the `seed_chunks` helper in
    /// `slash_clear_rpc046.rs`. Panics if no such session is open.
    pub fn seed_chunks(&mut self, id: &SessionId, count: usize) {
        let ctx = self
            .app
            .agent_view_store_mut()
            .session_context_mut_for(id)
            .expect("SessionContext present for seeded id");
        for i in 0..count {
            ctx.scrollback.push(RenderedChunk {
                seq: i as u64,
                lines: vec![Line::from(format!("seed-{i}"))],
                source: None,
            });
        }
    }

    // ── Dispatch helpers ───────────────────────────────────────────────

    /// Sugar over `app.dispatch(Action::SlashCommandSelected(action))`.
    pub fn dispatch_slash(&mut self, action: SlashCommandAction) {
        self.app.dispatch(Action::SlashCommandSelected(action));
    }

    /// Drive submit-input through the dispatch path —
    /// `app.dispatch(Action::InputSubmitted(text.to_string()))`. This is
    /// the entry the slash-parser hooks into for inline-arg branches
    /// like `/thinking high` and `/role <text>`.
    pub fn submit_input(&mut self, text: &str) {
        self.app
            .dispatch(Action::InputSubmitted(text.to_string()));
    }

    /// Drive a single key event through the navigator's handle_event
    /// path. Use the convenience constructors below for the common
    /// shortcuts (`key_shift_right()`, `key_ctrl_c()`, etc.).
    pub fn press_key(&mut self, key: KeyEvent) {
        let _ = self.app.handle_event(&Event::Key(key));
    }

    // ── Common KeyEvent constructors (sugar) ───────────────────────────

    pub fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    pub fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    pub fn key_ctrl_c() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    }

    pub fn key_ctrl_r() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
    }

    pub fn key_shift_right() -> KeyEvent {
        KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)
    }

    pub fn key_shift_left() -> KeyEvent {
        KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)
    }

    pub fn key_shift_up() -> KeyEvent {
        KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT)
    }

    // ── Observable state accessors ─────────────────────────────────────

    pub fn current_session(&self) -> Option<SessionId> {
        self.app.current_session()
    }

    /// Returns true iff the compositor has a layer with the given id.
    pub fn compositor_contains(&self, id: &str) -> bool {
        self.app.compositor().contains(id)
    }

    pub fn active_view(&self) -> ViewMode {
        self.app.active_view()
    }

    pub fn should_quit(&self) -> bool {
        self.app.should_quit()
    }

    pub fn scrollback_chunk_count(&self, id: &SessionId) -> usize {
        self.app
            .agent_view_store()
            .session_context_for(id)
            .map(|c| c.scrollback.chunk_count())
            .unwrap_or(0)
    }

    /// Current `offset` of `id`'s scrollback (first visible chunk index).
    /// Returns 0 when no SessionContext exists.
    pub fn scrollback_offset(&self, id: &SessionId) -> usize {
        self.app
            .agent_view_store()
            .session_context_for(id)
            .map(|c| c.scrollback.scroll_state().offset)
            .unwrap_or(0)
    }

    /// Whether `id`'s scrollback is currently stuck to the bottom
    /// (i.e. End / fresh-push semantics). Returns `false` when no
    /// SessionContext exists.
    pub fn scrollback_stick_to_bottom(&self, id: &SessionId) -> bool {
        self.app
            .agent_view_store()
            .session_context_for(id)
            .map(|c| c.scrollback.scroll_state().stick_to_bottom)
            .unwrap_or(false)
    }

    /// Seed the viewport height for `id`'s scrollback. Needed in tests
    /// that exercise PageUp / PageDown before any render has happened,
    /// because the page-size arithmetic in `App::dispatch` reads
    /// `scrollback_viewport_hint()` (which defaults to 10 when unset).
    pub fn set_scrollback_viewport_height(&mut self, id: &SessionId, height: u16) {
        if let Some(ctx) = self
            .app
            .agent_view_store_mut()
            .session_context_mut_for(id)
        {
            ctx.scrollback.set_viewport_height(height);
        }
    }

    /// Flatten every visible chunk's text in `id`'s scrollback into a
    /// single newline-joined String. Mirrors `session_scrollback_text`
    /// in `slash_clear_rpc046.rs`.
    pub fn scrollback_text(&self, id: &SessionId) -> String {
        let chunks = self
            .app
            .agent_view_store()
            .session_context_for(id)
            .map(|c| c.scrollback.visible_window(1024))
            .unwrap_or_default();
        chunks
            .iter()
            .flat_map(|c| {
                c.lines.iter().map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<String>()
                })
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// The current value of the navigator's input buffer.
    pub fn input_value(&self) -> String {
        self.app.navigator().agent.input.value()
    }

    /// Set the navigator's input value directly (for Esc-cascade tests
    /// that need to assert the input is left unchanged).
    pub fn set_input(&mut self, value: &str) {
        self.app.navigator_mut().agent.input.set_value(value);
    }

    /// Whether the AgentView's `search_view` mode-view is active.
    pub fn search_view_active(&self) -> bool {
        self.app.navigator().agent.search_view.is_some()
    }

    /// Whether the AgentView's `resume_view` mode-view is active.
    pub fn resume_view_active(&self) -> bool {
        self.app.navigator().agent.resume_view.is_some()
    }

    // ── Async drain / wait helpers ─────────────────────────────────────

    /// Await every spawned tokio task on `pending_tasks` AND fold any
    /// queued action_tx messages back into the App. Mirrors
    /// `slash_clear_rpc046.rs::drain_pending` exactly. Wrapped in a
    /// 2-second timeout so a stuck test fails fast.
    pub async fn drain_pending(&mut self) {
        let _ = timeout(Duration::from_secs(2), async {
            while let Some(handle) = self.app.next_pending_task() {
                let _ = handle.await;
            }
            while let Some(action) = self.app.try_recv_action() {
                self.app.dispatch(action);
                while let Some(handle) = self.app.next_pending_task() {
                    let _ = handle.await;
                }
            }
        })
        .await;
    }

    /// Poll-until-true helper. The predicate inspects the captured
    /// [`MockBackend`] (passed by reference). Between polls the harness
    /// drains any queued actions / pending tasks so spawned-task
    /// results land in the store before the predicate runs again.
    /// Wrapped in a 1-second timeout.
    pub async fn wait_for_mock<F: FnMut(&MockBackend) -> bool>(
        &mut self,
        mut predicate: F,
        label: &str,
    ) {
        let result = timeout(Duration::from_secs(1), async {
            loop {
                if predicate(&self.mock) {
                    return;
                }
                // Drain ready actions + spawned tasks so the next
                // predicate evaluation sees the latest counters.
                while let Some(action) = self.app.try_recv_action() {
                    self.app.dispatch(action);
                }
                while let Some(handle) = self.app.next_pending_task() {
                    let _ = handle.await;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        if result.is_err() {
            panic!("timed out waiting for: {label}");
        }
    }

    /// Poll-until-true helper that inspects the harness's own App
    /// state (compositor / store / scrollback). Identical mechanics to
    /// [`wait_for_mock`] but the predicate gets `&AppTestHarness`.
    pub async fn wait_for_self<F: FnMut(&AppTestHarness) -> bool>(
        &mut self,
        mut predicate: F,
        label: &str,
    ) {
        let result = timeout(Duration::from_secs(1), async {
            loop {
                if predicate(self) {
                    return;
                }
                while let Some(action) = self.app.try_recv_action() {
                    self.app.dispatch(action);
                }
                while let Some(handle) = self.app.next_pending_task() {
                    let _ = handle.await;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        if result.is_err() {
            panic!("timed out waiting for: {label}");
        }
    }
}

impl Default for AppTestHarness {
    fn default() -> Self {
        Self::new()
    }
}
