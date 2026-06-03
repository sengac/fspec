# RPC-089 AST Research — MCP Injection Drain Source-Shape

**Date:** 2026-06-01
**Work unit:** RPC-089 — Agent loop: MCP injection drain (`mcp_injection_rx` tokio::select! arm + `mcp_channel_open` flag)
**Pattern:** Regression-shape coverage (mirrors RPC-082 / RPC-083 / RPC-084 / RPC-085 / RPC-086 / RPC-088)

## Goal

Pin the source-shape of the MCP injection drain inside the canonical
agent loop at `codelet/agent-loop/src/agent_loop.rs` so the
implementation lifted from `codelet/napi/src/agent_loop.rs:323-460`
cannot silently regress. The original RPC-072 stub held the channel
open under the name `_mcp_injection_rx` and never drained it — a
regression to that shape would dead-letter every MCP server message
without any test failing.

## TypeScript/NAPI reference

`codelet/napi/src/agent_loop.rs:323-460` (canonical):

```rust
let mut mcp_channel_open = true;
loop {
    let input_with_images = tokio::select! {
        result = input_rx.recv() => match result {
            Some(prompt_input) => Some(InputWithImages { ... }),
            None => { break; }
        },
        result = mcp_injection_rx.recv(), if mcp_channel_open => {
            match result {
                Some(McpInjection::Notification(text)) => {
                    Some(InputWithImages { text, thinking_config: None, images: None })
                }
                Some(McpInjection::SamplingRequest { params, response_tx }) => {
                    /* V2 — reject with structured error */
                    None
                }
                None => {
                    mcp_channel_open = false;
                    None
                }
            }
        }
    };
}
```

Boundary contract:
- `mcp_channel_open` starts `true`.
- The `tokio::select!` arm is gated on `if mcp_channel_open` so a
  closed receiver does not busy-loop the executor.
- Three `McpInjection` outcomes are routed: Notification (process as
  turn), SamplingRequest (V2 — acknowledged but rejected), None
  (flag flip).

## Rust call site (AST search)

```
ast-grep --lang rust --pattern 'pub async fn agent_loop($$$ARGS) { $$$BODY }'
```

**Match:** `codelet/agent-loop/src/agent_loop.rs:74-243`

Relevant excerpts:

```rust
// Line 74-78
pub async fn agent_loop(
    session: Arc<BackgroundSession>,
    mut input_rx: mpsc::Receiver<PromptInput>,
    mut mcp_injection_rx: mpsc::Receiver<McpInjection>,
) {

// Line 79-83
    // MCP-001-FIX: Track whether the MCP injection channel is still open.
    // Once it returns None (sender dropped by cleanup_mcp_session), we must stop
    // polling it. Without this guard, the closed channel returns None immediately
    // every iteration, causing tokio::select! to resolve instantly → CPU busy-loop.
    let mut mcp_channel_open = true;

// Line 192-242 (excerpt)
    // MCP-001: Server-initiated MCP messages (notifications, sampling requests)
    // MCP-001-FIX: Only poll when channel is open to prevent busy-loop spin
    result = mcp_injection_rx.recv(), if mcp_channel_open => {
        match result {
            Some(McpInjection::Notification(text)) => {
                /* emit incoming_message chunk + InputWithImages */
            }
            Some(McpInjection::SamplingRequest { params, response_tx }) => {
                /* reject — V2 feature */
                let _ = response_tx.send(Err("sampling/createMessage not yet supported — V2 feature".to_string()));
                None
            }
            None => {
                mcp_channel_open = false;
                None
            }
        }
    }
```

## Invariants the source-shape test must pin

1. Function signature contains `mut mcp_injection_rx: mpsc::Receiver<McpInjection>`.
2. Function signature does NOT contain `_mcp_injection_rx` (regression to RPC-072 stub).
3. Body declares `let mut mcp_channel_open = true;`.
4. Body contains the inline-guarded select arm
   `result = mcp_injection_rx.recv(), if mcp_channel_open =>`.
5. The select arm body contains all three required match arms:
   - `Some(McpInjection::Notification(text)) =>`
   - `Some(McpInjection::SamplingRequest { params, response_tx }) =>`
   - `None =>`
6. Body contains `mcp_channel_open = false;` (busy-loop guard fix).
7. Byte-offset ORDER: `mut mcp_channel_open = true;` precedes
   `mcp_channel_open = false;` — proving the initialiser is the
   canonical state and the flip lives later (inside the None arm).

## Test file

`codelet/agent-loop/tests/rpc089_mcp_injection_drain.rs`

Reads the source of `agent_loop.rs` via `CARGO_MANIFEST_DIR` and
asserts the substring + brace-balanced body + byte-offset invariants.
Sub-millisecond execution. Pairs with the existing single behavioural
scenario in `spec/features/agent-loop-mcp-injection.feature` (currently
`@deferred` because it requires real MCP plumbing).

## Tags

- `@rpc-089` — work unit identifier
- `@source-shape` — pattern marker
- `@regression`, `@agent-loop`, `@mcp`, `@rust`

## Related cards

- **RPC-072** — original work-agent-roundtrip parent; RPC-089 is a child
- **RPC-080 / RPC-081 / RPC-082 / RPC-083 / RPC-084 / RPC-085 / RPC-086 / RPC-088** —
  sibling coverage cards on agent_loop.rs structural shape
- **MCP-001 / MCP-001-FIX** — original implementation tickets for the
  drain and the busy-loop guard
