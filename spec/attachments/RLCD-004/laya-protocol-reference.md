# Jev / Simple-Jev Classifier Protocol — v1 Reference

The protocol contract this server implements: the open-source
**Simple-Jev** HTTP API (`featherless-ai/simple-jev`, `hf-server/API_REFERENCE.md`
+ `common/request_schema.py`, prompt template version `v1`). Source of the
schemas below (fetched 2026-09-22):

- https://github.com/featherless-ai/simple-jev — README (Laya backend section), `common/` (shared v1 contract)
- https://simple-jev.featherless.ai/docs — client-facing API documentation
- https://www.jevai.org/docs — TypeSafe's official (hosted, closed) Jev API for comparison

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/v1/classifier` | Score the supplied questions. |
| POST | `/v1/systemone` | Exact alias of `/v1/classifier`. |
| GET | `/health` | `{"status":"ready","model":"<loaded model>"}` after service init (no inference probe). |

Transport: `Content-Type: application/json`, non-streaming JSON responses.
No query parameters, no required custom headers, no authentication in the
reference (the laya-rs server matches: no auth).

## Request body (JSON Schema)

```json
{
  "model":   { "type": "string", "minLength": 1,
               "description": "Must equal the model ID/path the server was started with." },
  "state":   { "anyOf": [
                 { "type": "string" },
                 { "type": "object" },
                 { "type": "array" }
               ],
               "description": "Shared context. Supply exactly one of state|messages (non-null)." },
  "messages": { "type": "array", "minItems": 1,
                "items": { "$ref": "#/definitions/chat_message" },
                "description": "Alternative to state: text chat history." },
  "questions": { "type": "object", "minProperties": 1, "maxProperties": 256,
                 "additionalProperties": { "$ref": "#/definitions/question" },
                 "description": "Question ID -> question definition. IDs become answer keys; insertion order is retained." },
  "options":   { "$ref": "#/definitions/options", "default": {} },
  "tools":             { "type": "array", "default": null,
                 "description": "Reserved; nonempty values are rejected by the laya backend." },
  "mm_processor_kwargs": { "type": "object", "default": null,
                 "description": "Reserved; nonempty rejected." },
  "media_io_kwargs": { "type": "object", "default": null,
                 "description": "Reserved; nonempty rejected." }
}
```

Unknown **top-level** fields are ignored (including `stream`, `temperature`,
`max_tokens`, etc.). Unknown **question** and **option** fields are rejected (422).

```json
{
  "chat_message": {
    "type": "object",
    "properties": {
      "role": { "enum": ["system", "developer", "user", "assistant"] },
      "content": { "type": "string" }
    },
    "required": ["role", "content"],
    "additionalProperties": false,
    "note": "laya-rs: text-only messages; image/audio/tool-call content and extra fields rejected (422 'text messages')."
  },

  "question": {
    "oneOf": [
      { "type": "choice" }, { "type": "score" }, { "type": "noul" }
    ],
    "discriminator": "type",
    "note": "Every question requires `type` plus `instructions`."
  },

  "choice_question": {
    "type": "object",
    "properties": {
      "type": { "const": "choice" },
      "instructions": { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] },
      "criteria": { "type": "object", "minProperties": 2, "maxProperties": 50,
                    "additionalProperties": { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] } }
    },
    "required": ["type", "instructions", "criteria"],
    "additionalProperties": false,
    "note": "Dictionary keys are the public answer IDs; insertion order assigns model labels and breaks exact ties. Null descriptions allowed."
  },

  "score_question": {
    "type": "object",
    "properties": {
      "type": { "const": "score" },
      "instructions": { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] },
      "criteria": { "type": "array", "minItems": 2, "maxItems": 50,
                    "items": { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] } }
    },
    "required": ["type", "instructions", "criteria"],
    "additionalProperties": false,
    "note": "Ordered rubric, lowest level first; order is meaningful and preserved."
  },

  "noul_question": {
    "type": "object",
    "properties": {
      "type": { "const": "noul" },
      "instructions": { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] },
      "criteria": { "type": "object",
                    "properties": {
                      "true":  { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] },
                      "false": { "anyOf": [ {"type":"string"}, {"type":"object"}, {"type":"array"}, {"type":"null"} ] }
                    },
                    "additionalProperties": false }
    },
    "required": ["type", "instructions"],
    "additionalProperties": false,
    "note": "criteria optional; when present only the keys 'true'/'false' are accepted (either or both)."
  },

  "options": {
    "type": "object",
    "properties": { "raw_logits": { "type": "boolean", "default": false } },
    "required": ["raw_logits"],
    "additionalProperties": false,
    "note": "laya-rs: options.raw_logits=true is rejected (422) — raw-logit diagnostics are not supported on the laya backend."
  }
}
```

### Context XOR rule

Exactly one of `state` (string, JSON object, or array — null is *not* supplied) or
`messages` (nonempty array) must be present. Both set, both null, or neither → 422
`"Provide exactly one of state or messages"`. An empty string or empty JSON
object/array *does* count as supplied `state`.

### Lay-a backend admission rules (enforced before inference)

- `request.model` must equal the loaded model's ID/path, else 422 with message
  `Loaded model is '<name>'`.
- Each compiled question sequence must fit `min(--max-model-len, checkpoint
  native max_len)`; there is **no automatic truncation** at the protocol layer —
  overflow is a 422 (laya-rs's `build_sequence` truncates internally, so the
  server must pre-check and reject rather than silently shorten).
- Question count ≤ `--max-request-branches` (default 100; the schema hard cap
  is 256) → 422 otherwise.
- Non-empty `tools`, `mm_processor_kwargs`, `media_io_kwargs`,
  `options.raw_logits` → 422 (media/tools/raw-logits unsupported on laya).
- Text-only chat: message content must be a string; image/audio/video parts,
  tool/function roles, `name`, or other extra message fields → 422
  (reference error phrase: "text messages").

## Response body (200)

```json
{
  "model":   "convaiinnovations/laya",
  "answers": {
    "qid_choice": {
      "type": "choice",
      "choice": "billing",
      "confidence": 0.8,
      "probabilities": { "billing": 0.8, "technical": 0.15, "account": 0.05 }
    },
    "qid_score": {
      "type": "score",
      "score": 1.75,
      "confidence": 0.8,
      "probabilities": { "0": 0.05, "1": 0.15, "2": 0.8 },
      "legend": { "0": "Routine", "1": "Important", "2": "Critical" }
    },
    "qid_noul": { "type": "noul", "noul": 0.9 }
  },
  "usage": { "input_tokens": 600, "output_tokens": 0 }
}
```

### Answer field rules (laya-native backend semantics)

- **choice**: `choice` = candidate ID with the greatest (temperature-scaled)
  probability; exact ties select the first candidate (insertion order).
  `confidence` = winning candidate's probability. `probabilities` maps every
  candidate ID to its probability, summing ≈ 1. **No `act_probability` field**
  (the laya SDK's action fields are omitted by the reference backend; laya-rs
  emits one today — the server strips it).
- **score**: `score` = expected zero-based rubric index `sum(p[i] * i)`
  (fractional allowed; range 0..N-1). `confidence` = largest criterion
  probability (NOT a CI). `probabilities` keyed by numeric strings `"0"`..
  `"<N-1>"`; `legend` maps those same numeric strings to the original criteria.
- **noul**: `noul` ∈ [0, 1] = the native binary **positive-class** (true)
  probability. No separate confidence field. (The v1 nine-bin 0.01–0.99
  mapping applies only to the LLM backends; the laya backend uses its native
  P(true).)
- Probabilities retain the checkpoint's per-(qtype, option-count) temperature
  calibration. Scores/confidences are not calibrated probabilities of
  correctness.

### Usage accounting

- `usage.input_tokens` = sum of the actual per-question sequence lengths
  (the laya backend re-tokenizes context per question, so repeated context is
  counted — NOT the shared-prefix "count once" the LLM backends use).
- `usage.output_tokens` = always `0` (no tokens are sampled).

## Errors

Shared 4xx envelope (JSON):

```json
{
  "error": {
    "message": "summary (schema errors: up to 10 details folded)",
    "type": "invalid_request_error",
    "code": 422,
    "param": "questions.color.criteria",
    "details": [ { "param": "questions.color.criteria", "message": "...", "type": "..." } ]
  }
}
```

- `param` is a dotted field path; array indices appear as `[0]`. The top-level
  `param` is the first detail's path. Details omit submitted input values.
- 429 body is the short form `{"detail": "Scoring queue is full"}` with header
  `Retry-After: 1`.
- 499 `{"detail": "Client disconnected"}` (only if a response can still be
  delivered).
- 500: unhandled runtime failure; no stable structured body guaranteed.

| Status | Meaning |
| --- | --- |
| 422 | Invalid JSON/schema, unknown model, invalid context combination, unsupported chat/media/tool input, branch/token limits |
| 429 | Request queue full |
| 499 | Client disconnected (best effort) |
| 500 | Unhandled runtime failure (e.g. model execution error) |

## Server startup arguments (reference CLI, for parity)

| Argument | Reference default | laya-rs mapping |
| --- | --- | --- |
| `--model` (required) | — | `--model` / `--model-variant` / `--models-root` via `model_path::resolve` |
| `--host` | `127.0.0.1` | `--host` (same default) |
| `--port` | `8000` | `--port` (same default) |
| `--max-model-len` | `16384` | `--max-model-len` (default: checkpoint `max_len`) |
| `--max-request-branches` | `100` | `--max-request-branches` (same default) |
| `--max-batch-size` / `--max-batch-tokens` | suffix-batching (LLM only) | N/A — laya batches all questions in one forward |
| admission queue | 16 waiting | `--max-queued` (default 16) |

## Differences from TypeSafe's official hosted API (out of scope)

- Hosted: `POST https://www.jevai.org/api/v1/decisions*` with Bearer keys,
  `{code, message, data}` envelope, 32 KiB body cap, and five preset endpoints
  (`tool-guard`, `model-route`, `route`, `research`, `completion`) that are
  canned question sets over the same native `{state, questions}` endpoint
  (`/api/v1/decisions`, model `typesafe-ai/jev`).
- This server implements the open Simple-Jev contract; the native
  `{state, questions}` shape is a subset of it, so simple-jev clients and the
  TypeSafe-style payloads are both compatible with `/v1/classifier`.
