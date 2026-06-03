# RPC-084 — AST research: streaming dispatch call sites

Performed via AstGrep before writing tests.

## 1. `codelet_cli::interactive::run_agent_stream_with_images` call sites

Pattern: `codelet_cli::interactive::run_agent_stream_with_images($$$ARGS)` (Rust)

- `codelet/agent-loop/src/agent_loop.rs:897` — OpenAI inlined match arm
- `codelet/agent-loop/src/agent_loop.rs:996` — `_ =>` custom-provider fallthrough match arm

(The third call lives inside the `run_with_provider!` macro body at
`codelet/agent-loop/src/dispatch.rs:88`. AST-grep does not descend into
`macro_rules!` bodies, so this site is verified via source-string
parsing in the test, not via AST. Manual `Grep` confirms the call.)

## 2. `pub enum StreamChunk` declaration

Pattern: `pub enum StreamChunk { $$$VARIANTS }` (Rust)

- `codelet/rpc-types/src/lib.rs:1006` — single canonical declaration.

The enum currently exposes 22 variants. The gap analysis requires ≥19.

## 3. `StreamChunk::$NAME(...)` construction sites in BackgroundOutput

Pattern: `StreamChunk::$NAME($$$ARGS)` (Rust)

In `codelet/agent-loop/src/background_output.rs`:

| Line  | Constructor                          |
|-------|--------------------------------------|
| 109   | `StreamChunk::text`                  |
| 117   | `StreamChunk::thinking`              |
| 134   | `StreamChunk::tool_call`             |
| 187   | `StreamChunk::user_notification`     |
| 210   | `StreamChunk::tool_result`           |
| 216   | `StreamChunk::tool_progress`         |
| 224   | `StreamChunk::user_notification`     |
| 233   | `StreamChunk::token_update`          |
| 244   | `StreamChunk::context_fill_update`   |
| 253   | `StreamChunk::error`                 |
| 258   | `StreamChunk::interrupted`           |
| 280   | `StreamChunk::done`                  |
| 289   | `StreamChunk::session_state_change`  |
| 306   | `StreamChunk::session_state_change`  |
| 308   | `StreamChunk::compaction_complete`   |
| 320   | `StreamChunk::session_state_change`  |
| 321   | `StreamChunk::user_notification`     |
| 328   | `StreamChunk::session_state_change`  |
| 353   | `StreamChunk::tool_progress`         |

All 11 canonical translations from rig `StreamEvent` to `StreamChunk`
are present.

## Conclusion

Implementation pre-exists from the RPC-080/081 ports. RPC-084 lands
ACDD coverage to lock the structural parity in place against future
drift — same pattern as RPC-082 / RPC-083.
