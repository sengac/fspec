//! AppTestHarness driving and assertion helpers (REFAC-025 split).

use super::super::TaskOutcome;
use super::*;

impl AppTestHarness {
    /// Seed the viewport height for `id`'s scrollback. Needed in tests
    /// that exercise PageUp / PageDown before any render has happened,
    /// because the page-size arithmetic in `App::dispatch` reads
    /// `scrollback_viewport_hint()` (which defaults to 10 when unset).
    pub fn set_scrollback_viewport_height(&mut self, id: &SessionId, height: u16) {
        if let Some(ctx) = self.app.agent_view_store_mut().session_context_mut_for(id) {
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
    /// `slash_clear_rpc046.rs::drain_pending` exactly. BUG-383: a drain that
    /// never finishes (TEST-042: the deadlock guard), or a task that
    /// panicked or was cancelled, is an error naming the cause.
    pub async fn drain_pending(&mut self) -> Result<(), String> {
        codelet_test_helpers::deadlock_guard("draining pending tasks", async {
            self.await_pending_tasks().await?;
            while let Some(action) = self.app.try_recv_action() {
                self.app.dispatch(action);
                self.await_pending_tasks().await?;
            }
            Ok(())
        })
        .await
        .map_err(|failure| failure.to_string())?
    }
    /// Await every queued task, reporting the first that panicked; a task
    /// the App cancelled is finished.
    async fn await_pending_tasks(&mut self) -> Result<(), String> {
        while let Some(handle) = self.app.next_pending_task() {
            handle
                .await
                .completed_or_cancelled()
                .map_err(|error| format!("a pending task failed: {error}"))?;
        }
        Ok(())
    }
    /// Poll-until-true helper. The predicate inspects the captured
    /// [`MockBackend`] (passed by reference). Between polls the harness
    /// drains any queued actions / pending tasks so spawned-task
    /// results land in the store before the predicate runs again.
    /// BUG-383: an expired deadlock guard (TEST-042) or a failed task is an
    /// error.
    pub async fn wait_for_mock<F: FnMut(&MockBackend) -> bool>(
        &mut self,
        mut predicate: F,
        label: &str,
    ) -> Result<(), String> {
        codelet_test_helpers::deadlock_guard(label, async {
            loop {
                if predicate(&self.mock) {
                    return Ok(());
                }
                // Drain ready actions + spawned tasks so the next
                // predicate evaluation sees the latest counters.
                while let Some(action) = self.app.try_recv_action() {
                    self.app.dispatch(action);
                }
                self.await_pending_tasks().await?;
                tokio::task::yield_now().await;
            }
        })
        .await
        .map_err(|failure| failure.to_string())?
    }
    /// Poll-until-true helper that inspects the harness's own App
    /// state (compositor / store / scrollback). Identical mechanics to
    /// [`wait_for_mock`] but the predicate gets `&AppTestHarness`.
    pub async fn wait_for_self<F: FnMut(&AppTestHarness) -> bool>(
        &mut self,
        mut predicate: F,
        label: &str,
    ) -> Result<(), String> {
        codelet_test_helpers::deadlock_guard(label, async {
            loop {
                if predicate(self) {
                    return Ok(());
                }
                while let Some(action) = self.app.try_recv_action() {
                    self.app.dispatch(action);
                }
                self.await_pending_tasks().await?;
                tokio::task::yield_now().await;
            }
        })
        .await
        .map_err(|failure| failure.to_string())?
    }
}
