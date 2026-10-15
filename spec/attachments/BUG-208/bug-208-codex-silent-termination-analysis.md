# BUG-208 Analysis: Why Codex Models Silently Terminate

**Date:** 2026-10-15
**Status of investigation:** Root cause confirmed, reproduced, and contrasted against the official Codex client (`/tmp/codex`, github.com/openai/codex).

---

## TL;DR

Two independent defects combine into one user-visible symptom ("I can't use the codex models — the conversation just ends"):

1. **The Codex backend rejects our request** because the `AstGrepRefactor` tool's JSON schema is invalid for its strict validator: an object-typed schema node without a `properties` key at `/properties/transforms/type/0`.
2. **rig silently swallows the rejection.** The backend sends an SSE `{"type":"error", ...}` event (with HTTP 200). rig's `StreamingCompletionChunk` untagged enum cannot deserialize it, so the event is logged at `debug!` and skipped (`continue`). The stream then "completes" with zero chunks → empty `FinalResponse` → the agent loop ends with no text and no error.

The official Codex client does **neither**: it parses `error` events into typed `ApiError`s and surfaces them, and it **sanitizes every tool schema** before sending, guaranteeing object-typed schemas always carry `properties`. Reference code below.

---

## Evidence (live test matrix)

Test file: `rust/providers/tests/codex_gpt6_astra_live_test.rs`
(real OAuth from `~/.codex/auth.json`, same token system as production `CodexProvider`)

| Test | Result | Meaning |
|---|---|---|
| `live_gpt6_astra_non_streaming_is_rejected` | ✅ pass | Backend requires `stream:true` (documents why the streaming path is the only path) |
| `live_gpt6_astra_streaming_completion` (no tools) | ✅ pass | Token, model, and streaming path are healthy — gpt-6-astra answers fine |
| `live_gpt6_astra_streaming_with_tools` (one clean dummy tool) | ✅ pass | Tool calling works: 15 chunks, text deltas, parsed `shell_command` tool call |
| `live_gpt6_astra_full_agent_path` (all ~25 production tools) | ❌ **FAIL** | `items=1 streamed_text_len=0 tool_call_events=0 final_len=Some(0)` — the exact TUI symptom |

The full-agent run captured the single SSE event the backend sent (raw-SSE debug patch in `rust/patches/rig-core/.../responses_api/streaming.rs`):

```json
{"type":"error","error":{"message":"Invalid schema for function 'AstGrepRefactor': At schema path /properties/transforms/type/0, object schema must define properties","type":"invalid_request_error","param":"tools","code":"invalid_function_parameters"}}
```

The backend validates **all** tool schemas and fails the whole request on the first offender — one bad tool kills the entire conversation.

## The offending schema (confirmed by schema dump)

`AstGrepRefactorArgs.transforms` is `Option<HashMap<String, Transform>>`
(`rust/tools/src/astgrep_refactor.rs:963`). schemars 0.8.22 generates:

```json
"transforms": {
  "type": ["object", "null"],
  "additionalProperties": { "$ref": "#/definitions/Transform" }
}
```

The backend's validator reads `type` array element `0` = `"object"`, finds no `properties` key → `invalid_function_parameters`. (The `Transform` enum's own `oneOf` variants are fine — they all define `properties`.)

Only `AstGrepRefactor` has `Option<HashMap<...>>` in its args today, but any future nullable object/map hits the same wall — hence the fix belongs in a **generic sanitizer**, not a one-off patch.

## How the official Codex client handles this (reference implementation)

Cloned at `/tmp/codex` (used for this comparison at the user's request).

### 1. SSE `error` events are parsed and surfaced — never dropped

`codex-rs/codex-api/src/sse/responses.rs:345-353`:

```rust
pub fn process_responses_event(
    event: ResponsesStreamEvent,
) -> std::result::Result<Option<ResponseEvent>, ResponsesEventError> {
    match event.kind.as_str() {
        "error" => {
            if let Some(error) = event.error.as_ref().and_then(parse_flex_unavailable) {
                return Err(ResponsesEventError::Api(error));
            }
        }
        ...
```

and the stream driver (`responses.rs:549`, `:576-580`) guarantees the error reaches the consumer — even if the server closes the stream after the error event:

```rust
let mut response_error: Option<ApiError> = None;
...
Ok(None) => {
    let error = response_error.unwrap_or(ApiError::Stream(
        "stream closed before response.completed".into(),
    ));
    let _ = tx_event.send(Err(error)).await;   // <- stream ENDS WITH AN ERROR, not silence
    return;
}
```

Error classification (codes → typed errors, retry advice) lives in
`codex-rs/codex-api/src/sse/responses_error.rs` (`parse_failed_response`, lines 26-103).

**Key invariant to copy:** *a stream must never end "successfully" without `response.completed`; if an error event was seen, the stream terminates with that error.* Our rig patch violates this — it treats an unparseable event as ignorable and lets the stream end empty.

### 2. Tool schemas are sanitized before sending — object types always get `properties`

`codex-rs/tools/src/json_schema.rs:47-52`:

```rust
fn prepare_tool_input_schema(input_schema: &JsonValue) -> JsonValue {
    let mut input_schema = input_schema.clone();
    sanitize_json_schema(&mut input_schema);
    prune_unreachable_definitions(&mut input_schema);
    input_schema
}
```

and the exact rule the backend enforces (`json_schema.rs:183-197`):

```rust
fn ensure_default_children_for_schema_types(
    map: &mut serde_json::Map<String, JsonValue>,
    schema_types: &[JsonSchemaPrimitiveType],
) {
    if schema_types.contains(&JsonSchemaPrimitiveType::Object) && !map.contains_key("properties") {
        map.insert("properties".to_string(), JsonValue::Object(serde_json::Map::new()));
    }
    ...
}
```

This is precisely the transformation our request lacks. Codex *never* sends an object-typed schema without `properties`, so it never triggers `invalid_function_parameters`.

### 3. Bonus finding (separate issue, not this bug)

`gpt-5.1-codex` fails differently on ChatGPT-plan auth:
`400 {"detail":"The 'gpt-5.1-codex' model is not supported when using Codex with a ChatGPT account."}` — surfaced correctly as an error (rig's non-streaming path). gpt-6-astra is the only Codex model usable with ChatGPT-plan OAuth; that 400 is expected backend policy.

---

## The fix (two parts, both mirroring codex)

### Part 1 — rig patch: surface SSE error events (fixes *silent* termination)

`rust/patches/rig-core/src/providers/openai/responses_api/streaming.rs` (~line 282):
before the generic untagged parse, detect `{"type":"error", ...}` events and
`yield Err(CompletionError::ProviderError(message))` — mirroring codex's
`"error" => Err(ResponsesEventError::Api(...))`. Also consider tracking a
`pending_error` so a stream that ends without `response.completed` terminates
with that error (codex's `response_error` pattern, `responses.rs:549/576`).

This alone converts every future backend rejection from *silent death* into a
visible, actionable error in the TUI.

### Part 2 — schema sanitizer: make requests acceptable (fixes *this* rejection)

Add a `sanitize_tool_schema(&mut serde_json::Value)` pass applied to every
`ToolDefinition.parameters` before the request is built (central place: the
provider/adapter layer that converts our tools for rig/OpenAI/Codex). Rule,
mirroring codex `ensure_default_children_for_schema_types`:

- any subschema whose `type` is `"object"` or whose `type` array contains `"object"`, and which lacks `properties`, gets `"properties": {}` inserted (keep `additionalProperties` as-is — it still governs map values);
- recurse through `properties`, `items`, `additionalProperties`, `prefixItems`, `anyOf`/`oneOf`/`allOf`, and `$defs`/`definitions`.

### Acceptance criteria (sketch for specifying phase)

1. Given a Codex-backend session with all production tools registered, when a prompt is sent, then the model responds (text and/or tool calls) — `live_gpt6_astra_full_agent_path` turns green.
2. Given a backend that rejects a request with an SSE `{"type":"error"}` event, when the stream is consumed, then the consumer receives an error containing the backend message (no silent empty completion).
3. Given any tool args struct with `Option<HashMap<...>>` / nullable object fields, the schema sent to the Codex backend contains `properties` on every object-typed node (unit test on the sanitizer + schema-dump regression test for `AstGrepRefactorArgs`).

### Test assets already in place

- Red test: `live_gpt6_astra_full_agent_path` (skips gracefully when no Codex OAuth on the machine).
- Controls: no-tools and single-tool tests (must stay green).
- Debug aid to revert when done: the `RAW SSE DATA` debug line added to the rig patch.
