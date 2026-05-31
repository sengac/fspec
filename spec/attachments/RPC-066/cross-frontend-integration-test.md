# RPC-066 — Cross-frontend integration test against stub provider

**Parent:** RPC-030 · **Phase:** 8.2 · **Estimate:** 5 pts · **Depends on:** RPC-065

## Goal

Boot the `fspec` binary in **combined mode** against a **stub provider** (deterministic LLM responses). Drive every slash command end-to-end. Capture the chunk stream. Assert the chunk stream matches the equivalent TS-frontend run against the same stub provider, modulo cosmetic differences (timestamps, UUIDs).

## Test location

`codelet/fspec/tests/cross_frontend_parity.rs` (top-level binary test, exercises the full stack).

## Approach

1. **Stub provider setup**: register a custom provider that returns canned responses. `codelet-providers` already supports this — confirm via `ProviderType::Custom` and `create_rig_agent`.

2. **TS reference capture**: pre-record the TS frontend's chunk stream for a fixed scripted run. Store as `tests/fixtures/ts_reference_run.jsonl`.

3. **Rust run**: spawn `fspec` (combined mode) with the same stub provider config + scripted input. Capture chunks via `backend.chunks_rx()`.

4. **Compare**: assert chunk streams are equivalent.

## Scripted run

```rust
async fn scripted_run() {
    // 1. Send "/help"     — expect HelpDialog chunk events (none, but UI side)
    // 2. Send "hello"     — expect Text + Done chunks
    // 3. Send "/clear"    — expect clear, scrollback empty
    // 4. Send "/thinking high" — expect ThinkingLevel chunk-like event
    // 5. Send a prompt that triggers a tool call
    //    — expect ToolCall + ToolResult chunks
    // 6. Send "/compact"  — expect CompactionComplete chunk
    // 7. Send "/quit"     — expect clean shutdown
}
```

## Stub provider implementation

`codelet/fspec/tests/fixtures/stub_provider.rs`:

```rust
pub struct StubProvider {
    responses: Vec<StubResponse>,
}

pub enum StubResponse {
    Text(String),
    Thinking(String),
    ToolCall { name: String, input: serde_json::Value, expected_result: serde_json::Value },
    Done,
}

impl Provider for StubProvider { /* …deterministic generation… */ }
```

Register via `ProviderManager::register_custom(...)` before constructing `SessionManager`.

## Equivalence definition

Two chunk streams are equivalent if, after substituting:
- Timestamps → `<ts>` placeholder
- UUIDs → `<uuid>` placeholder
- Correlation IDs → `<corr>` placeholder
- Tool call IDs → `<tc>` placeholder

the resulting JSON arrays are identical (modulo ordering of independent chunks, if any).

Use a custom JSON-diff helper:

```rust
fn normalise_chunk_stream(chunks: &[StreamChunk]) -> Vec<NormalisedChunk>;
fn assert_chunks_equivalent(rust: &[StreamChunk], ts_reference: &[NormalisedChunk]);
```

## TS reference capture (run once, commit)

Outside this test, manually:

1. Run the TS frontend against the stub provider with the scripted inputs.
2. Capture all chunks (TS frontend already logs to JSONL when `FSPEC_DEBUG_DIR` is set).
3. Save to `codelet/fspec/tests/fixtures/ts_reference_run.jsonl`.

Document the regeneration procedure in `codelet/fspec/tests/README.md`.

## Acceptance criteria

1. `codelet/fspec/tests/cross_frontend_parity.rs` exists and runs.
2. Stub provider is deterministic across runs.
3. Rust chunk stream matches TS reference (after normalisation).
4. The test catches regressions: try inserting a bug (e.g., emit wrong chunk type) — test must fail.
5. Test runs in < 60s.

## Risks

- Maintaining the TS reference fixture: every TS frontend change requires regeneration. Document clearly.
- Tool calls invoke real Bash / file I/O. Use sandboxed tools or stub-only paths.
- Network determinism: stub provider must never reach the network.

## Out of scope

- Visual / TUI rendering equivalence (covered by ratatui snapshot tests, separate concern).
- Performance / latency comparison.
