# RPC-045 — AgentView: subscribe to chunks + status broadcasts; handle every new `StreamChunk` variant

**Parent:** RPC-030 · **Phase:** 6.1-6.3 · **Estimate:** 5 pts · **Depends on:** RPC-044

## Goal

In `codelet/fspec-tui/src/app/run.rs` (the `App::run` `tokio::select!`), subscribe to `backend.chunks_rx()` AND `backend.status_changes_rx()`, dispatch chunks to `App::dispatch`, and drop the existing polling `get_session_status` calls. Wire every new chunk variant (the ones added or completed in RPC-036) into store state.

## Source — `codelet/fspec-tui/src/app/run.rs`

The `App::run` loop currently has a `tokio::select!` that polls or selectively subscribes. Audit the file and extend with:

```rust
let mut chunks_rx = backend.chunks_rx();
let mut status_rx = backend.status_changes_rx();
let mut work_units_rx = backend.work_units_rx();

loop {
    tokio::select! {
        // ... existing event-loop arms ...

        chunk_result = chunks_rx.recv() => {
            match chunk_result {
                Ok((session_id, chunk)) => {
                    self.dispatch(Action::StreamChunkReceived { session_id, chunk });
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(target: "agent_view", "chunks_rx lagged by {n}");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        status_result = status_rx.recv() => {
            match status_result {
                Ok((session_id, status)) => {
                    self.dispatch(Action::SessionStatusChanged { session_id, status });
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        // ... work_units_rx, key events, etc. ...
    }
}
```

## Action additions

In `codelet/fspec-tui/src/components/mod.rs` or wherever `Action` lives:

```rust
pub enum Action {
    // ... existing ...
    StreamChunkReceived { session_id: SessionId, chunk: StreamChunk },
    SessionStatusChanged { session_id: SessionId, status: SessionStatus },
}
```

## Dispatcher — `codelet/fspec-tui/src/views/agent/dispatch.rs` (and `app/dispatch_rpc020.rs`)

Add `handle_stream_chunk_received(session_id, chunk)` that branches on variant:

```rust
fn handle_stream_chunk_received(&mut self, session_id: SessionId, chunk: StreamChunk) {
    // Always: route into per-session scrollback
    if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session_id) {
        ctx.record_chunk(&chunk);
    }
    // Always: update token state where applicable
    self.agent_view_store.apply_chunk_to_token_state(&session_id, &chunk);

    match &chunk {
        StreamChunk::SessionStateChange { state } => {
            // SessionContext does not currently carry a status field — add one,
            // or store in AgentViewStore.session_status_by_session: HashMap.
            self.agent_view_store.set_session_status(session_id.clone(), (*state).into());
        }
        StreamChunk::IsolationStateChange { is_isolated, worktree_path } => {
            self.agent_view_store.set_isolation_state(
                session_id.clone(),
                IsolationState { is_isolated: *is_isolated, worktree_path: worktree_path.clone() },
            );
        }
        StreamChunk::DebugStateChange { enabled } => {
            self.agent_view_store.set_debug_state(session_id.clone(), *enabled);
        }
        StreamChunk::FooterStateUpdate { cwd, display_path, is_git_repo, branch } => {
            self.agent_view_store.set_workspace(WorkspaceInfo {
                cwd: cwd.clone(),
                display_path: display_path.clone(),
                is_git_repo: *is_git_repo,
                branch: branch.clone(),
            });
        }
        StreamChunk::FspecCommandRequest { fspec_request } => {
            // Spawn task to execute the requested command and send result back.
            self.spawn_fspec_command_runner(session_id.clone(), fspec_request.clone());
        }
        _ => { /* already handled by record_chunk */ }
    }
}

fn handle_session_status_changed(&mut self, session_id: SessionId, status: SessionStatus) {
    self.agent_view_store.set_session_status(session_id, status);
    // SessionFooter status pill reads from the store on next render.
}
```

## FspecCommandRequest runner

The TS frontend's `runFspecCommand` (in `src/tui/services/fspec-runner.ts` or similar) executes the requested CLI command and returns a `FspecResult`. The Rust equivalent:

```rust
fn spawn_fspec_command_runner(&self, session_id: SessionId, request: FspecRequest) {
    let backend = self.backend.clone();
    tokio::spawn(async move {
        // 1. Parse request.command + request.args_json
        // 2. Execute via a local command dispatcher (reuse codelet-fspec command-line surface)
        // 3. Construct FspecResult { success, data, error, system_reminder, tool_call_id }
        // 4. Call backend.send_fspec_result(session_id, result).
        let result = run_fspec_command(&request).await;
        let _ = backend.send_fspec_result(session_id, result).await;
    });
}
```

The actual command-dispatch helper may live in `codelet/fspec/src/command_dispatcher.rs` or be reused from the existing CLI entry point. Confirm a shared helper exists or scope it to "happy path: list-work-units, show-work-unit only" for this card and expand later.

## Drop polling `get_session_status`

Search `codelet/fspec-tui/src/` for `get_session_status` and remove any periodic polling. `SessionFooter` reads `agent_view_store.session_status_for(&session_id)` (synchronous, in-memory).

## Store additions

`AgentViewStore` gains:

```rust
session_status_by_session: HashMap<SessionId, SessionStatus>,
isolation_state_by_session: HashMap<SessionId, IsolationState>,
debug_state_by_session: HashMap<SessionId, bool>,
```

Plus accessors. Also extend `SessionContext::record_chunk` (in `store/agent_view/session_context.rs`) to handle the new variants (mostly already there from RPC-029).

## Acceptance criteria

1. `App::run` subscribes to `chunks_rx` and `status_changes_rx`.
2. Polling `get_session_status` is removed.
3. `Action::StreamChunkReceived` and `Action::SessionStatusChanged` exist and are dispatched.
4. The 5 specifically-new dispatcher branches handle `SessionStateChange`, `IsolationStateChange`, `DebugStateChange`, `FooterStateUpdate`, `FspecCommandRequest`.
5. `FspecCommandRequest` round-trips: a stub session that emits a request gets a `FspecResult` back within 1 second.
6. SessionFooter status pill updates within one frame of a `SessionStatus` broadcast.
7. Integration test in `codelet/fspec-tui/tests/chunk_dispatch.rs` drives a stub backend through all 5 new variants and asserts store state.

## Risks

- Broadcast lag: at high chunk volumes, the `chunks_rx` may lag. The TS frontend has its own batching strategy. For now, just log the lag — RPC-045 doesn't need to match TS batching.
- `FspecCommandRequest` handling can deadlock if the request runner blocks the App task. ALWAYS spawn on `tokio::spawn`.
- `SessionFooter` re-render frequency: ratatui re-draws on every frame. Status updates that flicker between Idle/Running/Idle rapidly may look janky. Mitigate with a 100ms debounce in the render path (separate concern).

## Out of scope

- Specific slash-command wiring → RPC-046 onwards.
- Pause/HITL UI → RPC-053.
