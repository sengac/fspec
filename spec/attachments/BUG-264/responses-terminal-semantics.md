# BUG-264: Generic OpenAI Responses terminal semantics

## Scope and confidence

Source-confirmed latent adapter issue. Ordinary patched-rig Responses is distinct
from the dedicated CodexResponsesModel. Native fspec OpenAIProvider constructs
Chat Completions; no separate native non-Codex Responses builder was found.
Do not attribute this finding directly to a live Codex session.

## Evidence

- `rust/patches/rig-core/src/providers/openai/responses_api/streaming.rs:128`
  declares failed/incomplete terminal event variants.
- Line 415 processes only ResponseCompleted, discarding other terminal semantics.
- Line 85 lacks a stop_reason override; the trait default is None in
  `rust/patches/rig-core/src/completion/request.rs:286`.
- Line 454 emits buffered function items before usage validation at lines 458–464.
- Accepted Done can terminate consumption before that later validation.
- BUG-208 (`38a95522`) handles type:error, not response.failed/incomplete.
  `2699693b` added usage validation, not full terminal-state propagation.

## Acceptance/test direction

Exercise actual SSE for completed, failed, incomplete/max_output_tokens,
content filtering and explicit type:error, with and without queued tool calls
and valid usage. Preserve backend error details and normalized stop/recovery
classification. Validate terminal evidence before accepting Done; retain immediate
completion and correct tool call_id/result history. Test metadata-only endings
where the API permits them instead of inventing success from EOF.

Keep dedicated Codex tests as regression controls, not as proof of this adapter.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
