# RPC-410 — HITL Wire Protocol Parity Dossier

**Goal:** the tarpc wire shapes must carry EXACTLY what the TypeScript NAPI surface carries, so the
frontend (RPC-411) can implement the TS `useHitlInput` state machine without lossy inference.
Do NOT change the tool-side internal types in `codelet/tools/src/request_user_input.rs` — they are
already parity-correct. Only the WIRE types, handle mapping, and transport plumbing change.

---

## 1. Reference semantics (TypeScript — the source of truth)

### Request read by the TUI (`sessionGetHitlRequest`)
- Full array of 1–3 questions: `HitlRequestInfo { questions: HitlQuestion[] }`
  (`src/tui/types/hitlRequest.ts:22-34`).
- `HitlQuestion { id: string; header: string; question: string; options?: HitlOption[] }`
- `HitlOption { label: string; description: string }`
- No `allow_text_input` flag exists in TS. Freeform capability is DERIVED:
  - question with NO options → pure freeform question
  - question WITH options → freeform via the virtual "Other..." entry (always available)

### Response sent by the TUI (`sessionSendHitlResponse`)
- Submit: `{ cancelled: false, answers: HitlAnswer[] }` where
  `HitlAnswer { id: string; selected: string[]; other?: string }`
  (`src/tui/hooks/useHitlInput.ts:22-26, 116-131`).
  - Option answer: `selected: [<option label>]`, no `other`.
  - Freeform / Other answer: `selected: []`, `other: <typed text>`.
- Cancel (Esc): `{ cancelled: true }` — no answers (`useHitlInput.ts:103-114`).
- One answer per question, all questions answered before submit (accumulated client-side).

### Internal tool response shape (already correct, unchanged)
`codelet/tools/src/request_user_input.rs:61-83`:
`HitlResponse::Answered { answers: HashMap<String, HitlAnswer> }` keyed by question id, or
`HitlResponse::Cancelled { cancelled: bool }`.

---

## 2. Current Rust wire shapes (the defects)

`codelet/rpc-types/src/lib.rs`:
- `HitlOption` (:1089-1095) — OK (label + description).
- `HitlRequest` (:1101-1109) — **single question** `{ id, question, header, options, allow_text_input }`.
- `HitlResponse` (:1114-1119) — `{ id, value }` — **no cancel affordance, no selected/other split**.

`codelet/sessions/src/handle_impl.rs`:
- `get_hitl_request` (:826-852) — surfaces ONLY `questions[0]` ("multi-question surface is wired in
  RPC-053" — never happened). Questions 2–3 are silently dropped → the LLM receives an answers map
  with only Q1. **Data loss.** Also hardcodes `allow_text_input: true` (:850).
- `send_hitl_response` (:748-816, RPC-408) — infers selected-vs-other by string-comparing `value`
  against the pending first question's option labels (:765-781). If a user types freeform text equal
  to an option label it is misclassified as a selection. Never sends `Cancelled` (:756-759).

`codelet/fspec-tui/src/transport/`:
- `mod.rs:605-621` — trait defaults `get_hitl_request → Ok(None)`, `send_hitl_response → Ok(())`.
- `websocket.rs:994-1023`, `embedded.rs:640-662` — tarpc forwarding.

Backend chain (unchanged by this card):
- Handler registration `codelet/agent-loop/src/agent_loop.rs:612-633` (store request → Paused →
  block on `wait_for_hitl_response` → clear → Running), cleanup at :1417.
- `codelet/sessions/src/background_session.rs` — `set_hitl_request`/`get_hitl_request`
  (:1102-1119), `wait_for_hitl_response` (:1074-1086, Cancelled{true} fallback on channel drop),
  `send_hitl_response` (:1092-1096).

---

## 3. Required target design

### 3.1 Wire types (`codelet/rpc-types/src/lib.rs`)
Replace the single-question shapes with TS-parity shapes:

```rust
pub struct HitlOption { pub label: String, pub description: String }          // unchanged

pub struct HitlQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Vec<HitlOption>,   // empty vec = freeform question (serde default)
}

pub struct HitlRequest { pub questions: Vec<HitlQuestion> }                    // FULL array

pub struct HitlAnswer {
    pub id: String,
    pub selected: Vec<String>,      // option labels; empty for freeform/Other
    pub other: Option<String>,      // typed text; None for option answers
}

pub struct HitlResponse {
    pub cancelled: bool,
    pub answers: Vec<HitlAnswer>,   // empty when cancelled
}
```

- DROP `allow_text_input` from the wire — freeform is derived exactly as TS derives it (see §1).
- Keep serde derives consistent with sibling wire types in the file (serialize/deserialize, Clone,
  Debug, PartialEq where siblings have it).

### 3.2 `handle_impl.rs::get_hitl_request`
Pass-through: map internal `codelet_tools::request_user_input::HitlRequest` → wire, converting EVERY
question (`options: None` → empty vec). No first-question slicing. Delete the RPC-053 TODO comment.

### 3.3 `handle_impl.rs::send_hitl_response`
Pass-through, replacing the RPC-408 label-inference heuristic entirely:
- `cancelled == true` → `internal HitlResponse::Cancelled { cancelled: true }`.
- else → `Answered { answers }` where the wire `answers` vec becomes the internal
  `HashMap<String, HitlAnswer>` keyed by each answer's `id`
  (internal `HitlAnswer { selected, other }` — tools crate, :61-67).
- NO comparison against option labels. NO reading of the pending request to classify answers.
  (It is acceptable to keep a `tracing::warn!` if an answer id doesn't match any pending question id,
  but the mapping must not depend on it.)

### 3.4 Transport (`fspec-tui/src/transport/`)
Update trait signatures + websocket + embedded + any mock backends in tests to the new shapes.
Compile-time propagation will find all sites; also check `codelet/fspec-tui/src/components/hitl_dialog.rs`
and `app/dispatch_pause_hitl.rs` consumers — for THIS card make the minimal mechanical adjustment to
keep the existing dialog compiling against the new shapes (RPC-411 replaces it):
first-question rendering + submit builds `HitlResponse { cancelled: false, answers: vec![one] }`,
Esc behavior unchanged in this card.

### 3.5 Out of scope
- Any UX change (RPC-411).
- napi crate (legacy path keeps using the tools-crate internal types directly — verify it still
  compiles; do not redesign it).

---

## 4. Acceptance criteria seeds (turn into rules/examples during Example Mapping)

1. A 3-question internal request surfaces ALL 3 questions over the wire in order.
2. A question without options surfaces `options: []`.
3. Wire response with `cancelled:true` reaches the blocked tool as `Cancelled { cancelled: true }`
   (tool returns the cancellation JSON to the LLM).
4. Wire response with answers `[{id:"approach", selected:["Option A"]}, {id:"notes", other:"free text"}]`
   reaches the tool as `Answered` with a 2-entry map keyed by "approach" and "notes", preserving
   selected/other EXACTLY (no inference).
5. Freeform text identical to an option label stays `other` (regression test for the RPC-408 heuristic).
6. `handle_impl.rs` contains no option-label comparison in the send path (source-shape test acceptable).

## 5. Testing notes
- Tests live in `codelet/sessions/tests/` (integration, real BackgroundSession — follow
  `rpc408_hitl_response_answer_mapping.rs` patterns: std-thread blocked waiter + bounded joins).
- **HANG-SAFETY:** any test blocking a tokio worker in `wait_for_hitl_response` MUST unblock the
  waiter BEFORE asserting and bound every join (see module docs in
  `codelet/sessions/tests/paused_chunk_delivery_rpc409.rs`). Never assert while a waiter is parked.
- `rpc408_hitl_response_answer_mapping.rs` pins the OLD heuristic — REWRITE it to pin the new
  pass-through contract (keep hang-safe helpers).
- Run from `codelet/`: `cargo test -p codelet-sessions -p codelet-rpc-types -p codelet-fspec-tui`,
  plus `cargo clippy` and `cargo fmt --check`. Tee output to a file; never pipe through head/grep.
