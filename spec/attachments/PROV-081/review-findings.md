# Review: PROV-081 — OpenAI provider drops vLLM-native reasoning/thinking tokens (streaming + non-streaming)

**Date:** 2026-04-19
**Reviewer:** Claude Code (fspec review skill — single-work-unit mode)
**Review phase:** Phase 3 — dry-review only, no code modifications performed.

## Status: PASS

All 8 scenarios are implemented, tested, and traceable to source lines. `cargo build --lib` and `cargo test --lib` both succeed; the 8 PROV-081 tests pass and all 210 library tests pass with 0 failures. Feature file, example map, implementation, and test code are in tight alignment with no critical or warning-level issues.

## 🔴 Critical Issues (Must Fix)

None.

## 🟡 Warnings (Should Fix)

None.

## 🟢 Observations (Nice to Have)

1. **Concatenation order is implementation-defined, not specified in Gherkin.** Rule [2] and architecture note [0] state reasoning should be concatenated "reasoning_content first, then reasoning" when both fields arrive on the same chunk. The implementation at `streaming.rs:308-314` does follow that order, but the feature-file Then step (line 68) only asserts that the reasoning contains both "A" and "B" without order — so the spec is actually weaker than the architecture note. The test at `mod.rs:1708-1750` matches the feature (unordered containment + no duplication), which is fine. Consider tightening the Gherkin if strict ordering ever matters downstream.

2. **Streaming concat path allocates even on non-reasoning chunks.** `streaming.rs:308` creates a `String::new()` on every delta, even when neither reasoning field is present. Micro-cost only (empty `String` is zero-alloc until first push), but an explicit early-out branch (`if delta.reasoning_content.is_none() && delta.reasoning.is_none()`) would make the hot path intent clearer. No functional impact.

3. **Outgoing-body negative assertion is defensive rather than proactive.** The test `prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys` (mod.rs:1598-1621) correctly asserts that the default-built body contains neither `include_reasoning` nor `chat_template_kwargs.enable_thinking`. However, the OpenAI provider accepts caller-supplied `additional_params` that flatten into the request (mod.rs:1033-1034). A caller could still set these keys via `additional_params` and silently defeat reasoning capture. Architecture note [2] acknowledges this as a passthrough and tracks it as documentation. Consider either (a) a warning log if these keys are found in `additional_params`, or (b) an explicit doc comment on `OpenAIRequestParams` pointing at PROV-081. Not a defect — just surface-area clarification.

4. **Assistant-side reasoning field relies on `skip_serializing_if = "Option::is_none"`** (mod.rs:147). When an assistant message is sent BACK upstream via `TryFrom<OneOrMany<message::AssistantContent>> for Vec<Message>` (mod.rs:567-581), `reasoning` is hardcoded to `None`. Combined with the pre-existing `panic!` at mod.rs:553 on outbound `AssistantContent::Reasoning`, this correctly prevents reasoning from being serialized back to the server. The invariant chain is: inbound capture → CompletionResponse → outbound drop-or-panic. Consider adding a one-line comment at mod.rs:575 explicitly citing PROV-081 so future refactors don't accidentally plumb reasoning outbound (the panic at :553 is the last line of defense but isn't self-documenting).

5. **@step docstring line length is long.** Feature-file steps like line 49 and 66 contain raw JSON payloads which produce very long lines (>120 chars). The @step comments in `mod.rs` (e.g. lines 1467, 1514) replicate them verbatim, as required by the skill's "EXACT match" rule. This is intentional and correct — no action needed, but it's worth noting for future reviewers who might be tempted to "clean up" these long lines.

## Coverage Verification

- **Feature file**: `spec/features/openai-provider-reasoning-tokens.feature` — OK
  - `@PROV-081` tag present on feature line 2 ✅
  - Architecture doc string present (lines 5-11) ✅
  - Background present with As-a/I-want/So-that user story (lines 41-44) ✅
  - All 8 scenarios: Given steps precede When, When precedes Then, And-after-Then are assertions (never preconditions) ✅
  - No placeholder text (`[role]`, `[action]`, `[benefit]`, etc.) detected ✅
  - 6 rules, 12 examples, no open questions, 5 architecture notes ✅
  - `fspec validate` reports feature file as valid ✅

- **Test file**: `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` — OK
  - Header at lines 1279-1289 references `spec/features/openai-provider-reasoning-tokens.feature` ✅
  - 8 tests under `#[cfg(test)] mod prov_081_tests` (lines 1290-1789) — one per scenario ✅
  - Every @step comment matches the Gherkin step text EXACTLY (verified via grep at lines 1466,1467,1490,1494,1502,1513,1514,1537,1541,1549,1556,1557,1576,1582,1586,1593,1599,1600,1603,1612,1628,1629,1635,1638,1654,1661,1670,1671,1677,1680,1696,1709,1710,1716,1719,1739,1757,1758,1764,1767,1777,1787) ✅
  - Tests verify real behavior (deserialize real JSON, drive SSE through the real streaming decoder, serialize real request bodies) — no trivial `expect(true).toBe(true)`-style assertions ✅
  - `MockSseClient` + `collect_streamed_contents` + `build_request_body_json` are proper integration-adjacent helpers that exercise production code paths ✅

- **Impl file(s)**:
  - `codelet/patches/rig-core/src/providers/openai/completion/streaming.rs` — OK
    - New `reasoning: Option<String>` field added at lines 42-47 with PROV-081 comment ✅
    - Consumption at lines 303-321 concatenates `reasoning_content` then `reasoning`, skips emit if both empty (rule 3 regression safety) ✅
  - `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` — OK
    - `Message::Assistant` gains `reasoning: Option<String>` with `alias = "reasoning_content"` and `skip_serializing_if = "Option::is_none"` at lines 138-149 ✅
    - `TryFrom<CompletionResponse>` extraction at lines 770-807 surfaces `AssistantContent::reasoning(...)` BEFORE text and tool calls ✅
    - Outbound `TryFrom<OneOrMany<message::AssistantContent>> for Vec<Message>` at mod.rs:567-581 sets `reasoning: None` — paired with the pre-existing panic at :553 guarding assistant-side reasoning serialization ✅

- **Scenario coverage**: 8/8 scenarios covered (100%), per `fspec show-coverage openai-provider-reasoning-tokens`

## Build & Test Verification

- `cargo build --lib` — **PASS** (0.06s, clean)
- `cargo test --lib prov_081` — **PASS** (8/8 tests pass)
  - `prov_081_non_streaming_vllm_reasoning_field_surfaces_reasoning_and_content` ✅
  - `prov_081_non_streaming_glm_reasoning_content_field_surfaces_reasoning_and_content` ✅
  - `prov_081_non_streaming_without_reasoning_field_still_works` ✅
  - `prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys` ✅
  - `prov_081_streaming_vllm_reasoning_field_surfaces_as_reasoning_delta` ✅
  - `prov_081_streaming_glm_reasoning_content_field_surfaces_as_reasoning_delta` ✅
  - `prov_081_streaming_concatenates_reasoning_when_both_fields_present` ✅
  - `prov_081_streaming_content_chunk_passes_through_unchanged` ✅
- `cargo test --lib` (full suite) — **PASS** (210 passed, 0 failed, 3 ignored)
  - No regressions introduced. Pre-existing `test_streaming_usage_only_chunk_is_not_ignored` still passes.

## Implementation Quality Assessment

### SOLID / DRY

- **Single Responsibility** — `StreamingDelta` and `Message::Assistant` each gained exactly one field; consumption logic stayed in the same locations (streaming.rs:303-321, mod.rs:770-807). No responsibilities leaked across types. ✅
- **DRY** — The concatenation logic appears once (streaming.rs:308-314). Non-streaming path uses serde `alias` to avoid a parallel concatenation pathway, which is the simplest possible design. `AssistantContent::reasoning(...)` factory (from `crate::completion`) is reused, not re-implemented. ✅
- **Open/Closed** — Adding a third reasoning-field variant in future (e.g. a new provider using yet another name) can be done by extending the serde alias list or adding another `Option<String>` field — no shape changes required elsewhere. ✅
- **DIP** — The patch depends on serde's `Deserialize` abstraction, not on any provider-specific type. ✅

### No shortcuts

- No `todo!()`, `unimplemented!()`, `FIXME`, `HACK`, or `XXX` introduced. The pre-existing `TODO` at mod.rs:641 (refusals-to-text) is not PROV-081 scope and is untouched. ✅

### Wired up end-to-end

- Streaming: request → `send_compatible_streaming_request` → `StreamingDelta::deserialize` → consumption at streaming.rs:303-321 → `RawStreamingChoice::ReasoningDelta` → `StreamedAssistantContent::ReasoningDelta` (reachable by user; verified by test `collect_streamed_contents`). ✅
- Non-streaming: response → `CompletionResponse::deserialize` → `TryFrom<CompletionResponse> for completion::CompletionResponse` at mod.rs:762-847 → `completion::AssistantContent::reasoning(...)` → caller-visible `OneOrMany<AssistantContent>` (verified by test `extract_reasoning_texts`). ✅
- Request-side guard: `CompletionRequest` has no `include_reasoning` or `chat_template_kwargs` fields anywhere (verified via grep; only references are in test assertions and comments). ✅

### Type safety & error handling

- All new fields are `Option<String>` — no `unwrap()` introduced in production code. The three `unwrap_or`, `unwrap_or_default` uses on streaming.rs:222, 388 are pre-existing and unrelated. ✅
- `.expect("...")` calls in streaming.rs:122-123 are pre-existing and appropriate (serializing a Rust struct to JSON cannot fail for well-formed types). ✅
- The `panic!` at mod.rs:553 is the intentional guard described in the work-unit description ("the outbound panic ... is intentional, leave alone"). ✅
- No `todo!`, `unimplemented!`, or new `unwrap()` anywhere in the PROV-081 diff. ✅

### File size

- `completion/mod.rs` is now 1790 lines (pre-existing size was already >300 lines; the ~270 lines added are test code in a new `mod prov_081_tests`). Per review brief: "flag only NEW large additions" — the additions here are reasonably scoped (8 tests + ~135 lines of reusable test infrastructure `MockSseClient`, `collect_streamed_contents`, `build_request_body_json`, `extract_reasoning_texts`, `extract_text_contents`). A future refactor could extract `mod prov_081_tests` into a separate `tests/prov_081.rs` integration-test file, but this is not a blocker.
- `completion/streaming.rs` is 692 lines; PROV-081's additions total ~20 lines of production code plus no new test code. ✅

### Rust-specific quality (per skill brief)

- No `unwrap()` in production code introduced by PROV-081. ✅
- No `todo!()` / `unimplemented!()` introduced. ✅
- Proper `Result` types preserved — all serde paths use `Result`, no `.unwrap()` on deserialization in production code. ✅
- No dead code: `reasoning` field is actively consumed at streaming.rs:312. ✅
- No unused imports introduced. ✅

### Cross-cutting concerns

- **Duplication check:** Grep for `reasoning_content` across `codelet/patches/rig-core/` returned only PROV-081 sites (streaming.rs:41, mod.rs:146, tests). No competing parallel implementation. ✅
- **Architecture-note alignment:**
  - Note [0] (streaming fix via both fields kept distinct + concat at consumption site) — matches streaming.rs implementation exactly ✅
  - Note [1] (Message::Assistant gains `reasoning` w/ alias, extraction propagates to `completion::AssistantContent::reasoning`) — matches mod.rs implementation exactly ✅
  - Note [2] (request-side guard — no `include_reasoning` / `chat_template_kwargs.enable_thinking` emitted) — implementation builds request with neither field; test `prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys` asserts it ✅
  - Note [3] (server evidence, no in-project changes) — no vLLM changes in the PR, correctly out-of-scope ✅
  - Note [4] (test strategy: canned JSON + SSE fixtures, run via `cargo test`) — test structure matches ✅
- **Security:** No secrets logged. `tracing::trace!` at streaming.rs:133-135 pretty-prints the request body at TRACE level only (gated by `enabled!(Level::TRACE)`), which is the pre-existing behavior and does not expose new sensitive fields (reasoning is inbound-only). ✅
- **Performance:** No unbounded loops introduced. Streaming concat path uses a single `String::push_str` pass per chunk (O(n) in reasoning text length). No pagination concerns. ✅

## Files Reviewed

- `spec/features/openai-provider-reasoning-tokens.feature` (110 lines — full read)
- `spec/attachments/PROV-081/vllm-research.md` (295 lines — full read)
- `codelet/patches/rig-core/src/providers/openai/completion/streaming.rs` (692 lines — full read)
- `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` (1790 lines — targeted reads of lines 100-320, 530-760, 786-1105, 1280-1440, 1440-1790 + full grep for `@step`, `panic!`, `unwrap`, `reasoning_content`, `include_reasoning`, `chat_template_kwargs`, `Feature: spec/features`)
- `spec/skills/review-skill.md` (read in full for checklist)
- Fspec commands: `show-work-unit PROV-081`, `show-coverage openai-provider-reasoning-tokens`, `show-deleted PROV-081`, `validate`
- Cargo: `cargo build --lib`, `cargo test --lib prov_081`, `cargo test --lib` (full suite, 210 tests)

---

**Confirmation of dry-review discipline:** I did NOT modify any source code, test code, feature files, or example-mapping data during this review. This is a read-only Phase 3 report. All findings above identify the current state; the orchestrator may invoke Phase 4 if any issues warrant action (none critical here).
