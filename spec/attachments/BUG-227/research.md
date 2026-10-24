# BUG-227 — Custom Rhai SSE errors can become successful EndTurn

## Summary and confidence
- Audit baseline: HEAD `7e60de8e`, 2026-10-21; read-only source audit.
- **Source-confirmed conditional defect; high confidence:** the custom Rhai HTTP streaming bridge logs and skips SSE-stream errors; after exhaustion it flushes normally. The rig adapter defaults an absent stop reason to EndTurn and emits FinalResponse.
- Thus a body/transport error surfaced by the SSE adapter can be concealed as normal completion if no explicit error reaches the next layer. This is not an executed reproduction or an established cause of a missing nudge.
- No tests, builds, endpoint calls, or private log/configuration extraction were performed.

## Source chain
1. `rust/providers/src/custom/stream_http.rs:27` accepts a byte stream whose items can be errors; `:63` converts it with `eventsource()`.
2. `rust/providers/src/custom/stream_http.rs:67` polls SSE events. The Err branch at `:70` logs a frame-parse warning then `continue`s at `:72`; no ProviderError is yielded.
3. `rust/providers/src/custom/stream_http.rs:109` handles stream exhaustion by calling `processor.finish()` at `:113`, yielding its chunks as Ok regardless of earlier skipped SSE errors.
4. `rust/providers/src/custom/stream.rs:170` marks the processor terminated and flushes pending state. `:194` emits StopReason only if an explicit pending reason exists; it does not establish transport success.
5. `rust/providers/src/custom/rig_streaming.rs:64` wires this bridge into the custom provider. `:75` initializes `final_stop = StopReason::EndTurn`; `:104` changes it only for a StopReason chunk.
6. `rust/providers/src/custom/rig_streaming.rs:105` correctly propagates yielded ProviderErrors, but skipped SSE errors never reach that arm. `:108` unconditionally emits a successful FinalResponse when the chunk stream exhausts.
7. Contrast: `rust/providers/src/custom/stream_http.rs:44` maps non-2xx HTTP responses to Err; `:98` yields and terminates on Rhai processing errors. Those established error paths are not the swallowed-SSE branch.

## Impact and provider boundaries
- Interrupted custom-provider output can appear normally completed, hiding partial output and preventing error-specific reporting/recovery at higher layers.
- The conditional EndTurn outcome requires that no explicit stop reason or propagated processor error supersede it. If a stop reason was seen first, the transport error is still concealed but the final reason may differ.
- This code is the custom Rhai adapter, not built-in OpenAI streaming. The supplied incident routing identifies built-in OpenAI; this defect is not attribution for that screenshot/provider.
- A fabricated clean completion does **not** itself prove no auto-continue nudge: the main completion engine may nudge on an armed clean stop (`rust/cli/src/interactive/stream_loop.rs:1615`). Continue mode, budgets, goals, and other branches still matter.

## Reproduction outline — proposed, not executed
1. Use an offline byte-stream fixture for `open_stream` with HTTP 200, one valid text SSE frame, then a deterministic body-stream Err, and exhaustion without stop/DONE.
2. Collect all bridge outputs; the source predicts the error is logged/skipped and not yielded as ProviderError.
3. Exercise the rig adapter with the equivalent controlled response; expect the current code to retain default EndTurn and produce FinalResponse despite the transport failure.
4. Repeat before any text, after usage/tool deltas, and after explicit stop. Compare with explicit DONE, valid EOF policy, Rhai runtime failure, and non-2xx controls.
5. Assert the corrected contract yields a terminal error and no successful FinalResponse after transport failure, while preserving already emitted partial text as partial evidence.

## Existing specifications/tests and missing regressions
- `spec/features/custom-provider-streaming-sse-bridge.feature:56` covers DONE termination; `:81` covers Rhai runtime errors, `:86` HTTP 401, and `:96` a successful mock SSE exchange.
- `rust/providers/tests/custom_streaming_sse_bridge_tests.rs:81`, `:302`, `:342`, and `:466` exercise those respective cases. They do not establish preservation of a byte-stream Err after HTTP success.
- `spec/features/custom-provider-streaming-sse-bridge.feature:23` describes a fallible stream; its existing examples do not define malformed-SSE versus fatal transport-error policy.
- Missing: injected underlying read errors before/after deltas, no success after fatal transport failure, no accidental tool execution from a failed flush, and distinguishable malformed-frame/EOF/DONE handling.
- Keep custom-provider tests distinct from built-in structured-rejection coverage in `spec/features/builtin-provider-structured-stream-rejection.feature:14` (OpenAI) and `:26` (Gemini).

## Recommended direction
- Classify SSE adapter errors rather than logging and skipping all of them; propagate underlying transport failures as typed ProviderError and terminate without success finalization.
- If malformed frames are intentionally recoverable, specify a narrow recovery policy and observable diagnostics separately. Do not use that policy to swallow underlying body errors.
- Track terminal evidence explicitly in the rig adapter. Decide ordinary EOF-without-stop policy through acceptance criteria instead of relying on unconditional EndTurn fallback.
- Add failing offline bridge and adapter regressions first; preserve non-2xx and Rhai-runtime error behavior.

## Uncertainties and related cards
- Actual transport failure frequency and library-specific event error shapes need fixture verification; none were executed. No assertion is made that the incident used a custom provider.
- PROV-064 owns the bridge; CMPCT-071 covers related but distinct built-in rejection evidence. BUG-224 is built-in Gemini terminal capture, not this custom-provider path.
- BUG-221, BUG-222, BUG-223, BUG-225, and CONT-010 cover separate exhaustion/retry/completion/cache/lifecycle questions. A causal missing-nudge claim requires independent incident evidence.

## Executed regression and fix — 2026-10-21

- Strict ACDD: captured rules/examples, wrote all twelve scenarios before tests,
  then observed behavioral red before changing production code. The unchanged
  scenarios were subsequently separated into bridge and rig features to meet
  the one-feature/one-test-module gate. Every scenario has exact `@step` comments.
- `/tmp/bug227-red.log`: **6 failed, 6 passed**. The real HTTP/rig regression
  observed `Final(RhaiCustomCompletion { stop_reason: "end_turn", ... })` after
  an interrupted HTTP body. Bridge regressions also demonstrated swallowed
  transport/UTF-8 errors and buffered-tool completion after failure.
- `stream_http.rs` now propagates every eventsource error as a terminal
  `ProviderError::Api`, preserving the custom provider name and underlying
  diagnostic. It returns without reading subsequent events or flushing tools.
- Ordinary EOF requires an emitted explicit stop. Missing stop/DONE, including
  empty, text-only, pending-tool and incomplete-frame responses, is an error.
  DONE still terminates immediately and flushes valid pending work. Explicit
  stop followed by clean EOF retains its reason; a later body error still fails.
- `stream.rs` only changes provider-name field visibility to `pub(super)`;
  no processor behavior or file length changes. `rig_streaming.rs` needs no
  change: its existing error branch now receives the previously swallowed error
  and returns before constructing `FinalResponse`.
- Already emitted deltas are retained as partial evidence. Work emitted before
  a later failure cannot be retracted; failure itself performs no tool flush.
- Tests use deterministic byte streams and real loopback HTTP responses with
  intentionally short Content-Length bodies, invalid UTF-8, and missing terminal
  evidence. Clean controls preserve text, reasoning, usage, tools, and all three
  stop reasons. Dropping pending bridge/rig streams releases the source/socket.

### Validation evidence

All Cargo commands use `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`
and `-j 2`; no workspace-wide build or test command was run.

- `cargo test -p codelet-providers --test bug227_custom_stream_failures -j 2`:
  **12 passed**, `/tmp/bug227-green.log` and `/tmp/bug227-final-green.log`.
- Scoped adjacent regression run adds `--test custom_streaming_sse_bridge_tests
  --test custom_streaming_reasoning_delta_tests --test rhai_rig_agent_keystone_tests`:
  **38 passed total**, `/tmp/bug227-regression.log`.
- `cargo check -p codelet-providers -j 2`: passed, `/tmp/bug227-check.log`.
- `cargo clippy -p codelet-providers --test bug227_custom_stream_failures -j 2
  -- -D warnings`: passed, `/tmp/bug227-clippy.log`.
- Scoped `rustfmt --edition 2021 --check` for both changed production files and
  the test entrypoint (recursively covering its modules): passed,
  `/tmp/bug227-fmt.log`. All new files are below 300 lines.
- Both features pass formatting, Gherkin validation, scoped `validate-tags`,
  and `audit-coverage`. Coverage is **8/8 bridge + 4/4 rig**, each scenario mapped
  to exactly one test and implementation. Logs: `/tmp/bug227-feature-validation.log`,
  `/tmp/bug227-tags-scoped.log`, `/tmp/bug227-coverage-audit.log`,
  `/tmp/bug227-coverage.log`.
- Repository-wide tag validation remains blocked by **530 unrelated feature
  violations**; both BUG-227 features pass independently. Summary:
  `/tmp/bug227-tags-global-final.log`. No unrelated tags were changed.
