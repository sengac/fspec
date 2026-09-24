# JEV-003 — Strict Jev Classifier Request Schema

Scope: `src/server/request.rs` — parse + validate an incoming
`/v1/classifier` body into a fully-typed, pre-inference `ClassifierRequest`.
Pure module: no model, no IO. Contract in `protocol-reference.md` (attached to
JEV-002, epic card).

## Public surface

```rust
/// A validated classifier request, ready for inference.
pub struct ClassifierRequest {
    pub model: String,
    pub context: Context,          // exactly one
    pub questions: Vec<(String, Question)>,  // preserves insertion order
    // options: raw_logits must be false (enforced; not stored)
}

pub enum Context { State(Value), Messages(Vec<ChatMessage>) }
pub struct ChatMessage { pub role: Role, pub content: String }
pub enum Role { System, Developer, User, Assistant }
```

`Question` reuses `laya::Question` (the existing `QType`/criteria structs from
`src/schema.rs`) — the only difference from `batching::RawQuestion` is
*strictness*: unknown question fields and unknown criterion keys are errors,
criteria descriptions must be string/object/array/null (non-strings
serialized with the protocol's canonical JSON, see below), and per-type
cardinality (2–50) is enforced.

## Validation rules (all → 422 unless noted)

1. Body must be a JSON object; top-level non-object → parse error.
2. Unknown top-level fields are **ignored** (record the parsed value; never
   fail on `stream`, `temperature`, `max_tokens`, ...).
3. `model`: required, nonempty string. (Equality against the loaded model is
   checked by the handler, not here — this module is model-agnostic; it
   returns the string for the caller.)
4. Context XOR: exactly one non-null of `state` / `messages`. Both set or both
   absent → `Provide exactly one of state or messages`.
   - `state`: string, JSON object, or array. A bare number/bool or `null` →
     422. Empty string / `{}` / `[]` are valid.
   - `messages`: array, ≥1 message; each message is exactly
     `{role: system|developer|user|assistant, content: string}` — `additionalProperties: false`.
     Tool/function roles, null content, content-part arrays, `name`, or any
     extra field → 422 with the reference phrase about *text messages* only.
5. `questions`: object, 1..=256 entries, nonempty string keys. Per entry, by
   `type` (discriminator):
   - unknown/missing `type` → 422 (name the offending question id in `param`)
   - unknown question fields (anything besides `type`, `instructions`,
     `criteria`) → 422
   - `instructions`: string / object / array / null (all four allowed)
   - **choice**: `criteria` object, 2..=50 keys; each value string/object/
     array/null; key order preserved (serde_json `preserve_order` or explicit
     index tracking — insertion order assigns labels and breaks ties)
   - **score**: `criteria` array, 2..=50 items, each string/object/array/null;
     order preserved (ordinal)
   - **noul**: `criteria` optional; when present an object with **only** the
     keys `true`/`false` (either or both), each value string/object/array/null
6. `options`: optional object; known field `raw_logits` (bool). Any nonempty
   value of the reserved fields `tools`, `mm_processor_kwargs`,
   `media_io_kwargs` → 422 (laya backend rejects them). `options` containing
   any unknown field → 422.
7. `allow_inf_nan` semantics: NaN/Infinity in JSON (which `serde_json` rejects
   by default anyway) stay rejected; document that the schema is strict here.

## Errors

Validation failures produce `Vec<RequestError { param, message, type_ }>`
(dotted paths with `[i]` indices), which the handler folds into the protocol
422 envelope (up to 10 details; the rest counted in the summary message,
matching the reference). A dedicated `ClassifierError` enum distinguishes
*schema errors* (this module) from *admission errors* (model mismatch,
token overflow, branch limit) raised by the handler — both map to 422 but
with different `param`/`type` values.

## Canonical serialization of non-string entries

The protocol's `canonical()` (simple-jev `common/prompt_builder.py`):
strings pass through; objects/arrays are serialized as deterministic JSON
(compact, keys sorted where objects, `null` for null entries). laya-rs must
port this byte-for-byte for `instructions` and criteria descriptions, since
it becomes prompt text via `schema::build_sequence`.

## Testing notes

- Weight-free: every rule above is unit-testable on plain JSON (the
  `tests/cli_answer.rs`-style gating convention applies only to inference).
- Mirrors simple-jev `common/tests/test_pipeline.py` + the validation cases in
  `hf-server/tests/test_api.py` (choice/score/noul shapes, XOR context,
  unknown-field rejection, criteria limits, reserved fields).
