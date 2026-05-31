# RPC-042 — Implement `SessionManagerHandle` for the extracted `SessionManager`

**Parent:** RPC-030 · **Phase:** 4.5 · **Estimate:** 5 pts · **Depends on:** RPC-041

## Goal

In `codelet/sessions/src/lib.rs`, add an `impl codelet_core::SessionManagerHandle for SessionManager { ... }` block covering every method from Phase 3 (RPC-037). Each impl delegates to the corresponding `BackgroundSession` method via `self.sessions.read().get(session_id)`.

After this card, the `fspec` binary can drive real agent sessions via the trait, and the existing `StubSessionManagerHandle`-based tests have a real-handle equivalent.

## Methods to implement (from RPC-037)

```rust
impl codelet_core::SessionManagerHandle for SessionManager {
    fn list_sessions(&self) -> Vec<SessionInfo> {
        self.list_sessions() // already exists on SessionManager
    }

    fn create_session(&self, role: Option<String>) -> SessionId {
        // Use existing async fn create_session — needs sync wrapper:
        let id = tokio::runtime::Handle::current()
            .block_on(async { self.create_session(None, std::env::current_dir().unwrap()).await })
            .expect("create_session failed");
        // If role provided, set it on the new session
        if let Some(r) = role {
            if let Ok(s) = self.get_session(Uuid::parse_str(&id).unwrap()) {
                s.set_role(r);
            }
        }
        SessionId::from(id)
    }

    fn send_input(&self, session_id: &SessionId, text: String) {
        if let Ok(s) = self.get_session(uuid_from(session_id)) {
            let _ = s.send_input(text, None);
        }
    }

    fn send_input_with_thinking(&self, session_id: &SessionId, text: String, thinking: Option<ThinkingConfig>) {
        let thinking_json = thinking.and_then(|t| serde_json::to_string(&t).ok());
        if let Ok(s) = self.get_session(uuid_from(session_id)) {
            let _ = s.send_input(text, thinking_json);
        }
    }

    fn interrupt(&self, session_id: &SessionId) {
        if let Ok(s) = self.get_session(uuid_from(session_id)) {
            s.interrupt();
        }
    }

    fn get_session_status(&self, session_id: &SessionId) -> SessionStatus {
        self.get_session(uuid_from(session_id))
            .map(|s| s.get_status())
            .unwrap_or(SessionStatus::Idle)
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }

    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }

    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }

    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        self.status_changes_tx.subscribe()
    }

    // Per-session derived state
    fn get_session_tokens(&self, session_id: &SessionId) -> SessionTokens {
        self.get_session(uuid_from(session_id))
            .map(|s| {
                let (input, output, _reasoning) = s.get_tokens();
                SessionTokens { input_tokens: input as i64, output_tokens: output as i64 }
            })
            .unwrap_or_default()
    }

    fn get_session_model(&self, session_id: &SessionId) -> SessionModel { /* delegate */ }
    fn get_compaction_progress(&self, session_id: &SessionId) -> Option<CompactionProgress> { /* delegate */ }
    fn get_buffered_output(&self, session_id: &SessionId, limit: u32) -> Vec<StreamChunk> { /* delegate */ }

    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        self.get_session(uuid_from(session_id))
            .map(|s| { s.clear_history(); Ok(()) })
            .unwrap_or_else(|e| Err(e.to_string()))?
    }

    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> { /* delegate */ }
    fn restore_session_messages(&self, session_id: &SessionId, envelopes: Vec<String>) -> Result<(), String> { /* delegate */ }
    fn restore_session_token_state(&self, session_id: &SessionId, state: TokenRestoreState) -> Result<(), String> { /* delegate */ }

    fn get_work_unit_context(&self, session_id: &SessionId) -> Option<WorkUnitContext> {
        self.get_session(uuid_from(session_id)).ok()
            .and_then(|s| s.get_work_unit_context())
    }

    fn set_work_unit_context(&self, session_id: &SessionId, ctx: Option<WorkUnitContext>) -> Result<(), String> {
        let s = self.get_session(uuid_from(session_id)).map_err(|e| e.to_string())?;
        match ctx {
            Some(c) => s.set_work_unit_context(c.id, c.title, c.status),
            None => { /* clear path — needs new method on BackgroundSession */ }
        }
        Ok(())
    }

    fn get_pending_input(&self, session_id: &SessionId) -> Option<String> { /* delegate */ }
    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) { /* delegate */ }

    fn set_active_session(&self, session_id: &SessionId) {
        self.set_active_session(uuid_from(session_id));
    }
    fn clear_active_session(&self) { SessionManager::clear_active_session(self); }
    fn get_active_session(&self) -> Option<SessionId> {
        SessionManager::get_active_session(self).map(|u| SessionId::from(u.to_string()))
    }

    fn get_effective_cwd(&self, session_id: &SessionId) -> PathBuf {
        self.get_session(uuid_from(session_id)).ok()
            .map(|s| s.effective_cwd())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId> {
        SessionManager::get_supervisors(self, uuid_from(session_id))
            .into_iter()
            .map(|u| SessionId::from(u.to_string()))
            .collect()
    }

    fn get_debug_enabled(&self, session_id: &SessionId) -> bool { /* delegate */ }
    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) { /* delegate */ }
    fn toggle_debug(&self, session_id: &SessionId, debug_dir: &str) -> Result<String, String> { /* delegate to BackgroundSession::toggle_debug, lifted from #[napi] toggle_debug at line 7727 */ }

    fn pause_resume(&self, session_id: &SessionId) -> Result<(), String> { /* PauseResponse::Resume */ }
    fn pause_confirm(&self, session_id: &SessionId, accept: bool) -> Result<(), String> { /* PauseResponse::ConfirmAccept/Deny */ }
    fn pause_triple(&self, session_id: &SessionId, choice: ApprovalChoice) -> Result<(), String> { /* PauseResponse::Triple* */ }
    fn send_hitl_response(&self, session_id: &SessionId, response: HitlResponse) -> Result<(), String> { /* delegate */ }
    fn get_pause_state(&self, session_id: &SessionId) -> Option<PauseState> { /* delegate, map codelet_tools::PauseState → codelet_rpc_types::PauseState */ }
    fn get_hitl_request(&self, session_id: &SessionId) -> Option<HitlRequest> { /* delegate */ }

    fn send_fspec_result(&self, session_id: &SessionId, result: FspecResult) -> Result<(), String> {
        let s = self.get_session(uuid_from(session_id)).map_err(|e| e.to_string())?;
        s.send_fspec_result(result);
        Ok(())
    }

    fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String> {
        let id = Uuid::new_v4();
        let project = std::env::current_dir().map_err(|e| e.to_string())?;
        let name = format!("isolated-{}", &id.to_string()[..8]);
        let result = tokio::runtime::Handle::current()
            .block_on(async {
                self.create_isolated_session_with_id(id, None, project, name).await
            })
            .map_err(|e| e.to_string())?;
        // If role provided, set it
        if let Some(r) = role {
            if let Ok(s) = self.get_session(id) {
                s.set_role(r);
            }
        }
        Ok(IsolatedSessionInfo {
            session_id: SessionId::from(id.to_string()),
            worktree_path: result.worktree_path,
            base_commit: result.base_commit,
        })
    }

    fn set_thinking_level_default(&self, session_id: &SessionId, level: ThinkingLevel) -> Result<(), String> {
        let s = self.get_session(uuid_from(session_id)).map_err(|e| e.to_string())?;
        s.set_base_thinking_level(level as u8);
        Ok(())
    }

    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        SessionManager::destroy_session(self, uuid_from(session_id)).map_err(|e| e.to_string())
    }

    // ModelInfo / ThinkingLevel / providers / set_model / set_thinking_level / get_role / set_role
    // (existing — delegate the same way)
}
```

Helper:

```rust
fn uuid_from(id: &SessionId) -> Uuid {
    Uuid::parse_str(id.as_str()).unwrap_or_else(|_| Uuid::nil())
}
```

## Tests to add

`codelet/sessions/tests/handle_impl.rs`:

1. Construct `SessionManager` with a stub `ProviderManager` (or use the real one with a recorded fixture).
2. Cast to `Arc<dyn SessionManagerHandle>`.
3. Drive every method once and assert sensible defaults / non-panic behaviour for a not-yet-created session.
4. `create_session` → `send_input` → consume chunks via `chunks_rx` → `get_session_status` cycle.

## Acceptance criteria

1. `impl SessionManagerHandle for SessionManager` exists in `codelet/sessions/src/lib.rs` covering ALL methods from RPC-037.
2. `cargo build -p codelet-sessions` passes.
3. `codelet/sessions/tests/handle_impl.rs` passes with at least one round-trip per method.
4. `fspec` binary in RPC-044 can construct `Arc::new(SessionManager::new(...)) as Arc<dyn SessionManagerHandle>` without compile errors.
5. Conversion `codelet_tools::tool_pause::PauseState ↔ codelet_rpc_types::PauseState` lives in `codelet/sessions/src/conversions.rs` (or similar).

## Risks

- `tokio::runtime::Handle::current().block_on(...)` inside synchronous trait methods is dangerous if called from a thread without a tokio runtime. The fspec binary always has one. Document this clearly.
- `SessionStatus` ↔ `BackgroundSession::status` (AtomicU8): confirm the discriminant mapping in `SessionStatus::From<u8>` (`codelet/rpc-types/src/lib.rs`) matches `BackgroundSession::set_status` byte values.
- `set_work_unit_context(None)` clear path: `BackgroundSession::set_work_unit_context` currently takes three Strings — extend it to accept `Option<WorkUnitContext>` or add a `clear_work_unit_context` method.

## Out of scope

- NAPI thin-adapter reduction → RPC-043.
- fspec binary wiring → RPC-044.
