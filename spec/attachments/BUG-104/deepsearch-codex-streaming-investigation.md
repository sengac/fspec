# BUG-104 DeepSearch Codex streaming investigation

## Summary

`DeepSearch` was partially fixed under `BUG-102` so that the ephemeral sub-agent now:

- inherits the parent session's provider and model,
- supports `codex` and `zai` in provider dispatch,
- applies provider-aware request shaping via `deep_search_provider_config.rs`.

However, `DeepSearch` still fails for Codex at runtime with:

```json
{"detail":"Stream must be set to true"}
```

This means the previous fix addressed provider/model selection and request parameters, but not the execution mode required by the Codex Responses API path.

## User-visible failure

Observed when invoking the `DeepSearch` tool from a Codex-backed session:

- tool call starts successfully,
- provider is recognized as `codex`,
- handler builds a Codex request,
- backend rejects the request with HTTP 400 because `stream=true` is missing.

Observed error:

```text
DeepSearch sub-agent failed: Prompt failed: CompletionError: HttpError: Invalid status code 400 Bad Request with message: {"detail":"Stream must be set to true"}
```

## What BUG-102 already changed

Recent `SessionSearch` history and current unstaged diff show that the previous work changed:

- `codelet/napi/src/deep_search_handler.rs`
  - added provider dispatch for `codex` and `zai`,
  - switched DeepSearch agent construction to a provider-aware request config helper,
  - updated unsupported-provider messaging.
- `codelet/napi/src/deep_search_provider_config.rs`
  - introduced provider-specific request configuration,
  - Codex config sets `store=false`, `include=["reasoning.encrypted_content"]`, default reasoning, and no `max_output_tokens`.
- `codelet/tools/src/deep_search/tests.rs`
  - added tests for captured `codex` and `zai` provider/model inheritance.
- `spec/features/deep-search-model-inheritance.feature`
  - added Codex and Z.AI inheritance scenarios.
- `spec/features/deep-search-provider-request-configuration.feature`
  - added provider-specific request-shaping scenarios.

## Root cause

### 1. DeepSearch still uses the non-streaming execution path

In `codelet/napi/src/deep_search_handler.rs`, the ephemeral sub-agent is still executed through:

- `RigAgent::prompt(query)`

That eventually goes through the non-streaming path in:

- `codelet/core/src/rig_agent.rs`

Specifically, `RigAgent::prompt()` calls:

- `self.agent.prompt(prompt).multi_turn(self.max_depth).await`

### 2. Codex Responses API only sets `stream=true` in the streaming path

In the patched rig-core Responses API implementation:

- `codelet/patches/rig-core/src/providers/openai/responses_api/streaming.rs`

`stream` is explicitly set only in the streaming request builder:

- `request.stream = Some(true);`

The non-streaming path does not appear to force this field.

### 3. Codex backend now requires streaming for this request mode

The backend error is explicit:

- `Stream must be set to true`

So even with correct Codex request params, DeepSearch still fails because it invokes a non-streaming multi-turn agent call against a backend path that requires SSE/streaming semantics.

## Important implementation constraints

Any fix for `BUG-104` must preserve:

- the existing `DeepSearch` tool contract: returns a final synthesized string,
- the existing read-only tool set,
- the provider/model inheritance added for `BUG-102`,
- low observability/logging overhead,
- provider-specific request config behavior already added.

The fix should not regress:

- Claude,
- OpenAI,
- Gemini,
- Z.AI,
- existing DeepSearch tool tests.

## Relevant files

### Primary runtime path

- `codelet/napi/src/deep_search_handler.rs`
- `codelet/core/src/rig_agent.rs`
- `codelet/napi/src/deep_search_provider_config.rs`

### Provider/runtime details

- `codelet/providers/src/codex/mod.rs`
- `codelet/providers/src/codex/refreshing_client.rs`
- `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs`
- `codelet/patches/rig-core/src/providers/openai/responses_api/streaming.rs`

### Existing acceptance/tests

- `spec/features/deep-search.feature`
- `spec/features/deep-search-model-inheritance.feature`
- `spec/features/deep-search-provider-request-configuration.feature`
- `codelet/tools/src/deep_search/tests.rs`
- `codelet/napi/tests/codex_reasoning_config_test.rs`

## Reproduction notes

1. Start from a parent session using provider `codex`.
2. Invoke `DeepSearch` with a normal query and scope.
3. Observe that provider/model inheritance now succeeds.
4. Observe runtime failure with backend 400 requiring `stream=true`.

## Suggested direction for the fix

The likely correct fix is to make DeepSearch use a streaming-compatible execution path for Codex while still collecting and returning only the final synthesized answer string.

Potential shape:

- keep `DeepSearch` result contract as `String`,
- internally run the Codex sub-agent through streaming execution,
- consume the stream to completion,
- capture the final response text,
- return that final text to the tool caller,
- keep other providers on their current compatible path unless a shared abstraction is cleaner and safe.

A secondary option would be to make the non-streaming Responses API request include `stream=true` if rig-core and the request/response handling support that cleanly, but the current code strongly suggests the streaming pathway is the intentional place where this flag is set.

## ACDD implications

This bug needs new acceptance coverage because `BUG-102` covered:

- provider/model inheritance,
- provider-specific request shaping,

but not:

- Codex-specific execution-mode compatibility for DeepSearch.

New scenarios should cover at least:

- Codex DeepSearch sub-agent uses a streaming-compatible execution path,
- DeepSearch still returns a final synthesized string to the parent caller,
- existing non-Codex providers remain unaffected.

## Session history reference

Recent `SessionSearch` history confirms the previous work ended after concluding the bug was fixed, but before verifying an actual Codex-backed `DeepSearch` run end-to-end. The current failure demonstrates that the earlier fix was necessary but not sufficient.
