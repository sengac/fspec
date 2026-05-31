# RPC-061 — Supervisor / subordinate links surface

**Parent:** RPC-030 · **Phase:** 7.8 · **Estimate:** 8 pts · **Depends on:** RPC-060

## Goal

Port the supervisor / subordinate session-linking feature (WATCH-003/006/008/011/019/020 work units, originally landed in TS). Trait additions, AgentView badge, "incoming message" plumbing.

## Backend state (already lifted)

`SessionManager` methods (in `codelet-sessions` after RPC-040):
- `add_supervisor(subordinate_id, supervisor_id)` (line 3993)
- `remove_supervisor(supervisor_id)` (line 3998)
- `get_supervisors(subordinate_id) -> Vec<Uuid>` (line 4003)
- `get_subordinate(supervisor_id) -> Option<Uuid>` (line 4008)
- `get_subordinates(supervisor_id) -> Vec<Uuid>` (line 4013)

`BackgroundSession`:
- `supervisor_broadcast: broadcast::Sender<StreamChunk>` (line 513) — pushes to supervisors
- `incoming_message_tx/rx` (lines 520-521) — supervisor-to-subordinate messages
- `incoming_message_pending: Arc<AtomicUsize>` (line 526) — pending count
- `receive_incoming_message(input)` (line 1206)

`ChainOfCommand` (line 3144 of `session_manager.rs`) is the tracker; lifted with `SessionManager` in RPC-040.

## Trait additions (extend RPC-037)

```rust
fn add_supervisor(&self, subordinate_id: &SessionId, supervisor_id: &SessionId) -> Result<(), String>;
fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String>;
fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId>; // already in RPC-037
fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId>;
fn get_subordinates(&self, supervisor_id: &SessionId) -> Vec<SessionId>;
fn receive_incoming_message(&self, subordinate_id: &SessionId, message: IncomingMessage) -> Result<(), String>;
```

Wire type `IncomingMessage` already exists indirectly (`StreamChunk::IncomingMessage { text, images }`). Pass as a struct in the RPC call.

## Frontend — subordinate badge

When `agent_view_store.supervisors_for(&session_id)` returns a non-empty vec, render `[Subordinate of: <id>]` in SessionHeader. Subscribe to `get_supervisors` once on session activation; refresh on `Action::SupervisorChanged`.

## Frontend — pending injection indicator

The `BackgroundSession.incoming_message_pending: AtomicUsize` tracks how many messages from supervisors are queued. When > 0, SessionFooter renders `[N pending from supervisor]`.

Push update: emit `StreamChunk::SupervisorPendingInjection` whenever the count changes (already does — confirm).

## Cross-session message send

The TS frontend has "send to subordinate" UI. On the Rust side, add an action:

```rust
Action::SendToSubordinate { supervisor_id, subordinate_id, message: IncomingMessage } => {
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.receive_incoming_message(subordinate_id, message).await;
    });
}
```

UI trigger: a slash command `/send <subordinate_id> <message>` or a dedicated picker view. Audit TS for the exact UX.

## Acceptance criteria

1. All trait methods exist on `SessionManagerHandle`, `FspecService`, `FspecBackend`.
2. SessionHeader shows "Subordinate of:" badge when supervisor exists.
3. SessionFooter shows pending-injection count.
4. `add_supervisor` / `remove_supervisor` actually update the chain-of-command.
5. `receive_incoming_message` delivers a message into the subordinate's chunk stream.
6. Integration test in `codelet/fspec-tui/tests/supervisor_links.rs` creates two stub sessions, links them, sends a message, asserts delivery.

## Risks

- Loops in the supervisor graph (A → B → A) must be rejected by `add_supervisor`.
- `IncomingMessage` may carry image data → wire type must handle binary efficiently. Use `IncomingMessageImage` already in `codelet-rpc-types`.

## Out of scope

- Auto-promotion of subordinates on supervisor disconnect.
