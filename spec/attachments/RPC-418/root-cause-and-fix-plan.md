# RPC-418 — Rust TUI `/compact` lands on a no-op stub

## Summary

The `/compact` slash command in the Rust ratatui TUI (`codelet/fspec-tui`) is
**wired at the UI/RPC surface but functionally disconnected from the compaction
engine.** The command is registered, appears in the palette, and makes a full
RPC round-trip, but the RPC handler it ultimately reaches is an explicit
placeholder **stub** that returns a 1:1 no-op result. It never builds a DAG,
never pins/injects a summary, and never clears the conversation.

The **automatic** compaction path, by contrast, is fully implemented in the
Rust standalone binary. So the two paths have diverged — the *opposite* of the
TypeScript/Ink reference, where they converge on a single engine.

This card fixes the stub so manual `/compact` performs real in-view DAG
compaction, matching the NAPI reference and the automatic path.

---

## The dead-end (current behaviour)

**File:** `codelet/sessions/src/handle_impl.rs` (`compact_session`, ~line 261)

```rust
fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
    let uuid = uuid_from(session_id);
    match self.get_session(&uuid.to_string()) {
        Ok(session) => {
            // RPC-042 scope: minimal delegating impl. The real
            // compaction pipeline is wired in RPC-047. Here we
            // simply snapshot the current input-token count and
            // return a 1:1 compression-ratio placeholder ...
            let (input_tokens, _output, _reasoning) = session.get_tokens();
            Ok(CompactionResult {
                original_tokens: input_tokens,
                compacted_tokens: input_tokens,
                compression_ratio: 1.0,
                turns_summarized: 0,
                turns_kept: 0,
            })
        }
        Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
    }
}
```

It does **not** touch `compaction_in_progress`, does **not** call
`execute_compaction`, does **not** send `"Continue"`. A user's `/compact`
returns a well-formed but meaningless notice (0 turns summarized, ratio 1.0),
and the session is untouched.

> The "it's wired in RPC-047" comment is **misleading**. RPC-047 only wired the
> UI→RPC surface and tested it against a `MockBackend`, so it never exercises
> this real handle. The stub was never subsequently replaced. This is dead
> deferred work, not a temporary state that got completed.

---

## The call chain (UX plumbing is correct)

```
/compact typed in AgentView
  -> fspec-tui/src/views/agent/slash_commands.rs   (SlashCommandAction::Compact, line 57; palette 126-129)
  -> fspec-tui/src/app/dispatch_slash_commands.rs  (67-96: bare-session guard, spawn task,
                                                    route result via Action::EmitSessionNotice)
  -> backend.compact_session
       -> transport/embedded.rs:461  /  websocket.rs:746
       -> rpc/src/lib.rs:353 (RPC trait)
       -> lib.rs:1507 (server impl)
       -> SessionManagerHandle::compact_session   <-- THE STUB (handle_impl.rs:261)
```

Everything up to the handle is fine. Only the handle body is wrong.

---

## The reference contract (NAPI — the behaviour we must match)

**File:** `codelet/napi/src/session_bindings.rs` (`session_compact`, line 3038)

The NAPI free function (called by the Ink TUI's `/compact` → `useCompaction.ts`
→ `sessionCompact`) does, in order:

1. `let mut inner = session.inner.lock().await;`
2. If `inner.messages.is_empty()` → return error **"Nothing to compact - no messages yet"**.
3. `session.set_status(SessionStatus::Compacting);`
4. Capture `original_tokens = inner.token_tracker.input_tokens;` and
   `session.pre_compaction_tokens.store(original_tokens, Release);`
5. `execute_compaction(&mut inner, session.compaction_in_progress.clone(), None).await`
   — clears conversation to system-reminders, injects `COMPACTION_INSTRUCTION`,
   resets turns + token tracker. **`None`** = manual/agent-initiated (no resume prompt).
   On error: `set_compaction_progress(None)`, `set_status(Idle)`, return error.
6. `let compacted_tokens = inner.token_tracker.input_tokens;`
7. `drop(inner);` — release the lock **before** sending input; the agent loop needs it.
8. `session.set_compaction_progress(None);`
9. `session.send_input("Continue".to_string(), None)` — kicks the agent loop, which
   builds the hierarchical DAG via SessionSearch, calls the `inject_summary` tool,
   and `apply_pending_dag` folds it back in and clears again.
10. Return `CompactionResult { original_tokens, compacted_tokens,
    compression_ratio: compression_ratio(original, compacted) * 100.0,
    turns_summarized: 0, turns_kept: 0 }`.

> Automatic compaction hits the **same** `execute_compaction`, differing only in
> one argument: `Some(prompt)` (resume the in-flight message) vs `None` for manual.

(The NAPI version also emits `debug_capture` events `compaction.manual.start` /
`.failed` / `.complete`. These are optional-nice-to-have telemetry; port them if
straightforward, but they are not required for functional parity.)

---

## The engine already exists in the Rust crates

This is **not** a "the Rust port lacks compaction" situation. All machinery is
present and reachable from the `sessions` crate:

| Piece | Location | Notes |
|-------|----------|-------|
| `execute_compaction(&mut Session, Arc<AtomicBool>, Option<&str>) -> Result<()>` | `codelet/cli/src/interactive_helpers.rs:533` (`pub async fn`) | In-view DAG flow. No LLM calls, <5s, in-memory only. |
| `BackgroundSession.inner` | `codelet/sessions/src/background_session.rs:295` (`Arc<Mutex<codelet_cli::session::Session>>`) | tokio Mutex. |
| `compaction_in_progress` | `background_session.rs:415` (`Arc<AtomicBool>`) | Pass `.clone()` to `execute_compaction`. |
| `pre_compaction_tokens` | `background_session.rs:423` (`AtomicU32`) | `.store(v, Release)`. |
| `set_status` / `get_status` | `background_session.rs:823` / `818` | `SessionStatus::Compacting` / `Idle`. |
| `set_compaction_progress(Option<CompactionProgress>)` | `background_session.rs:1148` | Pass `None` after run. |
| `send_input(String, Option<String>) -> Result<(), String>` | `background_session.rs:1234` | Sends `"Continue"`. |
| `get_tokens() -> (u32, u32, Option<u32>)` | `background_session.rs:786` | Fallback token read. |

`codelet/sessions/Cargo.toml` already depends on `codelet-cli`, `codelet-core`,
and `codelet-tools`, so `execute_compaction` is directly importable
(`use codelet_cli::interactive_helpers::execute_compaction;`).

---

## The sync→async bridge (critical implementation detail)

`compact_session` is a **synchronous** trait method, but `execute_compaction` is
`async` and `session.inner` is a `tokio::sync::Mutex`. This method is invoked
from inside a tarpc dispatcher on a **multi-thread** tokio runtime.

Use the **same bridge pattern already established in this file** — do NOT invent
a new one. Two equivalent options already in `handle_impl.rs`:

**Option A — explicit block_in_place + block_on** (as used by
`restore_session_messages`, lines 470-484):

```rust
tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async {
        let mut inner = session.inner.lock().await;
        // ... check empty, set status, execute_compaction, capture tokens ...
    })
});
```

**Option B — the `loop_block_on` helper** (defined at `handle_impl.rs:1851`,
which already asserts multi-thread runtime + wraps `block_in_place`):

```rust
let result = loop_block_on(async move {
    let mut inner = session.inner.lock().await;
    // ...
});
```

**IMPORTANT ordering constraint:** `send_input("Continue")` must be called
*after* the `inner` lock is dropped (mirror the NAPI `drop(inner)` at line 3096).
Structure the bridge so the async block returns the captured token counts and
releases the lock, then call `session.set_compaction_progress(None)` and
`session.send_input(...)` outside/after the locked section.

The module header (lines 11-27) documents why `block_in_place` is required and
that a single-thread runtime will panic — respect that; tests must use a
multi-thread runtime (`#[tokio::test(flavor = "multi_thread")]`).

---

## Acceptance criteria (example-map seed)

**Rules:**
1. Calling `/compact` on a session with messages MUST clear the conversation to
   system-reminders and inject the compaction instruction (via `execute_compaction`).
2. Calling `/compact` on an empty session (no messages) MUST return an error
   ("Nothing to compact") and leave the session untouched.
3. After a successful compaction the handle MUST send `"Continue"` to the agent
   loop so DAG construction is kicked off.
4. The returned `CompactionResult` MUST report real pre/post token counts and a
   real compression ratio (NOT the hard-coded 1.0 placeholder).
5. `compaction_in_progress` / status transitions MUST match the NAPI reference
   (Compacting during, progress cleared + not stuck-Compacting after).
6. On `execute_compaction` error, status MUST revert to `Idle` and progress
   cleared, and the error propagated to the caller.
7. Session-not-found MUST still return `Err("Session not found: ...")`.

**Examples:**
- Session with 5 user/assistant messages → `/compact` → messages cleared to
  reminders + compaction instruction present; `send_input("Continue")` observed;
  `CompactionResult.original_tokens > 0`, `compacted_tokens < original_tokens`.
- Brand-new empty session → `/compact` → `Err` "Nothing to compact"; message
  count unchanged.
- Unknown session id → `/compact` → `Err` "Session not found: <id>".

---

## Testing guidance

- Test crate: `codelet/sessions/tests/` (new file, e.g.
  `rpc418_compact_session.rs`). Follow the structure of
  `rpc081_restore_session_messages.rs`:
  - Construct a fresh `SessionManager`, create a session via the trait's
    `create_session` bridge (uses `NoopSessionManagerHooks` → no agent loop
    spawned).
  - Seed `session.inner.lock().await.messages` with hand-crafted rig messages.
  - Drive `compact_session` through `SessionManagerHandle`.
  - Observe: inner messages after (cleared + instruction), broadcast
    StreamChunks (`chunks_rx()` / `subscribe_to_stream`), and whether
    `send_input` fired (status → Running, or a buffered user_input chunk).
  - Use the `DATA_DIR_GUARD` serialization pattern from rpc081 if the test
    touches the process-global data directory.
- Use `#[tokio::test(flavor = "multi_thread")]` — `block_in_place` panics on a
  single-thread runtime.
- Every Gherkin step needs a matching `@step` comment in the test.

## Build / verify

```
cargo build -p codelet-sessions
cargo test  -p codelet-sessions rpc418
cargo clippy -p codelet-sessions
```

## Out of scope

- No changes to the UI/RPC surface (slash command, dispatch, transport, RPC
  trait, server impl) — those are already correct.
- No changes to `execute_compaction` itself or the agent-loop DAG machinery.
- Removing/deprecating the NAPI path — it stays as the reference.
</content>
