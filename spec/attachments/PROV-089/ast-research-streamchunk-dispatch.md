# PROV-089 AST Research — StreamChunk + stream_convert dispatch

AST analysis of the current PROV-064 streaming bridge to identify the three
extension points needed for ReasoningDelta wiring. Tools used: `AstGrep`,
`Grep`.

## Scope

Files under `codelet/providers/src/custom/` plus the test surface in
`codelet/providers/tests/custom_streaming_*`.

## 1. StreamChunk enum — `codelet/providers/src/custom/stream.rs`

Match for `pub enum StreamChunk { $$$VARIANTS }`:

- `codelet/providers/src/custom/stream.rs:34:1` — one hit; existing variants
  are TextDelta, ToolCallStart, ToolCallArgsDelta, ToolCallComplete,
  StopReason. No ReasoningDelta variant today.

**Extension point:** add `ReasoningDelta(String)` after `TextDelta(String)`.

## 2. stream_convert dispatch — `codelet/providers/src/custom/stream_convert.rs`

Match for `match kind.as_str() { $$$ARMS }`:

- `codelet/providers/src/custom/stream_convert.rs:53:5` — single match
  statement inside `handle_one`. Current arms handle `text_delta | text`,
  `tool_call_delta | tool_call`, `stop`, and `ignore | ""`.

**Extension point:** add arm `"reasoning_delta" | "thinking_delta" =>
handle_reasoning(&map)` and introduce a sibling helper that mirrors
`handle_text` (empty-text guard included).

## 3. Pass-through paths — unchanged

- `codelet/providers/src/custom/stream_http.rs` (`open_stream`): works on
  `Vec<StreamChunk>` generically; no match statement over variants.
- `codelet/providers/src/custom/provider_stream.rs`
  (`complete_with_tools_streaming`): also generic; no changes required
  beyond re-export of the new variant through `stream::StreamChunk`.

## 4. Downstream consumers (out of scope here)

Grep for `StreamChunk::` inside `codelet/providers` shows only internal
use plus tests. `cli/src/interactive/stream_loop.rs` consumes
`MultiTurnStreamItem`, not the internal `StreamChunk` type — a later
work unit will bridge `StreamChunk::ReasoningDelta` into
`StreamedAssistantContent::ReasoningDelta`.

## 5. Test surface

`codelet/providers/tests/custom_streaming_sse_bridge_tests.rs` already
covers TextDelta / ToolCall / StopReason cases via
`helpers::process_events`. New tests for PROV-089 will reuse the same
helpers (`build_processor`, `process_events`,
`streaming_config_with_script`, `build_streaming_provider`) plus a
reasoning-specific Rhai script constant.

## Conclusion

The change is contained to two files in `codelet/providers/src/custom/`:
`stream.rs` (enum variant) and `stream_convert.rs` (match arm + helper).
HTTP and provider plumbing is already variant-agnostic.
