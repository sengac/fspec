# BUG-224 — Gemini contentless terminal candidates lose final evidence

## Summary and confidence
- Audit baseline: HEAD `7e60de8e`, 2026-10-21; source-only investigation.
- **Source-confirmed conditional defect; high confidence:** the patched Gemini stream parser skips a candidate without content before recording its terminal finish reason and final usage.
- A contentless terminal candidate can therefore produce `missing Gemini usage` even when its frame supplied usage metadata. Reproduction and tests were not executed; no builds or live requests were made.

## Source chain
1. `rust/patches/rig-core/src/providers/gemini/streaming.rs:110` initializes `final_usage` to None and `:112` initializes the captured stop reason.
2. `rust/patches/rig-core/src/providers/gemini/streaming.rs:125` deserializes a wire event. `:136` can emit a Usage item before inspecting candidates, so incremental usage can reach consumers even for the problematic frame.
3. `rust/patches/rig-core/src/providers/gemini/streaming.rs:143` selects the first candidate; `:148` checks `choice.content`, logs the finish reason, and `continue`s when content is absent.
4. `rust/patches/rig-core/src/providers/gemini/streaming.rs:192` is the later terminal check; `:194` normalizes the finish reason and `:210` assigns `final_usage = data.usage_metadata`. Neither runs for the skipped candidate.
5. `rust/patches/rig-core/src/providers/gemini/streaming.rs:214` treats StreamEnded as loop termination. After closing the source, `:229` emits `ResponseError("missing Gemini usage")` if no final usage was captured.
6. `rust/patches/rig-core/src/providers/gemini/streaming.rs:233` separately rejects inconsistent measured usage; `:237` publishes FinalResponse only after terminal evidence validation.

## Impact and boundaries
- The terminal stop reason and final response can be lost, while an earlier Usage event has already been emitted. Incremental usage is not equivalent to terminal evidence capture.
- A candidate with `content: { parts: [] }` is different: it reaches the terminal check. The problematic shape omits content or represents it as null.
- A later usable terminal candidate could supply evidence; the missing-usage outcome specifically requires stream end without such a candidate. Transport errors can instead terminate with the HTTP error path at `rust/patches/rig-core/src/providers/gemini/streaming.rs:217`.
- This is the patched built-in Gemini provider, not the custom Rhai bridge and not the built-in OpenAI provider identified for the screenshot incident.

## Reproduction outline — proposed, not executed
1. Feed an offline SSE fixture with a normal content delta and then a terminal candidate containing `finishReason: "STOP"`, valid `usageMetadata`, and no `content` field.
2. End the stream normally; collect RawStreamingChoice items and errors.
3. The source predicts incremental Usage followed by missing-usage failure instead of one successful terminal response with end-turn and measured usage.
4. Repeat with null content, MAX_TOKENS, safety finish reasons, and explicit measured zero; compare with an otherwise identical empty-parts candidate.
5. Retain negative cases with genuinely absent/inconsistent usage; do not turn those into successful estimated or zero-valued responses.

## Existing specifications/tests and missing regressions
- `spec/features/builtin-provider-terminal-usage-evidence.feature:23` requires Gemini missing/inconsistent terminal evidence to be rejected; `:28` requires explicitly measured zero to complete successfully.
- `rust/patches/rig-core/tests/cmpct065_terminal_usage_evidence.rs:212` covers rejected Gemini terminal evidence and `:245` covers explicit measured zero. Its Gemini fixture at `:131` always supplies content; contentless terminal success needs a separate regression.
- `rust/patches/rig-core/src/providers/gemini/streaming.rs:492` tests deserialization of empty parts with a present content object, not the contentless stream-control branch.
- `rust/patches/rig-core/src/providers/gemini/streaming.rs:570` tests response stop-reason access; `:612` tests normalization mappings. Neither exercises this early continue in a physical stream.
- `spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature:21` states the broader propagation invariant.
- Missing: physical-stream fixtures for missing/null content plus valid terminal usage, preservation of each normalized finish reason, exactly one terminal success, and truly absent/inconsistent usage remaining errors.

## Recommended direction
- Separate terminal metadata handling from optional content-part iteration, so a contentless candidate does not discard its finish reason or measured terminal usage.
- Preserve the terminal usage validation contract; do not infer final usage from arbitrary earlier frames or manufacture zero to suppress the error.
- Specify candidate-less usage-only frames and multiple-candidate behavior separately before broadening the first-candidate policy; the demonstrated defect needs no such expansion.

## Uncertainties and related cards
- Actual provider emission frequency and whether any incident used this frame shape are unknown. No screenshot or private capture establishes Gemini routing here.
- BUG-223 covers a later helper-completion bypass, not this parser failure. BUG-221 and BUG-222 concern thinking exhaustion/retry handling after other stop sequences; no causal link is established.
- CMPCT-065 owns terminal measured-usage evidence; PROV-039 owns stop-reason propagation. CONT-010 is a separate profile/session lifecycle question, not a reason to accept invalid terminal evidence.
