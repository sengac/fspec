# BUG-284 fixture and historical recovery review

## Concrete safe evidence extraction

Four prompt-free projected fixtures have been extracted under this directory:

| Fixture | Original first divergence | Canonical output discrepancies | Extra canceled-era user records |
| --- | --- | --- | --- |
| `4d8eedb6-projected-fixture.json` | index 490 | 0 | 6 |
| `7b41d57c-projected-fixture.json` | index 425 | 0 | 2 |
| `7c55a508-projected-fixture.json` | index 3 | 233 | 9 |
| `c09c6649-projected-fixture.json` | index 3 | 165 | 0 |

`extract_projected_fixtures.py` reads only preserved private evidence and outputs
stable opaque item/call tokens. It retains item order/type/role, call-identity
relationships, checkpoint generation/model/protocol and the exact equality graph
between checkpoint and reconstructed input. It substitutes every message text,
outcome, argument, name and encrypted reasoning body, so no private prompt or
result text is exported. The script verifies the substitution is injective over
all projected items and that each unmodified sanitized fixture fails at the same
index and same expected/actual item type/role as its raw source.

Each fixture records the SHA256 of its raw preserved checkpoint and reconstructed
input. The corresponding `*-token-proof.json` maps source projected-item hashes
to opaque labels. `extraction-verification.json` records the assertions and
counterfactual counts. These are **evidence derivatives**, not independently
implemented green examples.

Only cloned sanitized inputs were altered for the counterfactual checks:
latest fixtures require 6/2 omitted unmatched user tokens; the earlier fixtures
require 233/165 canonical-outcome substitutions, plus 9 user omissions for the
first. Raw and live data remain untouched. **Counterfactual omission is a causal
probe, not the proposed production fix or recovery API.**

These fixtures are suitable for strict provider-lineage replay controls. They
are not sufficient as regression coverage for the repair: projection removes
stream partitioning and replaces byte payloads, so they cannot prove prompt
admission behavior or the encoding transform. Keep the separate private real
Rust API replay as evidence, and use the runtime harness below for the behavioral
RED and GREEN.

## What the old tests missed

`rust/agent-loop/tests/bug276_prompt_admission_cancellation.rs` runs the actual
queue/agent loop and deliberately verifies canceled input is recoverably archived
but absent from runtime/provider dispatch. Its
`support/prompt_admission.rs::new_session` uses a stub provider. The tests finish
before Codex checkpoint preparation, selected compaction and restoration.

`rust/agent-loop/tests/support/bug282.rs::Fixture::new` creates a real Codex
runtime and native BackgroundOutput, but discards the input receiver. `run_output`
persists text and calls `run_agent_stream` directly. That exercises canonical
outcome persistence, not `agent_loop::prompt::admit_prompt`, its cancellation
boundary, or draining queued supervisor inputs.

BUG-278's fixture tests provider/durable generation separation through two
compactions, cold runtime reconstruction and loopback transport. Reuse its
binding assertions; do not substitute manually appended successful checkpoint
fixtures for the causal prompt-admission regression.

The missing composition is therefore **real queue cancellation + recoverable
archive + real Codex checkpoint + selected compaction/restore**.

## Behavioral acceptance harness to implement

Use the shared temporary-directory/store helpers and serial store guards. Retain
an actual `mpsc::Receiver<PromptInput>`, launch the production `agent_loop`, and
select the real Codex provider against a controlled loopback HTTP server. Use
barriers/events, not timing sleeps, to control cancellation. Existing BUG-276 hook
and durable-journal barriers provide suitable patterns; BUG-282's EffectTool and
captured-request transport provide exactly-once and wire evidence.

1. **Seed real accepted history.** Obtain a genuine checkpoint through streamed
   assistant output, at least one function call/result and a once-only temporary
   effect. Capture the admitted input identity and actual request item order.
2. **Reproduce both evidence shapes.** Queue 6 supervisor inputs and cancel before
   admission; separately queue 2 with cancellation around the tool-outcome-unknown
   boundary. Use distinct opaque fixture texts linked to the extracted token
   proof, not copied private prompts. Verify the cancellation barrier was reached,
   the queue actually held the expected count, and each original is archived
   exactly once. Preserve the 6/2 distinctions as parameterized cases.
3. **Admit a genuine new input.** Drive the same normal queue path. Verify canceled
   inputs never reached provider dispatch or tool effects and are absent from the
   authoritative inference projection while remaining available to audit/recovery.
   A user input that was already admitted before cancellation must remain in the
   provider history; cancellation is not blanket permission to erase a turn.
4. **Exercise actual trigger paths.** First trigger automatic threshold compaction
   with the controlled selected request, and separately terminal-overflow/manual
   compaction where relevant. Do not merely call a preparation helper or return a
   prebuilt success result. Capture actual summary/DAG validation, durable commit,
   exact publication identity and post-publication checkpoint binding.
5. **Cold continuation and repetition.** Tear down runtime and reopen the isolated
   persisted state, preferably through the child-process machinery already used
   by `cmpct083_cold_process`. Continue once, cause another cancellation/admission
   cycle, compact a second time, then cold restore and continue again. Assert
   provider/durable generations, original byte/hash preservation, genuine output
   equality and exactly-once effect counts on every boundary.
6. **Streaming/parallel composition.** In at least one cycle return multiple tool
   calls, complete outcomes in a different order, split assistant text across
   delta/completed events, and cancel while one outcome is unknown. Preserve the
   existing strict policy for unknown outcomes; the test must not fabricate a
   successful result or silently rerun a side effect.

Expected pre-fix RED must be the real selected-lineage failure after canceled
records enter durable replay, not a constructor/serialization failure. On GREEN,
prove the admitted projection is consistent before and after cold restore, rather
than suppressing the provider's check. Keep every existing BUG-276 cancellation
and recoverability assertion: the repair must reconcile the two histories.

## Historical corruption and recovery semantics

A prevention fix does not automatically repair the four preserved failed
sessions. Make that boundary explicit in code/API behavior and documentation.

For newly recorded inputs, persist durable admission disposition and its ownership
(identity/epoch or equivalent) at the same authority boundary used for inference.
An archival canceled input must stay recoverable without being reintroduced as an
admitted conversation ancestor by cold restore or compaction. Keep archival
original retrieval separate from the inference projection; do not remove original
message references simply to satisfy the provider. Recovered/resubmitted input
requires an explicit new admission decision, not retroactive promotion of a
canceled record.

For legacy records, the observed extra originals have `requestId: null`; absence
from a checkpoint, a `[SUPERVISOR:]` text prefix, or proximity to a canceled event
alone is not durable authorization to omit arbitrary history. In particular,
7c55a508's nine extra records do not use the newer supervisor prefix. A general
"ignore unmatched user messages" change would turn this diagnosis into a lineage
bypass and must be rejected.

Until an explicit recovery operation can prove exact message ownership and
source/checkpoint binding, fail closed with a precise diagnostic identifying the
ambiguous archived input IDs/hashes. Preserve originals, manifest, checkpoint and
pending evidence. An authorized recovery feature should record a separate,
versioned, identity-bound reconciliation ledger; require exact source hashes,
reject changed/stale source, write atomically, survive lost acknowledgement and
be idempotent. It should not overwrite the original archived messages or mutate a
checkpoint into agreement. Any such migration needs its own executable criteria;
do not silently fold it into ordinary request preparation.

The old encoding corruption is additionally lossy: stripping/unescaping has no
unique inverse. If a legacy recovery operation is later authorized, it can use
an existing authoritative checkpoint's canonical outcome only with complete
call identity/invocation/source binding and exact proof that the archived text
matches the recognized old transform. It must never guess by re-escaping a
persisted string. The overlapping 233+9 case must remain blocked if either source
of divergence is unresolved; fixing one does not justify ignoring the other.

## Mandatory negative controls

- Alter or remove a genuinely admitted user message: rejection remains strict.
- Forged cancellation metadata, wrong session/epoch/request ID, duplicated
  admission transition, or changed source hashes cannot exclude an ancestor.
- A canceled input identical in text to an admitted input is distinguished by
  identity, not text equality or prefix matching.
- Cancellation after dispatch retains the actually submitted prompt; it must not
  retroactively become archive-only.
- Persistence failure/retry and crash between archive, admission and dispatch do
  not duplicate, lose or falsely admit input; unresolved state blocks inference.
- Changed assistant, invocation name/arguments, outcome, call identity and
  established DAG ancestor remain rejected without promotion/publication.
- Both canonical JSON-string outer quotes and object escapes remain byte-exact
  under the fixed stream handler; historical lossy data is not auto-decoded again.
- A source containing both historical encoding damage and extra canceled records
  cannot pass by resolving only one mismatch; use the 233+9 trace as provenance.
- Publication failure and lost acknowledgement preserve exact promoted identity,
  inference gating and idempotent reconciliation, without another generation.

## What can honestly be claimed

Passing the extracted replay controls and full isolated runtime tests supports a
source repair with demonstrated causal coverage. A live-fixed claim additionally
requires authorized installation/restart, the actual loaded executable hash/build
verification, and repeated live native compaction/restore/continuation receipts.
Those deployment actions and historical-session mutations remain outside this
read-only review. No source changes, builds, native calls or original writes were
performed here.
