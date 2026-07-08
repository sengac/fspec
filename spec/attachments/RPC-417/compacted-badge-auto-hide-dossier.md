# RPC-417 — COMPACTED header badge auto-hide (TS parity gap)

**Type:** Bug (parity defect)
**Epic:** rust-frontend
**Related:** RPC-100 (introduced the COMPACTED badge), RPC-099 (per-session tokens), RPC-416 (reconnect notice timer — the pattern to copy)

---

## 1. Problem Statement

In the **TypeScript / Ink** TUI the `SessionHeader` top-right context-fill bracket
switches from `[X%]` to `[X%: COMPACTED Y%]` right after a compaction completes, and
then **auto-hides after 10 seconds**, reverting to the plain `[X%]` form.

In the ported **Rust / ratatui** frontend (`fspec-tui`), RPC-100 correctly renders the
`[X%: COMPACTED Y%]` suffix, but there is **no auto-hide timer**. The badge stays pinned
until the user runs `/clear` (`SessionStateChange::Cleared`). This is a behavioural
regression from the TS original.

**Goal:** add the missing 10-second per-session auto-hide, matching TS TUI-044, using the
codebase's idiomatic timer pattern.

---

## 2. TypeScript Reference (source of truth)

DeepSearch-confirmed. Feature tag **TUI-044**.

### State — `src/tui/components/AgentView.tsx:1333-1336`
```tsx
// TUI-044: Compaction notification indicator (shows in percentage indicator for 10 seconds)
const [compactionReduction, setCompactionReduction] = useState<number | null>(null);
```

### Auto-hide effect — `src/tui/components/AgentView.tsx:1459-1466`
```tsx
// TUI-044: Hide compaction notification after 10 seconds
useEffect(() => {
  if (compactionReduction === null) return;
  const timeout = setTimeout(() => {
    setCompactionReduction(null);
  }, 10000);
  return () => clearTimeout(timeout);
}, [compactionReduction]);
```

Key properties of the TS behaviour:
- Duration is a hard-coded **10000 ms (10 s)** literal.
- The effect re-runs whenever `compactionReduction` changes → a **new** compaction
  **resets** the 10 s window (the cleanup `clearTimeout` cancels the previous timer).
- On unmount / value change, the pending timer is cleared (no stale hide).

### Renderer — `src/tui/components/SessionHeader.tsx:129-132`
```tsx
const percentText = compactionReduction !== null
  ? `[${formatPercentage(contextFillPercentage)}%: COMPACTED ${formatPercentage(Math.abs(compactionReduction))}%]`
  : `[${formatPercentage(contextFillPercentage)}%]`;
```

The sibling **TUI-031** (tokens/sec indicator) uses the exact same 10 s `setTimeout` pattern.

---

## 3. Rust Current State (RPC-100)

The badge is a per-frame render of the store's per-session `compaction_reduction`:

| Concern | File | Notes |
|---|---|---|
| Bracket text | `src/views/agent/header_build.rs::build_right_line` | `Some(r) => "[{pct}%: COMPACTED {r.abs()}%]"` |
| Widget field | `src/views/agent/header.rs` (`compaction_reduction: Option<i32>`) | |
| Per-frame source | `src/views/agent/chrome_paint.rs` | `store.compaction_reduction_for(s)` |
| Store accessors | `src/store/agent_view/chrome_state.rs:80-101` | `compaction_reduction_for / set_compaction_reduction / clear_compaction_reduction` |
| Set on complete | `src/app/dispatch_stream_chunks.rs:126-150` | `set_compaction_reduction(...)` after computing reduction |
| Clear on `/clear` | `src/app/dispatch_stream_chunks.rs:69-77` | `SessionState::Cleared` → `clear_compaction_reduction` |

**The gap:** nothing arms a timer after `set_compaction_reduction`. The badge only clears
on `Cleared`.

---

## 4. Idiomatic Timer Pattern (copy this) — `src/app/dispatch_reconnect.rs`

The codebase does **not** poll `Instant::elapsed()` in render/tick for dismissals. The
established pattern (RPC-416 reconnect notice, also `notification_dialog.rs`,
`status_dialog.rs`) is **spawn → sleep → self-addressed Action + seq-guard + abort**:

```rust
const DISMISS_DELAY: Duration = Duration::from_millis(1500);

fn arm_reconnect_dismiss(&mut self, session_id: SessionId, seq: u64) {
    self.abort_reconnect_dismiss();                 // cancel any prior timer
    let action_tx = self.action_tx.clone();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(DISMISS_DELAY).await;
        let _ = action_tx.send(Action::ClearReconnectNotice { session_id, seq });
    });
    self.reconnect_dismiss_handle = Some(handle);
}

fn abort_reconnect_dismiss(&mut self) {
    if let Some(handle) = self.reconnect_dismiss_handle.take() { handle.abort(); }
}

pub(crate) fn handle_clear_reconnect_notice(&mut self, session_id: &SessionId, seq: u64) {
    if self.reconnect_notice.as_ref() != Some(&(session_id.clone(), seq)) { return; } // superseded → no-op
    if let Some(ctx) = self.agent_view_store.session_context_mut_for(session_id) {
        ctx.remove_notice_by_seq(seq);
    }
    self.reconnect_notice = None;
    self.reconnect_dismiss_handle = None;
}
```

The `tokio::select!` run-loop in `src/app/events.rs` receives the fired `Action` on the
`action_rx.recv()` arm and routes it through `dispatch()`. The 16 ms tick arm is render-only.

**Difference for compaction:** reconnect tracks a **single** active notice
(`Option<(SessionId, seq)>`). Compaction badges are **per-session** (multiple sessions can
show COMPACTED at once), so the seq must be tracked **per session** (a `HashMap`), and
timer handles must be per-session too.

---

## 5. Implementation Plan

### 5.1 Action variant
Add to the `Action` enum (`src/components/…`):
```rust
ClearCompactionReduction { session_id: SessionId, seq: u64 },
```

### 5.2 Store — per-session seq (`store/agent_view.rs` + `store/agent_view/chrome_state.rs`)
Add field:
```rust
compaction_reduction_seq_by_session: HashMap<SessionId, u64>,
```
Accessors:
- `compaction_reduction_seq_for(&SessionId) -> u64` (default 0)
- `bump_compaction_reduction_seq(SessionId) -> u64` — increment & return the **new** seq.

`clear_compaction_reduction(&SessionId)` MUST **also bump the seq** so a still-pending
timer becomes stale (covers the `/clear`-before-10s case). Alternatively make `Cleared`
call `bump_compaction_reduction_seq` explicitly — either way the invariant is: after a
clear, the old seq no longer matches.

> Keep `agent_view.rs` and `chrome_state.rs` **under 300 LoC** (RPC-024/025 source-shape
> tests enforce this). If adding accessors pushes over, split into a new sub-module.

### 5.3 Arm on CompactionComplete (`src/app/dispatch_stream_chunks.rs`)
In the `StreamChunk::CompactionComplete` branch, after the existing
`set_compaction_reduction(...)`:
```rust
let seq = self.agent_view_store.bump_compaction_reduction_seq(session_id.clone());
self.arm_compaction_hide(session_id.clone(), seq);
```

### 5.4 Arm / handle helpers (new `src/app/dispatch_compaction_hide.rs`)
```rust
const COMPACTION_HIDE_DELAY: Duration = Duration::from_secs(10);

impl App {
    pub(crate) fn arm_compaction_hide(&mut self, session_id: SessionId, seq: u64) {
        // Runtime guard — mirror spawn_fspec_command_runner so synchronous
        // #[test] paths (RPC-100 tests) don't panic on tokio::spawn.
        if tokio::runtime::Handle::try_current().is_err() { return; }
        if let Some(prev) = self.compaction_hide_handles.remove(&session_id) { prev.abort(); }
        let action_tx = self.action_tx.clone();
        let sid = session_id.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(COMPACTION_HIDE_DELAY).await;
            let _ = action_tx.send(Action::ClearCompactionReduction { session_id: sid, seq });
        });
        self.compaction_hide_handles.insert(session_id, handle);
    }

    pub(crate) fn handle_clear_compaction_reduction(&mut self, session_id: &SessionId, seq: u64) {
        // seq-guard: only clear if this timer is still current.
        if self.agent_view_store.compaction_reduction_seq_for(session_id) != seq { return; }
        self.agent_view_store.clear_compaction_reduction(session_id); // NOTE: this bumps seq again — fine
        self.compaction_hide_handles.remove(session_id);
    }
}
```
> ⚠️ Careful: if `clear_compaction_reduction` bumps the seq, calling it inside the handler
> is fine (badge already cleared). But do NOT let the handler's own clear invalidate a
> *different* legitimately-armed timer for the same session — because we checked seq match
> first, the only armed timer with this seq is the one that just fired. Verify this
> interaction in a test (Rule: seq-guard).

### 5.5 App state field (`src/app/state.rs`)
```rust
compaction_hide_handles: HashMap<SessionId, tokio::task::JoinHandle<()>>,
```
Initialise empty in `App::new`.

### 5.6 Route the Action (`src/app/dispatch.rs`)
```rust
Action::ClearCompactionReduction { session_id, seq } =>
    self.handle_clear_compaction_reduction(&session_id, seq),
```

---

## 6. Testing Strategy

New test file: `codelet/fspec-tui/tests/agentview_compaction_badge_auto_hide_rpc417.rs`
(header comment must reference the feature file). Reuse helpers from RPC-100 test
(`agentview_session_header_compaction_percentage_rpc100.rs`) — `agent_app_with_single_session`,
`agent_app_with_two_sessions`, `render_app_buffer`, `header_text`, `drain_actions`, etc.

Because the timer uses `tokio::spawn` + `sleep`, tests must run under a tokio runtime.
Use **`#[tokio::test(start_paused = true)]`** so virtual time can be advanced without
real-world waiting:

- **Real-timer scenario:** dispatch `CompactionComplete`, assert `COMPACTED` present;
  `tokio::time::advance(Duration::from_secs(10)).await`; `tokio::task::yield_now().await`;
  `drain_actions`; render; assert badge gone (`[80%]`, no `COMPACTED`).
- **Seq-guard scenario:** two `CompactionComplete` for s-1 in quick succession; fire the
  stale `Action::ClearCompactionReduction{seq:0}` directly; assert newer badge survives.
- **Per-session scenario:** s-1 and s-2 both compacted; fire s-1's clear; assert s-1 plain,
  s-2 still `COMPACTED`.
- **/clear-before-10s scenario:** compaction then `SessionStateChange::Cleared`; badge
  gone immediately (`[0%]`); firing the original timer action is a no-op (no panic).
- **Handler no-op scenario (direct):** `handle_clear_compaction_reduction` with a
  mismatched seq leaves the badge intact.

Every Gherkin step needs a matching `// @step` comment (link-coverage enforcement).

⚠️ **Regression guard:** the existing RPC-100 tests run as plain `#[test]` (no runtime) and
dispatch `CompactionComplete`. The runtime guard in `arm_compaction_hide` must make those a
no-op (no panic). Run the RPC-100 test file after implementing to confirm it stays green.

---

## 7. Definition of Done

- [ ] Feature file `spec/features/agentview-compaction-badge-auto-hide.feature` with `@RPC-417` tag, no placeholders.
- [ ] All scenarios have failing tests first (red), then pass (green).
- [ ] `arm_compaction_hide` runtime-guarded; RPC-100 tests still pass.
- [ ] `cargo build -p codelet-fspec-tui` clean; `cargo test` for the new + RPC-100 files pass.
- [ ] No file over 300 LoC; no new `unwrap()/todo!()/unimplemented!()` in production code.
- [ ] Coverage linked (test + impl line ranges) for every scenario.
- [ ] `fspec validate` + `fspec validate-tags` clean.
