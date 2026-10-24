# Selected-adapter and lifecycle audit — coordinator 264d7f66

Gate owner `5ce54e0f` owns only CMPCT-083 tests, gate specifications, sidecars
and evidence. No production, board, staging or commit changes.

## Confirmed gaps

- The inherited seven rows covered five built-ins (Anthropic, OpenAI, Gemini,
  Codex, Copilot), plus custom and stub. Z.AI is a sixth supported built-in:
  `ProviderType::ZAI`, registry `zai`/`z-ai`, `ProviderManager::get_zai`,
  `ZAIProvider`, and agent-loop's selected `zai` dispatch are real production
  paths. Sharing OpenAI Chat Completions does not establish Z.AI coverage.
- Z.AI chooses `ZAI_PLAN_API_KEY` before `ZAI_API_KEY`; the selected endpoint
  is `/api/coding/paas/v4` versus `/api/paas/v4`, model `glm-4.7`, with GLM
  temperature 1.0 and top_p 0.95. Both credential configurations need explicit
  selected-provider checks, not merely another generic OpenAI row.
- The immediate post-promotion child is a useful stronger F2 probe. However,
  it currently precedes the next live request and therefore does not satisfy
  the original live → commit → physical → fresh → physical acceptance sequence.
  Retain that probe and add a second child after the live request.

The feature is extended first with Z.AI normal/plan rows and explicit retention
of both lifecycle orders. New selected-path and real chronological assertions
are being executed before harness changes; logs and exact results follow.
The existing checkpoint records, tool suffixes, lineage/freshness checks and
negative matrix remain intact. Current disk headroom is 26 GiB; commands are
target-scoped `ci-test -j2` only.

## Resumed execution: actual RED and newly exposed blocker

The initial new chronology regression executed and failed at `required live
physical request must precede the fresh-process physical request` in
`/tmp/cmpct083-required-order-red.log` (parent 0 passed, 1 failed, Cargo101).
Both new Z.AI rows failed `unknown provider gate row` in
`/tmp/cmpct083-zai-selection-red.log` (0 passed, 2 failed, Cargo101).
These are behavioral RED before harness implementation, not compile errors.

The harness now retains the immediate post-commit cold probe and separately
spawns another child after the live physical response. It records actual
request/process timestamps and requires distinct parent/child PIDs, live
response completion before fresh process startup, and physical request ordering.

The first rerun `/tmp/cmpct083-required-order-current2.log` passed its first
child but failed the second child with a missing committed message reference.
The parent held a stale in-memory message index across the child's genuine
successful persisted response, then overwrote the index on its next write.
The harness releases/reloads both message and manifest stores across each
serial child handoff; no disk record or acceptance check is removed.

`/tmp/cmpct083-zai-selection-current2.log` now reaches the actual selected
ZAIProvider-created agent and physical Chat Completions capture. Both normal
and plan rows fail because `max_completion_tokens` is null instead of8192.
`zai.rs::create_rig_agent` sets `.max_tokens(8192)`, but Rig's OpenAI
`CompletionRequest::try_from(OpenAIRequestParams)` destructures the core request
without carrying `max_tokens` into the physical body. This is a production
serializer blocker owned by the Rig/provider owner, not an endpoint or fixture
failure. Preserve this output-limit assertion; do not substitute a generic
OpenAI row or force the missing option via test-only additional_params.

Current headroom25GiB. The new coverage mappings and full matrix/quality reruns
remain pending; log names containing green are not successful evidence.

## Current production blockers — durable owner mailbox

The executed `/tmp/cmpct083-gate-resumed-matrix.log` reports cold 2 passed,
9 failed (11 total), negative 6 passed, 2 failed (8 total). Do not infer current
completion from older passing subsets. A rerun after harness schema/usage and
cross-process cache fixes is `/tmp/cmpct083-gate-latest-matrix.log` (pending).

- Rig owner: both actual selected Z.AI normal/plan requests omit the configured
  `max_completion_tokens: 8192`. The gate keeps that physical output-limit check.
- Rig/CLI owner: the Gemini physical suffix is one native `functionCall` named
  `lookup` with `args.path=src/deep.rs`, followed by `functionResponse` named
  `call_after_snapshot` with the complete result. Native Gemini correlates by
  function name, so that response must be `lookup`, not the unrelated source
  call ID. `/tmp/cmpct083-suffix-diagnostic-gemini.log` contains actual capture.
  The gate must assert native semantic pairing, not merely count source IDs.
- Sessions owner: the real immediate Codex child fails `replay revision conflict:
  committed context requires finite positive trigger and capacity limits`.
  Lost-notification child physically sends after exact reconciliation, but its
  returned `contextSnapshot.generation` remains null rather than committed 1.
  The gate now uses `BackgroundSession::reconcile_durable_compaction` directly,
  retaining wrong-identity rejection and the no-false-completion assertion.
- CLI/provider owner: both Codex negative controls reach the real runtime but
  lose their typed `WebSocketUpgradeRejected` / `CheckpointError::Lineage` source
  in the error chain. They remain RED despite no fallback transmission; typed
  failures, blocked inference and unchanged checkpoints remain required together.

Harness-only corrections preserve all behavior: stub checks actual separate
`id` and `call_id` fields rather than a raw string count; admitted positive
occupancy is distinct from measured occupancy after a successful response;
transport fixtures now provide real usage records. Parent manifest/message
cache ownership is released and reloaded around serial child writes without
deleting any record, lease or guard.

## Latest completed rerun — semantic native pairing

`/tmp/cmpct083-gate-semantic-matrix.log` is complete, Cargo exit101:
cold **7 passed / 4 failed / 0 ignored** (11 parents), negative **6 passed /
2 failed / 0 ignored** (8 parents). Passing cold parents are Anthropic, OpenAI,
Copilot, custom, stub, cancellation and notification-loss reconciliation.
The notification case now uses the repaired background publication API and passes
its generation/header/no-false-completion checks; its old blocker is superseded.

Current failures, preserved rather than bypassed:

1. Codex immediate child exits101 with the finite-positive trigger/capacity
   restore error. Fresh child authority checks are still required, not skipped.
2. Gemini native semantic assertion is behavioral RED: actual result name is
   `call_after_snapshot`, expected `lookup`; one complete ordered call/result is
   required, with native function-name correlation. This is not string-count RED.
3. Both Z.AI paths now pass their physical output-limit/config checks after the
   concurrent Rig fix, then fail actual candidate admission: `request admission
   requires known media occupancy`. The real ZAIProvider agent carries its native
   complete tool schemas. Provider `selected_request::media_occupancy` recursively
   scans the entire physical body including schemas; investigate schema keys such
   as `image`/`source` being misclassified as actual opaque media. Do not remove
   selected native tools, hardcode media zero, or weaken admission to pass.
4. Both Codex transport/lineage negatives still fail typed source-chain identity.

The stub assertion keeps exactly one source `id` per call/result plus role,
arguments, full result content and order. Its optional serializer-added `call_id`
must match whenever present; the authentic source has only `id`/`tool_use_id`,
so absent optional metadata is not invented. This replaces an invalid raw count
of a value that appears in both fields, not a source-identity requirement.
