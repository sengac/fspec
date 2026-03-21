# PROV-039 — Remaining Warnings for Future Review

**Date:** 2026-03-21
**Source:** PROV-039 ACDD review findings (review-findings.md)

These are non-blocking warnings identified during the PROV-039 review that were assessed as out-of-scope for the bug fix. They are documented here for future consideration.

---

## 1. Gemini provider does NOT propagate stop_reason through streaming

**Severity:** Enhancement (not a regression)
**Files:** `codelet/patches/rig-core/src/providers/gemini/streaming.rs:52-68,221-223`

The Gemini streaming handler does not override `stop_reason()` on the `GetTokenUsage` trait — it uses the trait default which returns `None`. Rule[1] in the PROV-039 feature file names Gemini as a provider that should propagate stop_reason, but Gemini never had this functionality (it wasn't broken by PROV-039, it was never implemented).

**Suggested fix:** Implement `stop_reason()` for Gemini streaming by mapping `FinishReason::MaxOutputTokens` → `StopReason::MaxTokens` and `FinishReason::Stop` → `StopReason::EndTurn`, similar to how Anthropic's streaming implementation works.

**Impact:** Without this fix, Gemini output truncation (max_tokens hit) will not be detected, and the session will silently treat truncated output as a normal completion.

---

## 2. Integration test `test_provider_manager_openai_max_output_tokens_env_var` is a tautology

**Severity:** Test quality
**File:** `codelet/providers/tests/stop_reason_propagation_test.rs:142-164`

The integration test reads the env var directly via `std::env::var()` instead of calling `ProviderManager::max_output_tokens()`. It doesn't actually test the production code path.

The real behavioral test exists as a unit test in `codelet/providers/src/manager.rs:631-647` which DOES call `manager.max_output_tokens()` and asserts correctly.

**Suggested fix:** Either rewrite the integration test to call `ProviderManager::max_output_tokens()`, or add a comment explaining this is a workspace-level coverage linkage test with the real behavior tested at the unit level.

---

## 3. Test `test_stop_reason_available_for_truncation_detection` is a tautology

**Severity:** Test quality
**File:** `codelet/providers/tests/stop_reason_propagation_test.rs:50-78`

Creates `StopReason::MaxTokens` and checks it equals `StopReason::MaxTokens`. This doesn't test the actual production behavior of extracting stop_reason from streaming responses.

The real truncation error enrichment test is `test_truncated_tool_call_json_produces_error` in `codelet/patches/rig-core/src/providers/anthropic/streaming.rs:1030`, which asserts the actual error message string.

**Suggested fix:** Either rewrite to test actual stop_reason extraction from a mock streaming response, or document the constraint (rig-core is not a workspace member) and mark as a coverage linkage test.

---

## 4. Shallow enum-level checks for first three scenarios

**Severity:** Test quality
**File:** `codelet/providers/tests/stop_reason_propagation_test.rs` (lines 9-11 header comment)

The test file header explains the constraint: rig-core is not a workspace member, so the workspace-level integration tests can only do shallow enum checks. Behavioral tests exist inside the rig-core source files and pass:

- `codelet/patches/rig-core/src/providers/anthropic/streaming.rs:935` — `test_message_delta_max_tokens_deserialization`
- `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:927` — `test_final_response_end_turn_stop_reason`
- `codelet/patches/rig-core/src/providers/anthropic/streaming.rs:1030` — `test_truncated_tool_call_json_produces_error`

**Suggested fix:** Consider making rig-core a workspace member (may have build implications), or accept the current pattern and link the rig-core tests in coverage metadata.

---

## 5. Persistence fallback silently defaults to "end_turn"

**Severity:** Defensive coding concern
**File:** `codelet/napi/src/session_manager.rs:3970`

The code uses `.or_else(|| Some("end_turn".to_string()))` as a fallback when `stop_reason` is `None`. This means if a provider fails to report a stop_reason, the session will record it as "end_turn" (normal completion) even though the real reason is unknown.

With the PROV-039 fix in place, `stop_reason` should always be populated for Anthropic and OpenAI. But for providers that don't implement it (like Gemini currently), this fallback masks the fact that stop_reason is unknown.

**Suggested fix:** Consider using a distinct sentinel value like "unknown" instead of "end_turn" to distinguish between "the API said it ended normally" and "we don't know why it ended". This would make it easier to identify providers that aren't propagating stop_reason.

---

## 6. rig-core third-party `unwrap()` calls

**Severity:** Pre-existing tech debt (third-party code)
**Files:** Multiple files in `codelet/patches/rig-core/src/`

The patched rig-core library contains many `unwrap()` calls in production code paths (HTTP client, provider implementations, streaming, tools). These are pre-existing in the upstream rig-core library and were not introduced by PROV-039.

Key locations:
- `codelet/patches/rig-core/src/http_client/mod.rs` — `response.text().await.unwrap()` in multiple error paths
- `codelet/patches/rig-core/src/tool/server.rs` — multiple `.unwrap()` calls on serialization
- `codelet/patches/rig-core/src/providers/groq.rs` — `function.name.clone().unwrap()`
- `codelet/patches/rig-core/src/providers/deepseek.rs` — `function.name.clone().unwrap()`
- `codelet/patches/rig-core/src/streaming.rs` — `serde_json::to_string_pretty(&res).unwrap()`

**Suggested fix:** Gradually replace these with proper error propagation as provider code is touched for other fixes. Low priority since these are in vendor-patched code.
