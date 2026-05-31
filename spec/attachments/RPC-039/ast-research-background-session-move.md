# AST Research — RPC-039 BackgroundSession move

## Goal of the research

Confirm the exact span of code being moved, identify every reference inside the
moved code that needs an import-path rewrite, and identify references OUTSIDE
the moved code that depend on the moved symbols (so the napi adapter knows
which symbols to `pub use` re-export).

## 1. Source span — `codelet/napi/src/session_manager.rs`

AST queries:

```
ast-grep --lang rust --pattern 'pub struct BackgroundSession { $$$FIELDS }'
ast-grep --lang rust --pattern 'impl BackgroundSession { $$$BODY }'
```

Results:

* `codelet/napi/src/session_manager.rs:459:1: pub struct BackgroundSession {`
  — fields run 459 → 600.
* `codelet/napi/src/session_manager.rs:602:1: impl BackgroundSession {`
  — impl block runs 602 → 1356 (the next top-level item is `pub struct
  ChainOfCommand` at line 1364, with the `}` closing the impl at 1356).

Net span to move: lines **459–1356** (struct + impl, ~898 LOC).

## 2. Supporting types defined above the struct that must move with it

These items are defined ABOVE line 459 in the same file but are owned by the
moved code at the field level. They must move with `BackgroundSession`
because the moved struct + impl will not compile without them:

| Line | Symbol | Why it moves |
|---|---|---|
|  93 | `pub(crate) struct PromptInput` | `input_tx: mpsc::Sender<PromptInput>` (field) |
| 116 | `pub(crate) struct CompactionProgress` | `compaction_progress: RwLock<Option<CompactionProgress>>` (field) |
| 128 | `pub struct WorkUnitContext` + `impl WorkUnitContext` | `work_unit_context: RwLock<Option<WorkUnitContext>>` (field) |
| 277 | `pub struct IncomingMessage` + `impl IncomingMessage` | `incoming_message_tx/rx: mpsc::*::<IncomingMessage>` (fields) |
| 291 | `pub struct BridgeImageData` | `IncomingMessage.images: Option<Vec<BridgeImageData>>` |
| 342 | `pub fn format_incoming_message` | called by methods on `BackgroundSession` |
| 428 | `pub enum SessionError` + `Display/Error/From<GitError>` | returned by `checkpoint`/`restore`/`list_checkpoints` |

`pub struct Interjection` (line 167) and `pub fn parse_interjection` (line
193) are NOT used by `BackgroundSession` itself — they are used by
`SessionManager`'s `agent_loop`. They STAY in `codelet/napi/src/session_manager.rs`.

## 3. Pre-existing imports inside the moved code that need rewriting

```
ast-grep --lang rust --pattern 'use crate::persistence::{ $$$NAMES }'
ast-grep --lang rust --pattern 'use crate::types::{ $$$NAMES }'
```

* `codelet/napi/src/session_manager.rs:12: use crate::persistence::{ load_session, append_message_with_metadata, update_session_tokens, MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent }`
  — these all came from the lifted `codelet_core::persistence` module
  (RPC-031..RPC-034). Inside the moved code rewrite to
  `use codelet_core::persistence::{ ... }`.
* `codelet/napi/src/session_manager.rs:16: use crate::types::{ CompactionResult, DebugCommandResult, NotificationSeverity, SessionState, StreamChunk, ToolCallInfo, ToolResultInfo, NapiTurnDetails, NapiToolCall, NapiFileModification }`
  — of those, only the ones used inside the BackgroundSession impl block
  must come from `codelet_rpc_types`. None of the NAPI-only types
  (`DebugCommandResult`, `NapiTurnDetails`, `NapiToolCall`,
  `NapiFileModification`) are referenced inside `impl BackgroundSession`
  (verified by `awk 'NR>=602 && NR<=1356' | grep -E 'DebugCommandResult|NapiTurnDetails|NapiToolCall|NapiFileModification'`
  which returns zero matches), so the moved code only needs
  `use codelet_rpc_types::{ CompactionResult, NotificationSeverity, SessionState, StreamChunk, ToolCallInfo, ToolResultInfo, SessionStatus, SessionId, SessionInfo }`.

`crate::types::FspecResult` shows up at lines 546, 547, 626, 1074, 1078,
1092 — all INSIDE the impl block. All rewrites use
`codelet_rpc_types::FspecResult`.

## 4. The single `napi::` reference inside the moved code

```
ast-grep --lang rust --pattern 'pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<()> { $$$BODY }'
```

Result:

```
codelet/napi/src/session_manager.rs:1237:5: pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<()>
```

`Result<()>` here resolves to `napi::bindgen_prelude::Result<()>` because
the file-level `use napi::bindgen_prelude::*;` is in scope. Body uses
`Error::from_reason(format!("Failed to send input: {}", e))`. The
rewrite is:

```rust
pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String> {
    ...
    self.input_tx
        .try_send(PromptInput { input, thinking_config })
        .map_err(|e| {
            self.set_status(SessionStatus::Idle);
            format!("Failed to send input: {}", e)
        })
}
```

The napi side wraps the new `Result<(), String>` back into
`napi::Result<()>` inside the `#[napi]` free function
`session_send_input` (codelet/napi/src/session_manager.rs:6555).

## 5. `GLOBAL_CHUNK_CALLBACK` references

```
ast-grep --lang rust --pattern 'GLOBAL_CHUNK_CALLBACK.get()'
```

Result (7 matches):

```
codelet/napi/src/session_manager.rs:963   ← INSIDE BackgroundSession::handle_output (moves)
codelet/napi/src/session_manager.rs:3521  ← SessionManager method (stays in napi)
codelet/napi/src/session_manager.rs:3775  ← SessionManager method (stays in napi)
codelet/napi/src/session_manager.rs:5056  ← agent_loop helper (stays in napi)
codelet/napi/src/session_manager.rs:5336  ← agent_loop helper (stays in napi)
codelet/napi/src/session_manager.rs:6203  ← worker emit (stays in napi)
codelet/napi/src/session_manager.rs:6469  ← supervisor relay (stays in napi)
```

Of these, only the call at **line 963 (inside `BackgroundSession::handle_output`)**
must be rewritten in this card. The other six remain in napi until
RPC-041 (which deletes the global entirely and migrates all six call
sites to the new `chunks_tx` broadcast subscriber).

The chosen replacement for line 963 (per the attachment, with the
mitigation note inverted to keep the moved code napi-free):

* Add a new field `chunks_tx: Option<tokio::sync::broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)>>` on `BackgroundSession`, defaulted to `None` in `BackgroundSession::new`.
* In `handle_output`, after the existing `supervisor_broadcast.send(chunk.clone())` line and BEFORE the (removed) GLOBAL_CHUNK_CALLBACK call, insert:

  ```rust
  // RPC-039: forward the chunk on the new SessionManager-owned broadcast
  // channel. RPC-041 wires both ends; in this card the field is None so
  // the call is a cheap no-op and no chunks are dropped because the
  // NAPI-side global callback (kept alive in this card) still fans out.
  if let Some(tx) = &self.chunks_tx {
      let _ = tx.send((
          codelet_rpc_types::SessionId::from(self.id.to_string()),
          chunk.clone(),
      ));
  }
  ```

* Delete the `if let Some(global_cb) = GLOBAL_CHUNK_CALLBACK.get() { global_cb.call(...) }` block from the moved version (it can no longer reference the napi global). Behavioural equivalence is preserved because the OTHER six call sites listed above still drive GLOBAL_CHUNK_CALLBACK from the napi side during RPC-039's lifetime, and `BackgroundSession::handle_output` itself is only called from those very same agent-loop sites. The end-to-end fan-out path remains intact.

## 6. References to moved symbols OUTSIDE the moved code

Symbols moved out of napi but still referenced from `codelet/napi/src/session_manager.rs` (and a few other napi modules):

| Symbol | First few external call-sites in napi |
|---|---|
| `BackgroundSession` | 3142 (field of `SessionManager`), 2900-3140 (test module), 4626 (`run_session_loop` signature) |
| `IncomingMessage` | 2513, 2570, 2597, 2604, 2634, 2658, 4626, 4713, 5259-5269 |
| `format_incoming_message` | 2514, 2543, 2659, 4713 |
| `BridgeImageData` | 4616, 5252, 5380 |
| `PromptInput` | 3290, 3584, 4626 |
| `CompactionProgress` | 1162, 1169, 1176, 6596-6598 |
| `WorkUnitContext` | 8022 (`session_get_work_unit_context` napi function), 2905-3133 (test module) |
| `SessionError` | Used only inside BackgroundSession impl — no external callers in napi |

Strategy: in the new
`codelet/napi/src/session_manager.rs` (post-move), insert near the top
(replacing the deleted struct/impl definitions):

```rust
pub use codelet_sessions::background_session::{
    BackgroundSession, IncomingMessage, BridgeImageData, format_incoming_message,
    PromptInput, CompactionProgress, WorkUnitContext, SessionError,
};
```

This preserves every external reference inside the file (and any other
napi module that may have imported them via `crate::session_manager::*`).

## 7. Existing tests that touch the moved symbols

The unit-test module inside `codelet/napi/src/session_manager.rs`
(`#[cfg(test)] mod tests { ... }`, ~lines 2870–3134) contains tests for:

* `WorkUnitContext::new`, `WorkUnitContext::is_set`,
  `WorkUnitContext::format_for_environment`,
  `WorkUnitContext::default`, debug formatting (~9 tests)
* `IncomingMessage::new`, `IncomingMessage::with_images`,
  `format_incoming_message` (~6 tests)
* `parse_interjection` (~4 tests — these stay napi-side because
  `parse_interjection` itself stays napi-side)

All of these tests resolve their type references via the re-exported
paths once the move is complete (no rename of imports required because
the re-exports preserve the original `crate::session_manager::*` paths).

## 8. Pre-move and post-move metrics

Pre-move:

* `codelet/napi/src/session_manager.rs`: **8645 LOC** (1 file)
* `codelet/sessions/src/background_session.rs`: **8 LOC** (placeholder)

Post-move target:

* `codelet/napi/src/session_manager.rs`: ~7720 LOC (898 LOC removed
  for the BackgroundSession struct+impl, ~70 LOC removed for the
  supporting types, ~10 LOC added for the `pub use` re-export).
* `codelet/sessions/src/background_session.rs`: ~990 LOC (~898 for
  the BackgroundSession + ~90 for supporting types).

The single-commit move keeps the napi file in a compilable state because
the re-exports preserve every external reference.

## 9. Build & test invariants asserted by the feature file

1. `cargo build -p codelet-sessions` succeeds.
2. `cargo build -p codelet-napi` (default features) succeeds.
3. `cargo build -p codelet-napi --release` regenerates
   `codelet/napi/index.d.ts` with NO removed or renamed symbols.
4. `cargo metadata -p codelet-sessions --format-version 1` contains
   zero `codelet-napi` package entries in the transitive graph
   (enforced by the existing `codelet/sessions/tests/skeleton_invariants.rs`).
5. `cargo test -p codelet-napi --lib session_manager::tests` passes.
6. `cargo test -p codelet-sessions --tests` passes (smoke +
   new `background_session_shape.rs`).
7. Grep `codelet/sessions/src/background_session.rs` for
   `napi::|use napi|#[napi` returns zero matches.
8. Grep `codelet/sessions/src/background_session.rs` for
   `crate::persistence|crate::types` returns zero matches.

The integration test that exercises (7) and (8) is the new
`background_session_shape.rs` written in TypeScript-port-style: the
test reads the file under
`codelet/sessions/src/background_session.rs` and runs `grep` /
`regex` assertions. The shape test also `use codelet_sessions::background_session::*;` to prove the public path
resolves at compile time.
