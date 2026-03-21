# PROV-039 — Remaining Warnings — ALL RESOLVED

**Date:** 2026-03-21
**Source:** PROV-039 ACDD review findings (review-findings.md)
**Status:** All items resolved as of 2026-03-21

---

## 1. ✅ Gemini provider now propagates stop_reason through streaming

**Resolution:** Implemented in `codelet/patches/rig-core/src/providers/gemini/streaming.rs`

- Added `stop_reason: Option<String>` field to `StreamingCompletionResponse`
- Overrode `fn stop_reason()` on `GetTokenUsage` impl
- Added `captured_stop_reason` variable in streaming loop
- Mapped Gemini `FinishReason` variants to normalized strings:
  - `Stop` → `"end_turn"`
  - `MaxTokens` → `"max_tokens"`
  - `Safety`/`Recitation`/`Language`/`Blocklist`/`ProhibitedContent`/`Spii` → `"content_filter"`
  - `MalformedFunctionCall` → `"end_turn"`
  - `Other`/`FinishReasonUnspecified` → `"unknown"`
- Added unit tests: `test_streaming_completion_response_stop_reason_max_tokens`, `test_streaming_completion_response_stop_reason_end_turn`, `test_streaming_completion_response_stop_reason_none`, `test_finish_reason_to_stop_reason_mapping`
- Reference: VTCode `vtcode-core/src/llm/providers/gemini/helpers.rs:433-439`

---

## 2. ✅ Integration test now calls ProviderManager::max_output_tokens()

**Resolution:** Rewrote `codelet/providers/tests/stop_reason_propagation_test.rs`

- Added `ProviderManager::for_testing(provider)` constructor (doc-hidden, no credentials needed)
- Test `test_provider_manager_openai_max_output_tokens_env_var` now calls `manager.max_output_tokens()` and asserts the result
- Added `test_provider_manager_openai_max_output_tokens_default` verifying fallback to 4096
- Added `test_provider_manager_openai_max_output_tokens_invalid_env_var` verifying graceful handling of non-numeric values

---

## 3. ✅ Truncation detection test now tests the actual predicate

**Resolution:** Rewrote `test_truncation_detection_predicate` in integration tests

- Tests the actual truncation detection predicate: `stop_reason == MaxTokens && json_parse_failed`
- Verifies true positive (MaxTokens + bad JSON → truncation detected)
- Verifies true negative (EndTurn + bad JSON → NOT truncation)
- Verifies true negative (MaxTokens + valid JSON → NOT truncation)

---

## 4. ✅ Shallow enum-level checks replaced with meaningful tests

**Resolution:** All five integration test scenarios rewritten to test real behavior

- `test_stop_reason_variants_map_to_correct_persistence_strings` — exhaustive variant distinctness + persistence string mapping
- `test_truncation_detection_predicate` — actual predicate logic with positive and negative cases
- `test_normal_end_turn_not_detected_as_truncation` — round-trip persistence string verification
- `test_openai_finish_reason_string_mapping` — contract verification for finish_reason → stop_reason mapping
- Header comment updated documenting rig-core behavioral test locations

---

## 5. ✅ Persistence fallback now uses "unknown" sentinel value

**Resolution:** Changed `codelet/napi/src/session_manager.rs:3984`

- Was: `.or_else(|| Some("end_turn".to_string()))`
- Now: `.or_else(|| Some("unknown".to_string()))`
- This distinguishes "the API said it ended normally" from "we don't know why it ended"
- With Gemini now propagating stop_reason, this fallback should rarely trigger

---

## 6. ✅ rig-core third-party unwrap() calls replaced with proper error handling

**Resolution:** Fixed across 5 files

### `codelet/patches/rig-core/src/http_client/mod.rs`
- Added `error_body_text()` helper that uses `unwrap_or_else` with fallback string
- All 6 `response.text().await.unwrap()` calls replaced with `error_body_text(response).await`
- Both `send_streaming` `.build().unwrap()` calls replaced with `?` error propagation
- `self.clone()` moved before `async move` block to fix borrow semantics

### `codelet/patches/rig-core/src/tool/server.rs`
- All 4 `callback_channel.send(...).unwrap()` calls replaced with `let _ = callback_channel.send(...)`
- `get_tool_definitions(prompt).await.unwrap()` replaced with match-based error handling

### `codelet/patches/rig-core/src/providers/groq.rs`
- `function.name.clone().unwrap()` → `function.name.clone().unwrap_or_default()`
- `serde_json::to_string(...).unwrap()` → `serde_json::to_string(...).unwrap_or_default()` (tracing span)
- Transcription request `.body(body).unwrap()` → `.map_err(...)? ` 
- Transcription response `.send_multipart(...).unwrap()` → `.map_err(...)?`

### `codelet/patches/rig-core/src/providers/deepseek.rs`
- `function.name.clone().unwrap()` → `function.name.clone().unwrap_or_default()`
- `serde_json::to_string(&message).unwrap()` → `serde_json::to_string(&message).unwrap_or_default()` (tracing span)

### `codelet/patches/rig-core/src/streaming.rs`
- `PauseControl::pause()` and `resume()`: `self.paused_tx.send(...).unwrap()` → `let _ = self.paused_tx.send(...)`
- `serde_json::to_string_pretty(&res).unwrap()` → `.unwrap_or_default()` (logging path)

**Not changed (acceptable):**
- `from_env()`/`from_val()` trait impls in groq.rs/deepseek.rs — these are `ProviderClient` trait methods that panic by design (trait returns `Self`, not `Result`)
- `unwrap()` calls inside `#[test]` functions — standard test practice
