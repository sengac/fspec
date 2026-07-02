# RPC-408 — send_hitl_response Discards the User's Answer

**Type:** Bug (data loss in HITL round-trip)
**Crates:** `codelet-sessions` (fix), `codelet-rpc-types` / `codelet-tools` (reference)

## 1. Problem statement

The HITL (request_user_input) round-trip in the standalone Rust TUI:

1. Agent loop registers a HITL handler (`agent-loop/src/agent_loop.rs` ~line 600-630): tool calls `execute_hitl(session_id, HitlRequest{questions})` → handler stores the request on the session, sets `Paused`, blocks on `session.wait_for_hitl_response()`.
2. TUI opens `HitlDialog` (`fspec-tui/src/components/hitl_dialog.rs`), user picks an option or types free text, dialog emits `Action::HitlSubmitted{session_id, response: codelet_rpc_types::HitlResponse{id, value}}` → `backend.send_hitl_response`.
3. **BUG:** `codelet/sessions/src/handle_impl.rs:748-766` ignores the payload entirely:

```rust
fn send_hitl_response(&self, session_id: &SessionId, _response: HitlResponse) -> Result<(), String> {
    ...
    session.send_hitl_response(
        codelet_tools::request_user_input::HitlResponse::Cancelled { cancelled: false },
    );
```

The comment says *"full answer-mapping is wired in RPC-053"* — RPC-053 wired the dialog but never fixed this stub. Every HITL answer arrives at the tool as `Cancelled{cancelled:false}`; the facade wrapper (`tools/src/facade/wrapper.rs:254`) maps `Cancelled` to a tool error, so the agent sees a cancellation regardless of what the user chose.

## 2. Type shapes (the mapping to implement)

**Wire** (`rpc-types/src/lib.rs:1114-1119`):
```rust
pub struct HitlResponse { pub id: String, pub value: String }
```
`id` = the question id from the wire `HitlRequest` (line 1103-1109, single-question); `value` = selected option **label** OR the freeform typed text.

**Internal** (`tools/src/request_user_input.rs:60-83`):
```rust
pub struct HitlAnswer { pub selected: Vec<String>, pub other: Option<String> }
pub enum HitlResponse {
    Answered { answers: HashMap<String, HitlAnswer> },   // keyed by question id
    Cancelled { cancelled: bool },
}
```

**Required mapping** in `handle_impl.rs::send_hitl_response`:
- Look up the session's stored internal/wire HITL request (`session.get_hitl_request()` — investigate its exact return shape and where the internal→wire request conversion lives; likely `sessions/src/conversions.rs` or `background_session.rs`) to obtain the pending question's `id` and its option labels.
- If `response.value` equals one of the pending question's option labels → `HitlAnswer{ selected: vec![value], other: None }`.
- Otherwise (free text via `allow_text_input`) → `HitlAnswer{ selected: vec![], other: Some(value) }`.
- Build `Answered { answers: { <question_id>: <answer> } }` and send via `session.send_hitl_response(...)`.
- Use `response.id` as the question id key; if it doesn't match the pending request's id, still answer with `response.id` as key but log via `tracing::warn!` (investigate what `execute_hitl` expects — if it validates ids strictly, prefer the pending request's id and document).
- Genuine cancel (HitlDialog Esc) sends **nothing** today (dialog pops without submit, RPC-053 rule 11) — do NOT introduce a Cancelled send from this path; `Cancelled` remains reserved for an explicit future cancel affordance.

## 3. Parity reference

The napi/TS path already does real answer mapping — find the napi-side equivalent (`codelet/napi/src/…`, search for `HitlResponse` / `sessionSendHitlResponse` or the TSFN HITL handler) and mirror its semantics so both frontends produce identical `Answered` payloads for the same user action.

## 4. Test plan (minimum)
- Unit/integration on the sessions crate: create a `BackgroundSession` (existing test helpers, e.g. the `rpc081_restore_session_messages.rs` pattern with `NoopSessionManagerHooks`), store a pending HITL request with options `["Yes","No"]` + `allow_text_input`, spawn a thread blocked on `wait_for_hitl_response()`, call handle `send_hitl_response` with:
  1. `value = "Yes"` → blocked thread receives `Answered` with `selected == ["Yes"]`, `other == None`, keyed by the question id.
  2. `value = "maybe later"` (not a label) → `Answered` with `selected == []`, `other == Some("maybe later")`.
  3. Assert the response is **never** `Cancelled` from this path.
- End-to-end shape: `execute_hitl` with a registered handler that routes through the session channel returns the mapped `Answered` to the tool (facade wrapper maps to JSON success, `wrapper.rs:246`).
- Regression lock: source-shape or unit assertion that `handle_impl.rs::send_hitl_response` no longer contains the hard-coded `Cancelled { cancelled: false }`.

## 5. Non-goals
- Multi-question wire support (wire `HitlRequest` is single-question by design).
- HitlDialog UI changes (covered by RPC-053; inline HITL is a separate concern).
- WebSocket transport changes (`transport/websocket.rs` just forwards the wire struct — untouched).
