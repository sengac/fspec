# Auto-continue investigation

Audited HEAD: `f0eb0759`, October 25, 2026. History window: September 25–October 25.
No repository files changed. This is an investigation, not an implemented fix.

## Conclusion

The completion-contract engine and conditional Done registration are still present.
There is no evidence that auto-continue was globally removed or disabled.
However, the provider/engine boundary has concrete defects and coverage gaps.
One new Codex follow-up failure was reproduced against the compiled production library.
An actual failing user session or provider trace is still needed to identify which
path explains the user's observed symptom.

## Intended contract and evolution

- CONT-001: a model segment that calls tools must receive their results in another
  model segment, even if text follows the tool call. The patched rig loop retains
  this behavior in `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:930`.
- CONT-002: opt-in `/continue`; default budget 10; only zero-progress nudges count;
  a subsequent segment with a tool call refunds its nudge. A clean stop without
  accepted Done triggers another prompt. Exhaustion warns and finishes without
  disabling the toggle. See `spec/attachments/CONT-002/design-auto-continue.md:47`
  and `spec/features/auto-continue-engine.feature:21`.
- CONT-003: an active goal implies continuation, with budget `max(explicit, 15)`.
  Done additionally requires evidence, a substantive assessment, and successful
  verification if configured. Four rejected Done calls, budget exhaustion, or two
  zero-activity nudges escalate. See `spec/features/goal-enforcement.feature:22`.
- CONT-005/006: accepted Done ends the stream immediately after its tool result,
  rather than waiting for natural provider completion. Goal acceptance atomically
  clears the goal and reminder. See `spec/features/goal-immediate-termination.feature:19`.
- CONT-007/008: counter snapshots update the footer; accepted goals write back to
  background state without resurrecting an already satisfied goal.
- CONT-009: production frontend dispatch must arm the registry before constructing
  provider agents. The formerly separate NAPI loop now delegates to the shared
  implementation. See `spec/features/completion-contract-dispatch-arming.feature:10`.
- PROV-142 originally seeded continuation only when creating a profile session.
  PROV-161 now resolves inherited settings on creation, resume, switching and the
  next dispatched turn; explicit session choices override inherited continuation.
  See `spec/features/provider-runtime-settings-lifecycle.feature:8` and
  `rust/sessions/src/runtime_settings.rs:7`.
- CONT-010 remains backlog, and older creation-only claims remain in
  `spec/features/continue-cache-reconciliation.feature:11`; they conflict with the
  newer PROV-161 lifecycle contract and need reconciliation.

Historical anchors:

- `7c95aa84` (July 11): completion-contract engine, goal mode and immediate Done.
- `e66f2cb1` (August 20): profile continuation defaults.
- `38a95522` (October 15): BUG-208, explicit Responses backend SSE errors and schema fixes.
- `2699693b` (October 25): dedicated Codex model, transport/history/recovery changes,
  provider runtime settings, mandatory usage evidence and shared-loop hardening.
- `f0eb0759` (October 25): transcript/recovery hardening and Gemini invocation identity.

## How it currently works

1. Shared background dispatch reloads effective provider settings and synchronizes
   continuation/goal state before building the agent:
   `rust/agent-loop/src/agent_loop.rs:118`.
2. The registry is armed when continuation is enabled OR a goal exists:
   `rust/sessions/src/background_session.rs:1737`.
3. All seven production agent builder families conditionally register Done.
   The exposed tool name is `Done`; documentation and nudge prose use `done()`.
4. Normal tool activity is followed by another model segment inside rig.
5. Accepted Done is consumed at the tool-result branch, which drops the stream,
   performs shared teardown and emits stop reason `done`:
   `rust/cli/src/interactive/stream_loop.rs:1114`.
6. Otherwise, natural FinalResponse consults continuation policy. Only absent,
   `stop`, and `end_turn` stop reasons are eligible. The nudge is another user
   message, not another real user turn:
   `rust/cli/src/interactive/auto_continue.rs:52` and
   `rust/cli/src/interactive/stream_loop.rs:1614`.
7. Errors, interrupts and stall exits do not run that ordinary nudge decision.
   Compaction recovery and repetitive-output corrective retries are separate
   mechanisms with separate budgets.

## Findings

### 1. New Codex refusal-history follow-up failure — reproduced

`rust/providers/src/codex/answers.rs:44` exposes refusal sections as ordinary
visible assistant text. The Codex checkpoint retains the original refusal item,
but `rust/providers/src/codex/history_projection.rs:11` normalizes only output_text,
not refusal. A subsequent request reconstructed from visible text therefore fails
the lineage comparison in `rust/providers/src/codex/history_lineage.rs:83`.

A standalone probe linked to the actual compiled provider confirms:

- output_text answer followed by a new request: accepted;
- refusal with identical visible text followed by the same request: ancestry error.

This can break an auto-continue nudge, tool-result follow-up, or subsequent user
request after refusal-bearing output. Introduced by `2699693b`.
Existing refusal tests check display, not a subsequent request.
Probe: `/tmp/fspec-auto-continue-history-audit.rs`.

### 2. Codex has a separate, restrictive protocol-continuation loop

`rust/providers/src/codex/lifecycle.rs:82` follows completed `end_turn:false`
responses and certain interrupted incomplete responses when no tool was seen.
But replay at `rust/providers/src/codex/lifecycle.rs:123` permits only nonempty
text-only assistant output. Reasoning, hosted-search items, refusal sections and
empty output cause an explicit error before the outer engine receives FinalResponse.

The provider also caps protocol follow-ups at three:
`rust/providers/src/codex/streaming.rs:57`. `/continue 300` does not change this cap.
Budget-limited and content-filtered incomplete responses are errors, not normal
outer nudges. This is specified policy in the new implementation, not proof of an
accidental regression, but it is a significant compatibility restriction.

### 3. Codex forced refresh loses retryable network error identity

After HTTP 401, `rust/providers/src/codex/recovery.rs:150` converts refresh timeout
and all refresh failures into generic authentication strings. The underlying
refresh path preserves typed network failures, but
`rust/providers/src/codex/reconnect.rs:177` retries only typed NetworkFailure.
An offline issuer during forced refresh can therefore terminate with “sign in
again” rather than enter network recovery. New in `2699693b`; source-confirmed,
not independently execution-reproduced in this audit.

### 4. OpenAI Chat tool order can cause premature accepted Done

Streaming indexed calls are stored in a HashMap at
`rust/patches/rig-core/src/providers/openai/completion/streaming.rs:167` and emitted
unsorted at lines 317 and 355. Done can execute before earlier-indexed ordinary
tools. Immediate stream dropping then prevents the earlier work from executing.
This affects native OpenAI, ZAI and native Copilot through their shared decoder.
Nonstreaming Chat preserves vector order.

This is longstanding, not newly introduced in the requested month. No mixed-batch
ordinary-tool/Done execution-order regression test was found.

### 5. OpenAI terminal evidence and accepted Done do not compose safely

- Chat ignores `[DONE]` and skips parse/schema failures at
  `rust/patches/rig-core/src/providers/openai/completion/streaming.rs:182`.
  Valid usage plus EOF can still yield success without any finish_reason.
- Ordinary Responses recognizes failed/incomplete event variants but handles only
  ResponseCompleted at
  `rust/patches/rig-core/src/providers/openai/responses_api/streaming.rs:415`.
  Failed details and truncation semantics generally become “missing Responses usage”.
- Both native Chat and ordinary Responses expose tools before later terminal
  usage validation. Ordinary Responses does so at
  `rust/patches/rig-core/src/providers/openai/responses_api/streaming.rs:454`.
- Accepted Done drops the stream before consuming later errors/Final; the finish
  hook runs only on Final at
  `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:837`.

Thus terminal validation added in `2699693b` is not necessarily reached on accepted
Done. Conversely, missing usage without accepted Done is now a hard error, and an
error does not trigger ordinary auto-continue. Preserve fail-closed validation;
do not fix this by treating provider failures as successful completion.

Ordinary patched-rig Responses is NOT the new dedicated Codex implementation.
Native fspec OpenAIProvider itself constructs Chat Completions, not Responses.

### 6. Shared recovery dispatch resets completion-contract accounting

Every dispatch calls the synchronization helper, including synthetic outer-loop
recovery inputs. The helper unconditionally resets nudges at
`rust/sessions/src/background_session.rs:1711` and calls set_session_goal at line
1738, which resets registry rejections even when the goal is unchanged:
`rust/tools/src/done.rs:115`.

This conflicts with real-user-turn budget boundaries and goal-lifetime rejection
counting. It predates the latest changes, but automatic compaction/recovery makes
the boundary more important. Ordinary in-stream nudge retries do not call this
helper, so they do not suffer this particular reset.

### 7. Custom provider and routing defects

- Built-in-name script shadowing resolves to Custom but shared dispatch matches
  only a provider-name string and selects the native branch. Native getters then
  reject the Custom enum. See `rust/providers/src/manager.rs:55`,
  `rust/agent-loop/src/agent_loop/routing.rs:7`,
  `rust/agent-loop/src/agent_loop/providers.rs:18`, and
  `rust/providers/src/manager.rs:817`. This fails before Done registration.
- A Rhai stop terminates the parser at
  `rust/providers/src/custom/stream_convert.rs:131`, suppressing later usage-only
  frames, while `rust/providers/src/custom/stream_http.rs:99` continues awaiting
  HTTP events until `[DONE]` or EOF. Settlement/nudging can be delayed into stall
  handling. Existing tests place usage before stop.
- Scripted OAuth's automatic refresh helper has no production caller in the
  custom HTTP completion path; scripts that refresh themselves are unaffected.
  See `rust/providers/src/oauth/scripted_refreshing_client.rs:58` and
  `rust/providers/src/custom/rig_streaming.rs:36`.

### 8. Copilot endpoint selection does not drive the native agent

The helper selects Responses for certain models at
`rust/providers/src/copilot/endpoint.rs:75`, but actual construction uses a
CompletionsClient at `rust/providers/src/copilot/provider.rs:127` and Chat agent
at `rust/providers/src/copilot/rig_agent.rs:61`. Existing routing tests test the
helper, not an observed provider request. This is longstanding.

## Provider coverage

| Provider/path | Done and continuation assessment |
| --- | --- |
| Claude API and OAuth | Same armed-only builder and shared engine; no missing registration found. OAuth changes authorization, not completion semantics. |
| OpenAI Chat, cloud/local profiles | Armed-only Done; SSE and JSON-to-stream paths reviewed; tool-order/terminal issues above. |
| Generic OpenAI Responses | Request/history conversion, streamed tools and terminal parser reviewed; distinct from dedicated Codex; no separate native non-Codex fspec builder found. |
| Codex API-key/OAuth | Armed-only Done intact at `rust/providers/src/codex/agent.rs:99`; dedicated history, native/custom tools, SSE/WebSocket and recovery reviewed. |
| Gemini | Armed-only Done intact; continuation sub-loop includes immediate Done exit. Latest invocation-ID correction is present. |
| ZAI | Armed-only Done; shares native Chat decoder and its ordering issues. |
| Native Copilot | Armed-only Done; actual Chat path despite Responses routing helper. |
| Custom/Rhai | Both parent-agent builder branches register native Done; compactor sub-agent branch excludes it. Routing/stream/OAuth issues above. |
| Scripted OAuth | Authentication plumbing, not a separate completion-contract engine. |
| Stub | Normal test stub has no tools; cannot validate native Done acceptance without attaching tools. Test seam limitation, not production regression. |

No tracked production `.rhai` bundles were found. User-installed provider scripts
outside this checkout were not audited; shared plumbing and embedded script
fixtures were reviewed.

## Validation

Executed scoped Cargo tests under release-slim, bounded to four jobs:

- codelet-cli / auto_continue_engine_test: 10 passed, 1 failed.
  Failure: obsolete Codex mod.rs source scan; registration moved to agent.rs.
- codelet-cli / goal_enforcement_test: 18 passed.
- codelet-cli / cont006_goal_immediate_termination: 6 passed, 1 failed.
  Failure: obsolete search for clear_goal in session/mod.rs; it moved to reminders.rs.
- codelet-providers / codex_terminal_regressions: 7 passed.
- codelet-providers / prov148_codex_completed_answers: 1 passed.
- codelet-providers / prov149_codex_history: 2 passed.
- Standalone compiled-provider history probe: 2 passed, establishing the normal
  text control and reproducing refusal-follow-up rejection.

Cargo output: `/tmp/fspec-auto-continue-audit-tests.log` and
`/tmp/fspec-auto-continue-goal-tests.log`.
Additional Codex targeted test output: `/tmp/fspec-auto-continue-codex-tests.log`.

## Recommended next implementation work

1. Add real provider-stream tests for armed stop -> nudge -> Done, rejected goal
   completion, mixed ordinary-tool/Done ordering and malformed terminal delivery.
2. Add a refusal-history follow-up scenario/test and fix its projection without
   weakening native lineage validation or changing refusal semantics.
3. Preserve typed network failures through forced Codex token refresh.
4. Separate user-turn reset from registry synchronization/recovery dispatch.
5. Resolve Codex protocol replay policy explicitly against typed-history support.
6. Fix ordering/terminal evidence at provider boundaries and reconcile stale tests
   and lifecycle specifications. Keep accepted Done immediate and do not execute
   arbitrary trailing tools merely to drain a stream.
