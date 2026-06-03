# RPC-088 AST Research: Interrupt Cascade Call Sites

Generated 2026-06-01 in support of RPC-088 structural ACDD coverage.

## Source-of-truth files

| Concern | File | Line range |
|---|---|---|
| `BackgroundSession.is_interrupted: Arc<AtomicBool>` field | `codelet/sessions/src/background_session.rs` | 307 |
| `BackgroundSession.interrupt_notify: Arc<Notify>` field | `codelet/sessions/src/background_session.rs` | 310 |
| Field initialisation in `new` | `codelet/sessions/src/background_session.rs` | 480, 481 |
| `BackgroundSession::interrupt()` | `codelet/sessions/src/background_session.rs` | 1114–1119 |
| `BackgroundSession::reset_interrupt()` | `codelet/sessions/src/background_session.rs` | 1124–1128 |
| `BackgroundSession::get_interrupt_notify()` | `codelet/sessions/src/background_session.rs` | 1134–1136 |
| `reset_interrupt()` call before each turn | `codelet/agent-loop/src/agent_loop.rs` | 308 |
| `run_with_provider!` macro forwards both handles to `run_agent_stream_with_images` | `codelet/agent-loop/src/dispatch.rs` | 93, 95 |
| OpenAI inlined arm forwards both handles | `codelet/agent-loop/src/agent_loop.rs` | 902, 904 |
| Custom-provider fallthrough forwards both handles | `codelet/agent-loop/src/agent_loop.rs` | 1001, 1003 |
| `is_interrupted.load(Acquire)` short-circuit + `output.emit_interrupted` | `codelet/cli/src/interactive/stream_loop.rs` | 697–714 |
| Esc → `is_interrupted.store(true, Release)` (CLI/TUI input branch) | `codelet/cli/src/interactive/stream_loop.rs` | 742 |
| `tokio::select!` on `interrupt_notify.notified()` (NAPI/agent path) | `codelet/cli/src/interactive/stream_loop.rs` | 773–778 |
| `StreamOutput::emit_interrupted` driver | `codelet/cli/src/interactive/output.rs` | (StreamEvent::Interrupted) |
| `BackgroundOutput::emit StreamEvent::Interrupted` arm → `StreamChunk::interrupted(queued)` | `codelet/agent-loop/src/background_output.rs` | 255–259 |
| `StreamChunk::Interrupted { queued_inputs }` variant + `interrupted` ctor | `codelet/rpc-types/src/lib.rs` | 1052–1055, 1172–1174 |

## Canonical body — dispatch.rs run_with_provider! tail

```rust
codelet_cli::interactive::run_agent_stream_with_images(
    agent,
    $input,
    $images,
    $inner,
    $session.is_interrupted.clone(),
    $session.compaction_in_progress.clone(),
    $session.interrupt_notify.clone(),
    $output,
)
.await
```

The 5th and 7th positional arguments are the `is_interrupted` clone and the
`interrupt_notify` clone — both pulled from `BackgroundSession` and both
required for Esc to actually abort the stream.

## Canonical body — agent_loop.rs reset_interrupt()

```rust
session.set_status(SessionStatus::Running);
session.reset_interrupt();
```

`reset_interrupt()` MUST run before each turn so a previous Esc does not
poison the next prompt.

## Canonical body — background_output.rs Interrupted arm

```rust
StreamEvent::Interrupted(queued) => {
    // REFAC-007: Persist any accumulated content on interrupt
    self.persist_assistant_message();
    StreamChunk::interrupted(queued)
}
```

## Pre-existing implementation status

`BackgroundSession::{is_interrupted, interrupt_notify, interrupt(),
reset_interrupt(), get_interrupt_notify()}` were already present on the
session struct. `run_with_provider!` and both inlined arms in
`codelet/agent-loop` were lifted as part of RPC-072 and forward both
handles. `stream_loop.rs` short-circuits the stream on
`is_interrupted.load(Acquire)` and selects on `interrupt_notify.notified()`.
`BackgroundOutput::emit` translates `StreamEvent::Interrupted(queued)`
into `StreamChunk::interrupted(queued)`.

RPC-088 lands ACDD coverage pinning the contract so subsequent edits
cannot drop the handle forwarding, the reset_interrupt() pre-turn call,
or the Interrupted translation arm without test failure.

## Coverage strategy

Structural source-string assertions over the dispatch/openai/custom-
provider arms plus census of the rpc-types Interrupted variant +
constructor, mirroring the pattern from RPC-082/083/084/086.
