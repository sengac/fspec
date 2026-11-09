# VTCode Message Queuing & Cooperative Agent Loop Interruption

## Reference Implementation Analysis

**Source**: https://github.com/vinhnx/VTCode (cloned to `/tmp/VTCode`)
**Analyzed**: 2026-04-14
**Purpose**: Blueprint for implementing message queuing and agent loop interruption in codelet/fspec

---

## Executive Summary

VTCode uses a **three-layer interrupt and message queuing architecture** built on Tokio async primitives. There is no single "message queue" — instead, **three cooperating mechanisms** handle different aspects of message queuing, interrupt delivery, and loop continuation:

1. **`CtrlCState`** — atomic state machine for Ctrl+C cancel/exit signals
2. **`InlineQueueState` (`VecDeque<String>`)** — in-process message queue for user inputs submitted while the agent is busy
3. **`RuntimeSteering` + `SteeringMessage` channel** — `tokio::sync::mpsc::UnboundedReceiver` for external control signals (pause/resume/stop/follow-up input injection)

---

## 1. Message Queue Data Structures

### A. The Input Queue — `VecDeque<String>` (`queued_inputs`)

**File**: `src/agent/runloop/unified/turn/session_loop_runner/mod.rs` (line ~849)

```rust
let mut queued_inputs: VecDeque<String> = VecDeque::with_capacity(8);
```

This is a plain `VecDeque<String>` threaded through the entire run loop via mutable references. The `InlineQueueState` wrapper provides smart dequeue semantics:

**File**: `src/agent/runloop/unified/turn/session/inline_events/queue.rs`

```rust
pub(crate) struct InlineQueueState<'a> {
    handle: &'a InlineHandle,
    queued_inputs: &'a mut VecDeque<String>,
    prefer_latest_once: &'a mut bool,
}
```

**Enqueue paths**:
- **`queue_submit()`** (from `InlineEvent::QueueSubmit`): `push_back()` + sets `prefer_latest_once = true`
- **Scheduled tasks**: `queued_inputs.push_back(task.prompt)` in the interaction loop runner
- **Steering follow-ups**: `RuntimeSteering::queue_follow_up_input()` queues into a separate `VecDeque<String>`

**Dequeue semantics** — `take_next_submission()`:

```rust
fn take_next_submission(&mut self) -> Option<String> {
    if *self.prefer_latest_once {
        *self.prefer_latest_once = false;
        self.queued_inputs.pop_back()   // newest-first (once!)
    } else {
        self.queued_inputs.pop_front()  // FIFO thereafter
    }
}
```

**Key Design Insight**: This **one-shot latest-first** behavior ensures the most recently typed message gets processed first (e.g., when the user hits Enter while the agent is mid-turn, the newest message is what they care about most), then reverts to FIFO for any older queued messages.

### B. The Steering Channel — `tokio::sync::mpsc::UnboundedSender<SteeringMessage>`

**File**: `src/agent/runloop/unified/turn/session_loop_runner/mod.rs` (line ~687-689)

```rust
let steering_sender = if steering_receiver.is_none() {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    *steering_receiver = Some(receiver);
    Some(sender)
} else { None };
```

```rust
pub enum SteeringMessage {
    SteerStop,                  // Immediate stop
    Pause,                      // Pause the loop
    Resume,                     // Resume after pause
    FollowUpInput(String),      // Inject input after current turn
}
```

**File**: `src/agent/runloop/unified/session_setup/ui.rs` (line ~112-128)

The sender is passed into the TUI event callback:

```rust
Arc::new(move |event: &InlineEvent| match event {
    InlineEvent::Interrupt => {
        let _ = request_local_stop(&state, &notify);
    }
    InlineEvent::Pause => {
        let _ = sender.send(SteeringMessage::Pause);
    }
    InlineEvent::Steer(text) => {
        let _ = sender.send(SteeringMessage::FollowUpInput(text.clone()));
    }
    _ => {}
})
```

The receiver lives inside `RuntimeSteering`:

```rust
pub struct RuntimeSteering {
    steering_receiver: Option<UnboundedReceiver<SteeringMessage>>,
    queued_follow_up_inputs: VecDeque<String>,
}
```

### C. The TUI Event Channel — `tokio::sync::mpsc::UnboundedSender<InlineEvent>`

The TUI processes keystrokes into `InlineEvent` variants and sends them through an unbounded channel to the agent loop:

```rust
pub enum InlineEvent {
    Submit(String),          // Direct submission (user hits Enter while idle)
    QueueSubmit(String),     // Queue for later (Enter while agent is running)
    Steer(String),           // Inject as FollowUpInput via steering channel
    ProcessLatestQueued,     // Promote latest queued to run next
    EditQueue,               // Pop latest from queue into input buffer
    Interrupt,               // Ctrl+C signal
    Pause,                   // Pause signal
    Resume,                  // Resume signal
    Cancel,                  // Cancel (no active run)
    Exit,                    // Exit
    // ...
}
```

---

## 2. The Nested Run Loop — Three Levels of Interrupt Checking

The architecture is a **nested loop with three levels**, each with its own interrupt checking:

### Level 1: Session Loop

**File**: `src/agent/runloop/unified/turn/session_loop_runner/mod.rs` (line ~887)

```rust
loop {
    // 1. Check for follow-up inputs from steering
    if let Some(input) = runtime.run_until_idle() {
        // Skip interaction loop, go straight to turn execution
        InteractionOutcome::Continue { input, ... }
    } else {
        // 2. Run the interaction loop (poll for user input)
        run_interaction_loop(ctx, state).await
    }
    
    // 3. Execute the turn (with timeout wrapper)
    timeout(Duration::from_secs(timeout_secs),
        run_turn_loop(&mut working_history, turn_loop_ctx)
    ).await
}
```

`runtime.run_until_idle()` pops from `RuntimeSteering::queued_follow_up_inputs` — these are messages injected via `SteeringMessage::FollowUpInput` during a previous turn. If any exist, the loop **skips the interaction loop entirely** and starts a new turn immediately.

### Level 2: Interaction Loop (Idle Polling)

**File**: `src/agent/runloop/unified/turn/session/interaction_loop_runner.rs` (line ~794)

On each iteration:

```rust
let inline_action = poll_inline_loop_action(session, ctrl_c_notify, resources).await?;
```

**File**: `src/agent/runloop/unified/turn/session/inline_events/driver.rs` (line ~101)

Inside `poll_inline_loop_action`:

```rust
async fn poll(&mut self, session, ctrl_c_notify) -> Result<InlineLoopAction> {
    // 1. Check for pending interrupt notice
    if let Some(action) = self.ensure_interrupt_notice()? { return Ok(action); }
    
    // 2. Drain queued submissions FIRST (before waiting for events)
    if let Some(action) = self.take_queued_submission() { return Ok(action); }
    
    // 3. Wait for next event OR ctrl_c OR timeout
    let maybe_event = tokio::select! {
        biased;
        event = session.next_event() => event,
        _ = ctrl_c_notify.notified() => None,
        _ = tokio::time::sleep(idle_wake_delay) => None,
    };
    
    // 4. Check exit condition
    if let Some(action) = self.exit_action() { return Ok(action); }
    
    // 5. Check interrupt notice again
    if let Some(action) = self.ensure_interrupt_notice()? { return Ok(action); }
    
    // 6. Process the event
    context.process_event(event, &mut self.queue).await
}
```

The `biased` `tokio::select!` ensures `session.next_event()` takes priority over the timeout, and `ctrl_c_notify` can wake the select even when no user event arrives.

### Level 3: Turn Loop (At Top of Every Step)

**File**: `src/agent/runloop/unified/turn/turn_loop.rs` (line ~543)

```rust
loop {
    // Check for steering messages (pause, stop, follow-up injection)
    if handle_steering_messages(&mut ctx, working_history, &mut result).await? {
        break;  // Turn was cancelled or stopped
    }
    
    // ... build and execute LLM request ...
    // ... handle tool calls ...
}
```

`handle_steering_messages` does a non-blocking `try_recv()` on the steering channel:

```rust
while let Ok(message) = receiver.try_recv() {
    pending.push(message);
}
if pending.iter().any(|m| matches!(m, SteeringMessage::SteerStop)) {
    cancel_for_steering_stop(tool_registry, result).await;
    break Ok(true);
}
// Handle Pause/Resume/FollowUpInput...
for message in pending {
    if let SteeringMessage::FollowUpInput(input) = message {
        queue_follow_up_input(renderer, ctx.runtime_steering, input)?;
    }
}
```

---

## 3. Mechanism for Interrupting In-Progress LLM Generation / Tool Execution

### Ctrl+C Interrupt Path

**File**: `src/agent/runloop/unified/session_setup/signal.rs` (line ~29)

1. **OS signal** → `spawn_signal_handler` → calls `request_local_stop(&ctrl_c_state, &ctrl_c_notify)`
2. **TUI keypress** → `InlineEvent::Interrupt` → event callback → same `request_local_stop()`

**File**: `src/agent/runloop/unified/turn/session_loop_runner/stop_requests.rs`

```rust
pub(crate) fn request_local_stop(
    ctrl_c_state: &Arc<CtrlCState>,
    ctrl_c_notify: &Arc<Notify>,
) -> CtrlCSignal {
    let signal = ctrl_c_state.register_signal();  // atomic state transition
    ctrl_c_notify.notify_waiters();                // wake all select! branches
    signal
}
```

### How the LLM Generation is Interrupted

**File**: `src/agent/runloop/unified/turn/turn_processing/llm_request/mod.rs` (line ~360-378)

The LLM request runs inside a `tokio::select!` that races against `ctrl_c_notify`:

**Non-streaming mode**:

```rust
loop {
    let cancel_notifier = ctx.ctrl_c_notify.notified();
    tokio::pin!(cancel_notifier);
    
    let outcome = tokio::select! {
        res = &mut generate_future => Some(res),
        _ = &mut cancel_notifier => {
            Some(Err(interrupted_provider_error(&turn_snapshot.provider_name)))
        }
        _ = &mut keepalive_sleep => None,
    };
    // ...
}
```

When `ctrl_c_notify.notify_waiters()` fires, the `cancel_notifier` branch wins the select, returning an `interrupted_provider_error`. The LLM future is **dropped** (Tokio cancels it), and control flows back up.

**During retry backoff** (line ~188-198):

```rust
tokio::select! {
    _ = tokio::time::sleep(delay) => {}
    _ = &mut cancel_notifier => {
        if ctrl_c_state.is_cancel_requested() || ctrl_c_state.is_exit_requested() {
            llm_result = Err(interrupted_provider_error(...));
            break;
        }
    }
}
```

### How Tool Execution is Interrupted

After a turn is cancelled/exited, all PTY sessions are force-terminated:

```rust
if matches!(result, TurnLoopResult::Cancelled | TurnLoopResult::Exit) {
    ctx.tool_registry.terminate_all_exec_sessions_async().await;
}
```

**File**: `src/agent/runloop/unified/turn/tool_outcomes/handlers_batch.rs` (line ~124)

For parallel tool groups:

```rust
async fn interrupt_parallel_group<F>(/* ... */) -> /* ... */ {
    // Drains pending futures and terminates exec sessions
}
```

---

## 4. The CtrlCState State Machine

**File**: `vtcode-core/src/core/agent/state.rs` (line ~603-710)

```
Idle → CancelRequested → ExitArmed → ExitRequested
        (1st Ctrl+C)     (handled)    (2nd Ctrl+C within 1s)
```

### Single Ctrl+C (Cancel)

1. First Ctrl+C → `CtrlCPhase::CancelRequested` → `CtrlCSignal::Cancel`
2. `InlineInterruptCoordinator::ensure_notice_displayed()` shows: *"Interrupt received. Stopping task..."*
3. Turn loop catches the cancel → `TurnLoopResult::Cancelled`
4. Back in session loop → `mark_cancel_handled()` → `CtrlCPhase::ExitArmed`
5. History is **rolled back** via `turn_history_checkpoint.rollback()`
6. The session loop `continue`s → goes back to interaction loop → **waits for next user input**
7. When user types next input → `ctrl_c_state.reset()` → back to `Idle`

### Double Ctrl+C (Exit)

If second Ctrl+C fires within 1 second of `ExitArmed`:
- `CtrlCPhase::ExitRequested` → `CtrlCSignal::Exit`
- `TurnLoopResult::Exit` → `SessionEndReason::Exit`
- Loop breaks entirely

### Atomic Implementation Details

```rust
struct CtrlCState {
    phase: AtomicU8,        // Current phase (Idle/CancelRequested/ExitArmed/ExitRequested)
    armed_at: AtomicU64,    // Timestamp when ExitArmed entered (for 1s window)
}

impl CtrlCState {
    fn register_signal(&self) -> CtrlCSignal {
        let current = self.phase.load(Ordering::Acquire);
        match current {
            IDLE => {
                self.phase.store(CANCEL_REQUESTED, Ordering::Release);
                CtrlCSignal::Cancel
            }
            EXIT_ARMED => {
                let armed = self.armed_at.load(Ordering::Acquire);
                if now - armed < 1_000_000_000 { // 1 second in nanos
                    self.phase.store(EXIT_REQUESTED, Ordering::Release);
                    CtrlCSignal::Exit
                } else {
                    // Armed window expired, treat as new cancel
                    self.phase.store(CANCEL_REQUESTED, Ordering::Release);
                    CtrlCSignal::Cancel
                }
            }
            _ => CtrlCSignal::AlreadyRequested,
        }
    }

    fn mark_cancel_handled(&self) {
        self.phase.store(EXIT_ARMED, Ordering::Release);
        self.armed_at.store(now_nanos(), Ordering::Release);
    }

    fn reset(&self) {
        self.phase.store(IDLE, Ordering::Release);
    }
}
```

---

## 5. How the Loop Resumes After Interruption — Three Paths

### Path A — Cancel (Ctrl+C)

1. Turn loop breaks with `TurnLoopResult::Cancelled`
2. PTY sessions force-terminated: `tool_registry.terminate_all_exec_sessions_async().await`
3. History rolled back: `turn_history_checkpoint.rollback()`
4. Session loop `continue`s → back to interaction loop → waits for user
5. When user types next input → `ctrl_c_state.reset()` → back to `Idle`

### Path B — Follow-Up Input (Steering)

1. `SteeringMessage::FollowUpInput("...")` arrives during turn
2. `handle_steering_messages` drains it via `try_recv()` → `queue_follow_up_input()` → `RuntimeSteering::queued_follow_up_inputs.push_back()`
3. Turn completes **normally** (not cancelled)
4. Back at top of session loop: `runtime.run_until_idle()` pops the queued input
5. New turn starts **immediately** without waiting for user interaction

### Path C — Queued TUI Input

1. User types + Enter while agent is running → `InlineEvent::QueueSubmit(text)`
2. `InlineEventContext::process_event` → `queue_submit()` → `queue.push(text)` → returns `InlineLoopAction::Continue` (stays in turn)
3. After turn completes, session loop returns to interaction loop
4. `poll_inline_loop_action` → `take_queued_submission()` fires **before** waiting for events → immediately returns `InlineLoopAction::Submit(text)`
5. New turn starts with the queued input

---

## 6. Complete Signal/Channel/Token Summary

| Mechanism | Type | Purpose |
|-----------|------|---------|
| `CtrlCState` | `AtomicU8` + `AtomicU64` | Lock-free cancel/exit state machine with debounce + 1s escalation window |
| `ctrl_c_notify` | `Arc<tokio::sync::Notify>` | Wakes all `tokio::select!` branches when Ctrl+C fires |
| `queued_inputs` | `VecDeque<String>` (borrowed) | In-process queue for TUI-submitted inputs while agent is busy |
| `SteeringMessage` channel | `tokio::sync::mpsc::unbounded_channel` | External control: pause/resume/stop/follow-up injection |
| `RuntimeSteering::queued_follow_up_inputs` | `VecDeque<String>` | Follow-up inputs extracted from steering, consumed by `run_until_idle()` |
| `InlineEvent` channel | `tokio::sync::mpsc::unbounded_channel` | TUI keypress → agent loop event delivery |
| `InlineEventCallback` | `Arc<dyn Fn(&InlineEvent)>` | Synchronous callback fired on every TUI event for immediate side-effects |
| `CancellationToken` | `tokio_util::sync::CancellationToken` | Graceful shutdown of the signal handler task |
| `tokio::time::timeout` | Turn-level timeout | Wraps entire `run_turn_loop` with configurable timeout |

---

## 7. Key Design Principles

### Cooperative Cancellation

The system **never cancels an async task mid-execution by dropping it arbitrarily**. Instead:

1. `ctrl_c_notify` **wakes** `tokio::select!` branches where the LLM future is racing
2. The cancel branch supersedes the LLM branch, which returns an error
3. The error propagates through normal turn loop error handling
4. The turn loop breaks cleanly with `Cancelled` or `Exit`
5. PTY sessions are explicitly terminated
6. History is rolled back to the checkpoint
7. Control returns to the interaction loop normally

**No half-written state is left behind.**

### Newest-First Then FIFO

The `prefer_latest_once` flag ensures the most recently submitted message gets priority (it's what the user is thinking about NOW), then any older queued messages drain in order.

### Separation of Concerns

- **CtrlCState**: Hard interrupt (cancel/exit) — atomic, lock-free, signal-safe
- **InlineQueueState**: Soft input queuing — synchronous, no channel overhead
- **RuntimeSteering**: External control — async channel for pause/resume/follow-up from non-TUI sources (bridges, APIs)

---

## 8. VTCode Source File Index

### Core Agent Loop Files

| File | Purpose |
|------|---------|
| `src/agent/runloop/unified/turn/session_loop_runner/mod.rs` | **Top-level session loop** — orchestrates interaction loop, turn loop, steering drain, queued input allocation |
| `src/agent/runloop/unified/turn/session_loop_runner/tests.rs` | Tests for session loop runner |
| `src/agent/runloop/unified/turn/session/interaction_loop_runner.rs` | **Interaction loop** — idle polling for user input, scheduled task dispatch |
| `src/agent/runloop/unified/turn/session/inline_events/driver.rs` | **InlineLoopAction poll driver** — biased select!, queued submission drain, interrupt notice |
| `src/agent/runloop/unified/turn/session/inline_events/queue.rs` | **InlineQueueState** — smart dequeue with prefer_latest_once |
| `src/agent/runloop/unified/turn/turn_loop.rs` | **Turn loop** — LLM request + tool execution cycle with steering checks |

### Interrupt & Signal Files

| File | Purpose |
|------|---------|
| `vtcode-core/src/core/agent/state.rs` | **CtrlCState** — atomic state machine (Idle → CancelRequested → ExitArmed → ExitRequested) |
| `src/agent/runloop/unified/session_setup/signal.rs` | **Signal handler** — OS signal → `request_local_stop` |
| `src/agent/runloop/unified/turn/session_loop_runner/stop_requests.rs` | **`request_local_stop()`** — atomic transition + notify_waiters |

### Steering & External Control Files

| File | Purpose |
|------|---------|
| `src/agent/runloop/unified/session_setup/ui.rs` | **UI event callback** — maps InlineEvent → SteeringMessage via sender |
| `src/agent/runloop/unified/turn/turn_processing/llm_request/mod.rs` | **LLM request with cancel racing** — tokio::select! against ctrl_c_notify |

### TUI Input Queue Files

| File | Purpose |
|------|---------|
| `vtcode-tui/src/core_tui/session/state.rs` | **TUI session state** — input buffer, queue display |
| `vtcode-tui/src/core_tui/session/tests/queue_inputs.rs` | **Queue input tests** — 577 lines of comprehensive queue behavior tests |
| `vtcode-tui/src/core_tui/app/session/events.rs` | **TUI event processing** — keystroke → InlineEvent mapping |

### Tool Execution Interrupt Files

| File | Purpose |
|------|---------|
| `src/agent/runloop/unified/turn/tool_outcomes/execution_result.rs` | **Tool execution result handling** |
| `src/agent/runloop/unified/turn/tool_outcomes/error_handling.rs` | **Error handling for tool outcomes** |
| `src/agent/runloop/unified/turn/tool_outcomes/handlers/mod.rs` | **Tool handlers dispatch** — includes batch interrupt |
| `src/agent/runloop/unified/turn/guards.rs` | **Turn guards** — pre/post turn safety checks |

---

## 9. Test Files for Reference

| Test File | Lines | What It Tests |
|-----------|-------|---------------|
| `vtcode-tui/src/core_tui/session/tests/queue_inputs.rs` | 577 | Queue submit, edit queue, process latest queued, prefer_latest_once behavior |
| `src/agent/runloop/unified/turn/session_loop_runner/tests.rs` | 506 | Session loop runner with steering, follow-up inputs, turn execution |
| `vtcode-core/src/core/agent/runner/tests.rs` | 581 | Agent runner state transitions, cancellation, error recovery |

---

## 10. Adaptation Notes for Codelet/fspec

### What We Have Already
- Agent loop in Rust (codelet-core)
- Session management with multiple concurrent sessions
- TUI with Ink/React (codelet-tui)
- NAPI bindings bridging Rust ↔ TypeScript
- ESC-based interruption (basic, via `BASH_ABORT_FLAG`)

### What We Need to Build

1. **`CtrlCState` equivalent** — Replace the global `BASH_ABORT_FLAG: AtomicBool` with a proper per-session state machine (Idle → CancelRequested → ExitArmed → ExitRequested) that supports the 1-second double-tap escalation pattern

2. **`InlineQueueState` equivalent** — Per-session `VecDeque<String>` with `prefer_latest_once` smart dequeue, exposed via NAPI so the TUI can enqueue messages while the agent is running

3. **`RuntimeSteering` equivalent** — Per-session `tokio::sync::mpsc::unbounded_channel` for external control (bridges, scheduled tasks, subordinate agents) to inject follow-up inputs, pause, resume, or stop the agent loop

4. **Turn-level interrupt checking** — At the top of every turn loop iteration, non-blocking `try_recv()` on the steering channel to check for pause/stop/follow-up

5. **LLM request cancellation** — Race the LLM streaming future against a `tokio::sync::Notify` so Ctrl+C/ESC can immediately cancel in-flight requests

6. **History rollback on cancel** — Checkpoint the conversation history before each turn; rollback on cancel so the user can retry cleanly

7. **NAPI bridge for queue operations** — `queue_input(session_id, text)`, `cancel_session(session_id)`, `get_queue_depth(session_id)` bindings for the TUI

### Key Differences from VTCode

- VTCode is pure Rust CLI; we have Rust + TypeScript TUI via NAPI
- Our "steering" sources include: TUI keyboard, Bridge WebSocket, AgentManager messages, Schedule triggers
- We already have per-session isolation (fixed BUG-126 through BUG-129) — the queue must be per-session too
- Our compaction system (CMPCT-*) needs to be interrupt-aware

---

## 11. Data Flow Diagrams

### Message Queuing Flow

```
User types message while agent is running
    ↓
TUI detects agent is busy
    ↓
Emits InlineEvent::QueueSubmit(text)
    ↓
InlineQueueState.queue_submit()
    → queued_inputs.push_back(text)
    → prefer_latest_once = true
    ↓
Agent turn completes normally
    ↓
Session loop returns to interaction loop
    ↓
poll_inline_loop_action()
    → take_queued_submission() [checked BEFORE waiting for events]
    → prefer_latest_once? pop_back() : pop_front()
    ↓
Returns InlineLoopAction::Submit(text)
    ↓
New turn starts immediately with queued input
```

### Ctrl+C Interrupt Flow

```
User presses Ctrl+C (or ESC in our case)
    ↓
request_local_stop()
    → ctrl_c_state.register_signal() [atomic: Idle → CancelRequested]
    → ctrl_c_notify.notify_waiters() [wakes all select! branches]
    ↓
LLM request's tokio::select!
    → cancel_notifier branch wins
    → returns interrupted_provider_error
    ↓
Turn loop catches error
    → result = TurnLoopResult::Cancelled
    ↓
Session loop
    → terminate_all_exec_sessions_async()
    → turn_history_checkpoint.rollback()
    → mark_cancel_handled() [CancelRequested → ExitArmed]
    → continue (back to interaction loop)
    ↓
User types new input
    → ctrl_c_state.reset() [ExitArmed → Idle]
    → New turn starts
```

### Steering Follow-Up Flow

```
External source (Bridge, AgentManager, Schedule) sends follow-up
    ↓
steering_sender.send(SteeringMessage::FollowUpInput(text))
    ↓
Turn loop's handle_steering_messages()
    → try_recv() drains channel
    → queue_follow_up_input() → RuntimeSteering.queued_follow_up_inputs.push_back()
    ↓
Turn completes normally
    ↓
Session loop top
    → runtime.run_until_idle() → pops from queued_follow_up_inputs
    → Skips interaction loop entirely
    ↓
New turn starts immediately with follow-up input
```
