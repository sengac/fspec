# BUG-268 corrected live-session lineage design for review

## Acceptance and chronology

Controller source-grounded correction supersedes only the earlier model-owned
clearing-at-genuine-turn approval. Immutable b2475dcd PROV-055 and PROV-057
scenarios explicitly require the previous reasoning_opaque unchanged on the
NEXT distinct genuine TUI turn. Restart/durable replay is not required. Proof:
`/Users/rquast/Documents/Codex/2026-10-23/task-3/validation/bug268-original-scope-review.json`.
The earlier approvals and draft remain preserved as chronology. No model-only
opaque owner was implemented, no opaque acceptance was claimed, and no Done
transition was attempted. Native Example Mapping was corrected by soft-removing
the two incompatible rules/example and adding the original next-turn requirement.

The last build completed with exit 101, not uncertain execution: unnecessary
Clone derive on a nonstreaming Chat raw response fails compilation. No active
Cargo/rustc process was found. That is introduced unfinished type composition,
not a baseline failure or valid runtime result. The earlier three actual URI
causal reds and six old type controls remain valid; invalid fixture attempts are
retained separately. Existing typed dialect edits are preserved, not discarded.

## Bounded candidate — no new registry or durable protocol

Register a narrowly scoped CopilotSession extension in the EXISTING
ProviderSession only. Its Arc ownership lives with one live CLI/background
session; ProviderSession clones for that same session share it. There is no
global/session-ID map, independent service or durable continuation store.
ProviderAdapter::with_session already reaches the shared dispatcher and NAPI
prepare_provider; Copilot binds that same extension into its typed client/model
and native agent, so reconstruction retains the owner rather than replaces it.
Standalone front doors without an explicit live session get an isolated owner,
never an implicit lookup of another session's state.

One latest committed opaque JSON value is kept uninterpreted, together with
provenance: canonical Copilot provider, logical GitHub account fingerprint (not
the renewing short-lived Copilot token), deployment/effective API origin, exact
model, and owner generation. It is not ordinary assistant/prompt content and is
never adopted from caller parameters, transcripts, serialized raw responses,
checkpoint envelopes or restored state. No arbitrary native fields are copied.
Only the known response reasoning_opaque is captured at validated successful
settlement through the Copilot-scoped adapter; shared Chat/Responses policies and
all ordinary history/reasoning/tools remain intact. No Codex policy changes.

## Exact history and stale/cancelled settlement guards

Replay requires the request preamble and typed-history prefix to match the last
successfully committed request history plus its actual settled assistant output
(including tool/call IDs and existing reasoning). A genuine next user message,
normal paired tool results or a nudge may append to this prefix. Rewritten,
cleared, incompatible or compacted ancestry fails closed and clears lineage.
Comparisons may use canonical typed serialization internally but do not log or
persist the history or encrypted value. The prior committed opaque value cannot
be replayed merely because a slug or request text happens to match.

begin_turn advances an attempt epoch to invalidate late previous-turn writes,
but DOES NOT discard compatible committed live lineage. Every request captures
the active epoch/provenance; only the current successful attempt can commit.
Failure, cancellation or supersession cannot resurrect older metadata. Reset,
compaction generation changes, model/account/deployment change and provider
switch invalidate it. Ordinary same-account short-token refresh preserves
ownership. Registering/ensuring a different provider must invalidate Copilot's
existing extension through the existing lifecycle, not leave a dormant reusable
cross-provider value. Poison/error handling fails closed rather than resetting
generation checks or replaying an uncertain blob.

## Nonpersistence and propagation

set_writer accepts the existing lifecycle hook without calling/storing it for
opaque data. Checkpoint preparation contributes no Copilot opaque metadata;
generation/restore paths reset rather than adopt any opaque envelopes. status,
Debug/Serialize output and ordinary transcript persistence exclude the value.
Restored/new sessions get fresh empty owners. Model/client clones share only
their bound live session owner and cannot migrate it to another session.

Retain the earlier approved actual request-time refresh coordinator and explicit
nondefault numeric-loopback issuer seam constraints. Add no refresh/inference
retry policy or shipping endpoint override. Reuse existing generic adapters,
keeping physical usage, tool-result identity, normal admission and immediate
Done behavior. The two necessary old Chat-only static assertions may change
only with real typed/middleware compatibility tests.

## Required runtime proof before new production integration

First add a two-distinct-send_input shared-background-loop test through actual
manager/provider reconstruction, not a manually retained model. Assert the exact
owned opaque value on request two, plus preserved ordinary history/tools and
normal armed Done. Capture genuine causal red before implementing the session
owner. Include same-account actual short-token refresh, model/account/deployment
switch, incompatible/cleared/compacted history and stale/cancelled settlement.
Test a separate/new/restored session and serialization/checkpoint exclusion.
Keep endpoint table, exact mini exception/fallback, client/nonstreaming/native
tool-stop-nudge-Done tests. Prove local issuer seam isolation BEFORE expired-token
fixtures. All data/keyring/config is synthetic and temporary; no live support or
restart survival claim is permitted. If any requirement cannot be met safely,
record the precise unmet behavior instead of marking BUG-268 complete.
